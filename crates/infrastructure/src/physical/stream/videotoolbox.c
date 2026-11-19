// In-process H.264 decoder. No filesystem/device access, no logging of frames.
// One submitter; the callback and frame consumer share only a bounded latest slot.
#include <VideoToolbox/VideoToolbox.h>
#include <Accelerate/Accelerate.h>
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    VTDecompressionSessionRef session;
    CMVideoFormatDescriptionRef format;
    pthread_mutex_t mutex;
    CVPixelBufferRef latest;
    OSStatus error;
} HubDecoder;

static void output(void *context, void *frame_context, OSStatus status,
                   VTDecodeInfoFlags flags, CVImageBufferRef image, CMTime pts, CMTime duration) {
    (void)frame_context; (void)flags; (void)pts; (void)duration;
    HubDecoder *decoder = context;
    pthread_mutex_lock(&decoder->mutex);
    if (status) decoder->error = status;
    if (image && !status) {
        CVPixelBufferRetain(image);
        if (decoder->latest) CVPixelBufferRelease(decoder->latest);
        decoder->latest = image;
    }
    pthread_mutex_unlock(&decoder->mutex);
}
void hub_vt_destroy(HubDecoder *decoder) {
    if (!decoder) return;
    if (decoder->session) {
        VTDecompressionSessionWaitForAsynchronousFrames(decoder->session);
        VTDecompressionSessionInvalidate(decoder->session);
        CFRelease(decoder->session);
    }
    if (decoder->format) CFRelease(decoder->format);
    if (decoder->latest) CVPixelBufferRelease(decoder->latest);
    pthread_mutex_destroy(&decoder->mutex);
    free(decoder);
}
HubDecoder *hub_vt_create(const uint8_t *sps, size_t sps_len,
                          const uint8_t *pps, size_t pps_len, int32_t *error) {
    *error = -1;
    if (!sps || !pps || !sps_len || !pps_len || sps_len > 65536 || pps_len > 65536) return NULL;
    HubDecoder *decoder = calloc(1, sizeof(*decoder));
    if (!decoder) return NULL;
    if (pthread_mutex_init(&decoder->mutex, NULL)) { free(decoder); return NULL; }
    const uint8_t *sets[] = {sps, pps};
    const size_t lengths[] = {sps_len, pps_len};
    OSStatus status = CMVideoFormatDescriptionCreateFromH264ParameterSets(kCFAllocatorDefault,
        2, sets, lengths, 4, &decoder->format);
    if (status) goto failed;
    CMVideoDimensions size = CMVideoFormatDescriptionGetDimensions(decoder->format);
    if (size.width <= 0 || size.height <= 0 || size.width > 4096 || size.height > 4096) { status = -2; goto failed; }
    const void *decoder_keys[] = {kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder};
    const void *decoder_values[] = {kCFBooleanTrue};
    CFDictionaryRef specification = CFDictionaryCreate(kCFAllocatorDefault, decoder_keys, decoder_values, 1,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    int32_t pixel_format = kCVPixelFormatType_32BGRA;
    CFNumberRef number = CFNumberCreate(kCFAllocatorDefault, kCFNumberSInt32Type, &pixel_format);
    const void *pixel_keys[] = {kCVPixelBufferPixelFormatTypeKey};
    const void *pixel_values[] = {number};
    CFDictionaryRef pixels = CFDictionaryCreate(kCFAllocatorDefault, pixel_keys, pixel_values, 1,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    VTDecompressionOutputCallbackRecord callback = {output, decoder};
    status = VTDecompressionSessionCreate(kCFAllocatorDefault, decoder->format, specification,
        pixels, &callback, &decoder->session);
    CFRelease(pixels); CFRelease(number); CFRelease(specification);
    if (status) goto failed;
    CFTypeRef hardware = NULL;
    status = VTSessionCopyProperty(decoder->session, kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder,
        kCFAllocatorDefault, &hardware);
    bool actual_hardware = hardware && CFEqual(hardware, kCFBooleanTrue);
    if (hardware) CFRelease(hardware);
    if (status || !actual_hardware) { if (!status) status = -3; goto failed; }
    *error = 0;
    return decoder;
failed:
    *error = status;
    hub_vt_destroy(decoder);
    return NULL;
}
int32_t hub_vt_submit(HubDecoder *decoder, const uint8_t *bytes, size_t length, int64_t pts_us) {
    if (!decoder || !bytes || !length || length > 8 * 1024 * 1024) return -1;
    CMBlockBufferRef block = NULL;
    CMSampleBufferRef sample = NULL;
    OSStatus status = CMBlockBufferCreateWithMemoryBlock(kCFAllocatorDefault, NULL, length,
        kCFAllocatorDefault, NULL, 0, length, 0, &block);
    if (status) return status;
    status = CMBlockBufferReplaceDataBytes(bytes, block, 0, length);
    if (status) goto done;
    CMSampleTimingInfo timing = {kCMTimeInvalid, CMTimeMake(pts_us, 1000000), kCMTimeInvalid};
    status = CMSampleBufferCreateReady(kCFAllocatorDefault, block, decoder->format, 1,
        1, &timing, 1, &length, &sample);
    if (status) goto done;
    status = VTDecompressionSessionDecodeFrame(decoder->session, sample, 0, NULL, NULL);
    // Bound decoder work: complete each access unit; never discard dependent
    // compressed packets. Only already-decoded output may replace an older frame.
    if (!status) status = VTDecompressionSessionWaitForAsynchronousFrames(decoder->session);
done:
    if (sample) CFRelease(sample);
    CFRelease(block);
    return status;
}
CVPixelBufferRef hub_vt_take(HubDecoder *decoder, int32_t *error) {
    pthread_mutex_lock(&decoder->mutex);
    CVPixelBufferRef frame = decoder->latest;
    decoder->latest = NULL;
    *error = decoder->error;
    pthread_mutex_unlock(&decoder->mutex);
    return frame; // transfers one retain to caller
}
int32_t hub_vt_frame_info(CVPixelBufferRef frame, uint32_t *width, uint32_t *height) {
    size_t w = CVPixelBufferGetWidth(frame), h = CVPixelBufferGetHeight(frame);
    if (!w || !h || w > 4096 || h > 4096 || CVPixelBufferGetPixelFormatType(frame) != kCVPixelFormatType_32BGRA) return -1;
    *width = (uint32_t)w; *height = (uint32_t)h;
    return 0;
}
int32_t hub_vt_copy_rgba(CVPixelBufferRef frame, uint8_t *target, size_t length) {
    uint32_t width, height;
    if (hub_vt_frame_info(frame, &width, &height) || length != (size_t)width * height * 4) return -1;
    CVReturn status = CVPixelBufferLockBaseAddress(frame, kCVPixelBufferLock_ReadOnly);
    if (status) return status;
    void *base = CVPixelBufferGetBaseAddress(frame);
    vImage_Error result = -1;
    if (base) {
        vImage_Buffer source = {base, height, width, CVPixelBufferGetBytesPerRow(frame)};
        vImage_Buffer destination = {target, height, width, (size_t)width * 4};
        const uint8_t permutation[] = {2, 1, 0, 3}; // BGRA -> RGBA, vectorized CPU copy
        result = vImagePermuteChannels_ARGB8888(&source, &destination, permutation, kvImageNoFlags);
    }
    CVPixelBufferUnlockBaseAddress(frame, kCVPixelBufferLock_ReadOnly);
    return (int32_t)result;
}
void hub_vt_release_frame(CVPixelBufferRef frame) { if (frame) CVPixelBufferRelease(frame); }
