# Physical display spike — US-056 (completed feasibility spike)

Experimental code, not wired into the shipped application. Camera measurement was
waived by the user; end-to-end latency remains unmeasured. The Rust protocol adapter
and server are pinned together to scrcpy **v5.0.1**:

- https://github.com/Genymobile/scrcpy/blob/v5.0.1/doc/develop.md
- https://github.com/Genymobile/scrcpy/blob/v5.0.1/app/src/control_msg.c

Build the Cargo example `physical-spike` and compile `decode.swift` with `xcrun swiftc
-O`. The example takes two positional arguments: an already-owned localhost ADB
forward port and the absolute decoder executable path. It exits after 150 seconds.
The operator owns the phone-side server, forward and temporary JAR: create unique
resources for an explicitly validated physical transport and remove only those
resources after the example exits. Never target a device implicitly.

Start the matching server with H.264, audio disabled, control enabled and
`tunnel_forward=true`. Keep default device metadata and dummy byte enabled: the
client consumes the dummy byte, connects control, then consumes 64 bytes of device
metadata and the four-byte codec before reading v5 session/media packets. Disable
clipboard autosync. Current measured configuration is max-size 1280, 8 Mbps, 60 FPS.

`decode.swift` accepts length-prefixed Annex-B H.264 access units on stdin and emits
width/height/length-prefixed RGBA frames on stdout. It requires VideoToolbox hardware
decode, converts Annex-B to AVCC and explicitly CPU-swizzles BGRA into RGBA. There
are compressed-buffer copies, a decoded-frame copy, pipe transfers and an Iced
texture upload. This is not zero-copy. Synthetic 60 Hz sample timestamps preserve
order only; they are not source PTS or latency evidence.

On the Samsung SM-G996B/Android 15 trial, Iced displayed the live QA fixture and
click/drag produced Down 2, Up 2, Moves 1, Held false. A separate 150-second session
exited normally after decoding 7,806 frames. This proves feasibility, not production
performance or broad OEM compatibility. See the linked report for the encoder crash
observed with an earlier customized server handshake.

Limitations: synchronous control writes can block the UI up to 250 ms; capture-session
changes are parsed but not coordinated with active input; focus-loss/reconnect and
control responses are not production-ready. Decoded handoff holds only the latest
frame; compressed packets remain ordered. Output is fixed at 288 logical pixels
wide. No audio, clipboard, APK or production Hub integration is implemented here.
One-second stderr counters report `decoded` frames and `selected` frames handed to
an Iced image handle. `selected` is not GPU presentation or display scan-out; neither
counter is end-to-end latency. Measure the viewer and decoder process together.

Wire tests run through Cargo or standalone `rustc --test protocol.rs`. Run repository
checks after edits; unit tests alone do not prove video, input or end-to-end latency.

Evidence: [spike report](../../../../docs/reports/physical-device-spike-20261119.md).
