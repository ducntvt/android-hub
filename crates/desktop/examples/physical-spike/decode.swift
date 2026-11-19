// US-056 experiment. stdin: BE u32 length + Annex-B H.264 access unit.
// stdout: BE u32 width, height, RGBA length, then tightly packed RGBA.
// No device access. Requires hardware decode; CPU RGBA copy is intentional/measured.
import Foundation
import CoreMedia
import CoreVideo
import VideoToolbox
import Darwin

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}
func readExact(_ count: Int, allowEOF: Bool = false) -> Data? {
    var data = Data()
    while data.count < count {
        guard let part = try? FileHandle.standardInput.read(upToCount: count - data.count), !part.isEmpty else {
            if allowEOF && data.isEmpty { return nil }
            fail("Truncated decoder input")
        }
        data.append(part)
    }
    return data
}
func be32(_ data: Data, _ offset: Int = 0) -> UInt32 {
    data[offset..<offset+4].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
}
func append32(_ value: UInt32, to data: inout Data) {
    var value = value.bigEndian
    withUnsafeBytes(of: &value) { data.append(contentsOf: $0) }
}
func nals(_ data: Data) -> [Data] {
    let bytes = [UInt8](data)
    var markers: [(Int, Int)] = []
    var i = 0
    while i + 2 < bytes.count {
        if bytes[i] == 0 && bytes[i+1] == 0 {
            if bytes[i+2] == 1 { markers.append((i, 3)); i += 3; continue }
            if i+3 < bytes.count && bytes[i+2] == 0 && bytes[i+3] == 1 {
                markers.append((i, 4)); i += 4; continue
            }
        }
        i += 1
    }
    if markers.first?.0 != 0 { fail("Expected Annex-B access unit") }
    return markers.enumerated().compactMap { index, marker in
        let start = marker.0 + marker.1
        let end = index+1 < markers.count ? markers[index+1].0 : bytes.count
        return start < end ? Data(bytes[start..<end]) : nil
    }
}
let outputLock = NSLock()
var outputFrames = 0
var copiedBytes: UInt64 = 0
let callback: VTDecompressionOutputCallback = { _, _, status, _, image, _, _ in
    guard status == noErr, let image = image else { fail("Decode callback error \(status)") }
    let width = CVPixelBufferGetWidth(image), height = CVPixelBufferGetHeight(image)
    guard width > 0 && height > 0 && width <= 8192 && height <= 8192 else { fail("Invalid decoded size") }
    guard CVPixelBufferGetPixelFormatType(image) == kCVPixelFormatType_32BGRA else { fail("Expected BGRA output") }
    guard CVPixelBufferLockBaseAddress(image, .readOnly) == kCVReturnSuccess else { fail("Pixel lock failed") }
    defer { CVPixelBufferUnlockBaseAddress(image, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(image) else { fail("Missing pixel data") }
    let stride = CVPixelBufferGetBytesPerRow(image)
    var result = Data()
    append32(UInt32(width), to: &result); append32(UInt32(height), to: &result)
    append32(UInt32(width * height * 4), to: &result)
    result.count = 12 + width * height * 4
    // One explicit CPU row copy/swizzle; pipe and Iced upload are additional copies.
    result.withUnsafeMutableBytes { storage in
        let target = storage.bindMemory(to: UInt8.self)
        for y in 0..<height {
            let row = base.advanced(by: y * stride).assumingMemoryBound(to: UInt8.self)
            for x in 0..<width {
                let offset = 12 + (y * width + x) * 4
                target[offset] = row[x*4+2]; target[offset+1] = row[x*4+1]
                target[offset+2] = row[x*4]; target[offset+3] = row[x*4+3]
            }
        }
    }
    outputLock.lock()
    defer { outputLock.unlock() }
    do { try FileHandle.standardOutput.write(contentsOf: result) }
    catch { fail("Frame consumer disconnected") }
    outputFrames += 1; copiedBytes += UInt64(width * height * 4)
}
var sps: Data?, pps: Data?
var format: CMVideoFormatDescription?
var session: VTDecompressionSession?
var counter: Int64 = 0
func resetSession() {
    if let current = session {
        VTDecompressionSessionWaitForAsynchronousFrames(current)
        VTDecompressionSessionInvalidate(current)
    }
    session = nil; format = nil
}
func createSession() {
    guard let s = sps, let p = pps else { return }
    let result = s.withUnsafeBytes { sb in p.withUnsafeBytes { pb -> OSStatus in
        let pointers = [sb.bindMemory(to: UInt8.self).baseAddress!, pb.bindMemory(to: UInt8.self).baseAddress!]
        let sizes = [s.count, p.count]
        return CMVideoFormatDescriptionCreateFromH264ParameterSets(allocator: kCFAllocatorDefault,
            parameterSetCount: 2, parameterSetPointers: pointers, parameterSetSizes: sizes,
            nalUnitHeaderLength: 4, formatDescriptionOut: &format)
    }}
    guard result == noErr, let description = format else { fail("H264 format failed \(result)") }
    var record = VTDecompressionOutputCallbackRecord(decompressionOutputCallback: callback, decompressionOutputRefCon: nil)
    let decoder = [kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder: true] as CFDictionary
    let pixels = [kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_32BGRA] as CFDictionary
    let status = VTDecompressionSessionCreate(allocator: kCFAllocatorDefault,
        formatDescription: description, decoderSpecification: decoder,
        imageBufferAttributes: pixels, outputCallback: &record, decompressionSessionOut: &session)
    guard status == noErr else { fail("Hardware decoder unavailable \(status)") }
    FileHandle.standardError.write(Data("VideoToolbox hardware-required session created\n".utf8))
}
while let header = readExact(4, allowEOF: true) {
    let size = Int(be32(header))
    guard size > 0 && size <= 8 * 1024 * 1024 else { fail("Invalid compressed packet size") }
    let units = nals(readExact(size)!)
    var changed = false
    for unit in units {
        let kind = unit[0] & 31
        if kind == 7 && unit != sps { sps = unit; changed = true }
        if kind == 8 && unit != pps { pps = unit; changed = true }
    }
    if changed { resetSession(); createSession() }
    let slices = units.filter { ($0[0] & 31) != 7 && ($0[0] & 31) != 8 }
    if !slices.contains(where: { [1, 5].contains($0[0] & 31) }) { continue }
    guard let current = session, let description = format else { fail("Frame before SPS/PPS") }
    var avcc = Data()
    for unit in slices { append32(UInt32(unit.count), to: &avcc); avcc.append(unit) }
    var block: CMBlockBuffer?
    var status = CMBlockBufferCreateWithMemoryBlock(allocator: kCFAllocatorDefault, memoryBlock: nil,
        blockLength: avcc.count, blockAllocator: kCFAllocatorDefault, customBlockSource: nil,
        offsetToData: 0, dataLength: avcc.count, flags: 0, blockBufferOut: &block)
    guard status == noErr, let block = block else { fail("Block allocation failed") }
    status = avcc.withUnsafeBytes { CMBlockBufferReplaceDataBytes(with: $0.baseAddress!, blockBuffer: block,
        offsetIntoDestination: 0, dataLength: avcc.count) }
    guard status == noErr else { fail("Block copy failed") }
    var timing = CMSampleTimingInfo(duration: .invalid, presentationTimeStamp: CMTime(value: counter, timescale: 60), decodeTimeStamp: .invalid)
    counter += 1
    var sample: CMSampleBuffer?
    var length = avcc.count
    status = CMSampleBufferCreateReady(allocator: kCFAllocatorDefault, dataBuffer: block,
        formatDescription: description, sampleCount: 1, sampleTimingEntryCount: 1,
        sampleTimingArray: &timing, sampleSizeEntryCount: 1, sampleSizeArray: &length, sampleBufferOut: &sample)
    guard status == noErr, let sample = sample else { fail("Sample creation failed") }
    status = VTDecompressionSessionDecodeFrame(current, sampleBuffer: sample, flags: [], frameRefcon: nil, infoFlagsOut: nil)
    guard status == noErr else { fail("Decode failed \(status)") }
    VTDecompressionSessionWaitForAsynchronousFrames(current)
}
resetSession()
FileHandle.standardError.write(Data("frames=\(outputFrames) CPU-swizzled-bytes=\(copiedBytes)\n".utf8))
