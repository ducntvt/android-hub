# 0060 — Separate physical-device sessions from owned emulator lifecycle

- **Status**: Accepted
- **Date**: 2026-11-19
- **Deciders**: Android Hub maintainer; user-approved US-056 camera waiver
- **Supersedes**: none

## Context

Hub currently models owned AVDs with an emulator PID, console port and authenticated
gRPC display. A connected phone has an ADB transport but no Hub-owned emulator
process or disk image. Reusing AVD deletion/shutdown semantics would be incorrect.
The user requested a feature branch and stories for discovery, embedded display,
control and performance comparison before implementation.

## Decision

Use a device kind and capability boundary while retaining existing AVD profiles
and gRPC streaming. A physical session has its own transport identity, connection
generation and owned helper/tunnel resources. UI shares device selection, frame
presentation and applicable controls; infrastructure validates every operation's
target and capability. Domain stays free of I/O.

Start with USB discovery through ADB and one physical device alongside one emulator.
Do not require emulator SDK/images or Java for a physical-only workspace. ADB setup,
dependencies and any downloads must be explicit and versioned.

Select scrcpy **v5.0.1**, pinning the Android server and Rust client protocol together,
for USB physical-device implementation. The spike proves VideoToolbox hardware
decode and embedded Iced input on one Samsung phone. The CPU RGBA/pipe decoder
bridge is experimental: optimize the production render path in US-059 and verify
it in US-064. GPU interop/zero-copy and low end-to-end latency are not proven.

Quit stops Hub-owned emulators as before, but only disconnects physical sessions.
Never turn off/reset a phone, apply AVD backup/restore/delete to it, kill the shared
ADB server or delete another tool's tunnels. Uninstall must preserve phone content
and user-exported files. Do not replay input or writes across reconnect generations.

## Spike evidence and current gate

US-056 has proven embedded video and click/drag on Samsung SM-G996B / Android 15
using scrcpy **v5.0.1** on both ends, a Rust wire adapter and a VideoToolbox-required
Swift helper. Three 120-second trials decode approximately 60 FPS, but select only
55.5–55.8 frames/s for Iced and consume mean summed CPU 40.6–51.5% of one core.
The CPU RGBA/pipe bridge is therefore **not approved as the production renderer**.

Use v5.0.1 as the pinned transport for further implementation, not an unversioned protocol.
The source of truth for server checksum, benchmark limitations and Google/Qt
comparison is the [spike report](../reports/physical-device-spike-20261119.md).
A standard metadata handshake worked after the earlier customized server launch
crashed in Samsung encoder code; the exact cause remains unresolved.

Go for physical-device implementation using this transport; no-go for shipping the
CPU/pipe renderer unchanged or claiming performance qualification. On 2026-11-19
the user waived the camera measurement for US-056 and accepted continuing with
latency explicitly unmeasured. This resolves the spike gate, not production
qualification. US-064 retains its performance targets; camera measurement is not
a prerequisite to starting US-057–063 and must not be reported as passed.

## US-057 implementation boundary

Domain now owns namespaced IDs, operation capabilities and monotonic physical
session tokens. Desktop selection and APK confirmation bindings preserve device
kind; AVD-only handlers reject physical targets, including identical raw IDs.
Infrastructure exposes a typed `DeviceBackend` facade; raw AVD mutations are
crate-private. The emulator registry schema and process ownership remain unchanged.
Physical metadata is not persisted. Discovery and stream workers are subsequent
stories and must validate session tokens before applying asynchronous results.

## US-058 discovery transport

Use the ADB smart socket on loopback port 5037 for `host:version`,
`host:track-devices-l` and read-only metadata queries selected by transport ID.
This avoids the CLI's automatic server replacement on a version mismatch and
keeps disconnect detection tied to a persistent connection. Protocol 41 is
required; failures invalidate tokens and retry with bounded backoff. Only
positive USB transports are probed; network and unknown transports are labeled,
and known emulators are excluded. No phone settings are modified.

ADB executable selection verifies local `adb version` and persists only its path.
For this story server startup is explicit via the documented Terminal command;
there is no Platform-Tools downloader, license auto-acceptance or custom server
endpoint. The installed emulator SDK remains an optional default path suggestion.

Protocol references: [ADB services](https://android.googlesource.com/platform/packages/modules/adb/+/HEAD/docs/dev/services.md),
[track-devices CLI](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/commandline.cpp).

## US-059 embedded display

The macOS implementation embeds the official scrcpy 5.0.1 server with a checked
SHA-256 and Apache-2.0 notices. User-started, video-only sessions upload a random
owned JAR, select the current USB transport and connect directly to its unique
ADB localabstract socket. There are no shared forward mappings, external viewers,
audio, input, clipboard synchronization, power changes or automatic reconnects.
Stop, selection change, USB invalidation and quit cancel the session. Server
self-cleanup removes its JAR; fallback cleanup validates the same transport and
only removes that session's generated path.

Replace the spike's Swift subprocess/pipe with a statically linked C shim calling
VideoToolbox in-process. Require and query hardware H.264 decode; fail visibly if
unavailable. Compressed packets remain ordered in a bounded queue of two. Decode
waits per access unit, retains only the latest pixel buffer, and uses Accelerate
to copy BGRA into RGBA off the UI thread before the Iced upload. Capture and UI
session epochs discard obsolete frames. This still copies pixels; it is **not
zero-copy**, and US-064 performance qualification remains outstanding.

Initial controls request 30/60 FPS and a 720/1280/1920-pixel maximum long edge.
These are stream caps, not guaranteed delivered FPS or phone display settings.
The native protected-content test displays black, with a permanent explanation
of lock/protected/OEM capture limits. Control is deferred to US-060.

## Consequences

- Shared workspace can represent both device kinds without fake emulator fields.
- Discovery, stream, APK and clipboard need target-aware routing and cancellation.
- Physical-only setup and packaging need separate dependency checks.
- Native phone evidence is required; unit tests alone cannot qualify OEM capture,
  input, hardware decoding or latency. No physical Android version matrix is claimed yet.
- Wi-Fi/audio/capture/file transfer remain separate future stories.
- This accepts the implementation direction, not shipped functionality or support
  for untested OEMs. Only Samsung SM-G996B / Android 15 has spike evidence.

## Alternatives considered

### Extend Profile with optional physical fields

This would mix AVD storage and process ownership with connected-device metadata.
Prefer separate typed records and explicit capability checks at the backend boundary.

### Launch an external viewer as the final feature

Useful as a benchmark, but it does not satisfy the requested screen inside Hub.

### Embed QtScrcpyCore or Google's mirroring implementation

Google Device Hub works externally but its CLI lacks matched benchmark controls.
QtScrcpyCore introduces C++/Qt integration and transitive packaging obligations
without measured advantage here. Prefer the proven scrcpy protocol path; neither
alternative is claimed slower without comparative evidence.

### Build a new Android capture/control server

Would add OEM/API maintenance before proving user benefit. Prefer reuse if the spike
finds a suitable implementation; a custom server requires a separate justified decision.

## References

- [Epic E02](../stories/epics/E02-physical-devices/README.md)
- [US-056](../stories/epics/E02-physical-devices/US-056-physical-device-streaming-spike.md)
- [Current architecture](../ARCHITECTURE.md)
- [ADB](https://developer.android.com/tools/adb)
- [scrcpy development](https://github.com/Genymobile/scrcpy/blob/master/doc/develop.md)
- [Google Device Hub](https://developer.android.com/tools/agents/android-cli/commands/device_hub)
- [QtScrcpyCore](https://github.com/barry-ran/QtScrcpyCore)
