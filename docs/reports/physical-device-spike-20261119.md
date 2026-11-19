# US-056 — Physical device spike, 2026-11-19

Status: completed feasibility spike, 2026-11-19. Transport selected: scrcpy v5.0.1.

## Final user-approved decision

The user explicitly waived the camera measurement and approved continuing the goal.
US-056 is complete with optical latency **unmeasured**, not passed. The observed
CPU/RAM/FPS and embedded input evidence support implementing physical-device support
with pinned scrcpy v5.0.1. They do not qualify production performance. The CPU/pipe
prototype requires optimization; US-064 budgets remain future acceptance targets.
Display/refresh, cable specification and non-synchronized input timing remain stated
limitations of diagnostic measurements, not claims of controlled responsiveness.
All QA processes/artifacts were cleaned; no camera session is scheduled or required
to close this spike. US-057–068 remain separate planned work. ADR 0060 is Accepted.

The remainder is the chronological research/test record. Earlier statements about
pending camera work, Proposed ADR status or incomplete spike are historical and
superseded by this final decision. Numerical measurements remain unchanged.

## Observed host

- Apple M2, 16 GB RAM; macOS 27.0.1 (26A434), ARM64.
- Displays reported: 2560×1600 internal Retina, 1920×1080 external. Refresh rate and active benchmark display not yet established.
- Selected SDK ADB: 37.0.1-15733141, protocol 1.0.41.
- Initial and post-user-plug ADB inventory: only the existing emulator, no physical device listed. No commands sent to that emulator.
- scrcpy not found on PATH. `/opt/homebrew/bin/android` resolves to Homebrew's old SDK tools/4333796 installation, not evidence of the new Android CLI.
- No capture helper, test APK or viewer started yet.

## Research findings

[scrcpy architecture](https://github.com/Genymobile/scrcpy/blob/master/doc/develop.md)
uses separate video/control sockets and compressed video packet metadata. Embedded
Iced needs a decoder plus a control client; launching the native viewer is only a
baseline. Preserve compressed frame dependencies; bound decoded presentation queues.
Protocol/client and server must be pinned together before any device trial.

[QtScrcpyCore CMake](https://github.com/barry-ran/QtScrcpyCore/blob/main/CMakeLists.txt)
requires Qt Widgets/Network and includes a macOS VideoToolbox decoder. Its
[license](https://github.com/barry-ran/QtScrcpyCore/blob/main/LICENSE) is Apache-2.0;
transitive Qt/codec dependencies need separate packaging review if selected.
Embedding this library adds C++/Qt interop, not evidence of faster Rust rendering.

[Google Device Hub](https://developer.android.com/tools/agents/android-cli/commands/device_hub)
launches a separate viewer and downloads an extension on first use. The old local
SDK command must not be mistaken for this tool. No Google baseline measured yet.

## Reproducible measurement protocol

1. Bind explicitly to the user-designated physical USB transport; record model,
   Android/OEM version, cable, display refresh, tool versions and server checksum.
2. Use a deterministic QA screen: animated frame counter plus touch indicator.
   Start with H.264, maximum dimension 1280, 8 Mbps, 60 FPS, audio disabled. Match
   effective output size/codec across candidates; report unsupported controls.
3. After a short warm-up, run three 120-second repetitions per candidate, same
   animation and input sequence. Record decoded/presented frame intervals separately,
   drops/reconnects, process CPU and RSS (including owned helper processes).
4. For input-to-visible latency use an external high-frame-rate camera showing the
   physical input action and host response, or equivalent validated end-to-end
   instrumentation. Report sampling resolution. Decode time alone is not latency;
   without instrumentation mark latency unmeasured, never substitute advertised values.
5. Prototype must render and inject input within Iced. Log actual decoder path,
   frame conversion/upload copies and queue bounds. Compare with native scrcpy
   before setting production budgets; do not choose thresholds after qualification.
6. Stop only owned viewers/helpers/tunnels. Preserve phone apps/accounts and the
   running emulator; remove temporary host outputs after recording concise evidence.

## Completion gaps

Physical ADB access and embedded video/touch are now proven on one Samsung phone.
Still open: controlled repeated performance/input trials, display/cable inventory,
end-to-end latency, comparative resource measurements, production budgets and ADR
decision. Existing native FPS windows are not a completed matched comparison.

## Tool preflight follow-up

- USB IORegistry now shows `SAMSUNG_Android`; ADB still lists only the emulator.
  This distinguishes a USB-visible phone from an authorized ADB transport. Exact
  phone model/OS remains unknown; no shell commands sent to the phone.
- Downloaded official scrcpy **v5.0.1** ARM64 archive into the isolated
  `/private/tmp/android-hub-us056-tools` workspace. Archive SHA-256:
  `33611e51977a8289e2e124b220f727895181ef2eb9060e0987a4dc2db9cf0d6b`.
- Bundled server SHA-256 matches the official release API digest:
  `764eb6f79811d5211fe9df341120882ba9994c7a61b897d7bf3fb662e53bc536`.
- `scrcpy --version` succeeds: SDL 3.4.18, libavcodec 63.1.102,
  libavformat 63.1.102, libavutil 61.1.102, libusb 1.0.30.
- Local help confirms explicit `--hwdec=videotoolbox`, `--print-fps`,
  `--no-clipboard-autosync`. This proves tool availability, not hardware decoder
  operation or measured latency. No viewer/helper has been started on a device.
- Baseline must set the existing SDK ADB explicitly, specify the physical serial,
  disable audio and clipboard autosync, and use H.264/max-size 1280/8M/60 FPS.
  Never invoke a command without an explicit target while the personal emulator runs.
- v5.0.1 is pinned for the baseline only; selection for production remains open.
  Temporary tool directory is owned by this ongoing spike and must be removed at
  final handoff; archive removed after extraction/checksum verification.

## CPU/RSS collector verification

Added `scripts/measure-display-process.py`: explicit PID allowlist, process
start-time/executable validation, exclusive output creation and a 120-second cap.
It observes only selected processes and never starts/stops them. Missing or changed
processes produce incomplete evidence rather than sampling a replacement. Four
unit tests and a one-second native ps self-process smoke passed; temporary smoke
output was removed. This is collector validation, not a phone benchmark.

Example after starting a verified viewer (replace PID and choose a new output):

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/measure-display-process.py --pid 12345 --seconds 120 --output /tmp/us056-baseline-run1.jsonl
```

Repeat `--pid` for known owned host helpers. Summed RSS may double-count shared
pages and excludes GPU/device memory; CPU is ps averaged utilization rather than
instantaneous frame cost. Do not use this output to infer FPS or end-to-end latency.

Latest ADB recheck still shows no physical transport; Samsung USB authorization
remains needed before baseline/prototype device trials. Tool preflight and collector preparation are complete; story remains in progress.

## Hardware gate — awaiting user action

Repeated read-only checks across three consecutive goal turns still list only
`emulator-5584` in ADB, while IORegistry sees `SAMSUNG_Android`. The exact cause
(debugging disabled, device policy, authorization or USB interface mode) is not yet
proven. No attempt was made to restart the shared ADB server or alter the phone.
The next required work is the real-device baseline, followed by the embedded
prototype trial and budget/ADR decision; none can be signed off without physical
ADB access. No engine is selected and no acceptance criterion is marked complete.

User action: enable USB debugging in Developer options, unlock the phone and
accept this Mac's RSA prompt if offered. If ADB remains absent, provide the phone
model and the debugging setting/error shown so diagnosis can be device-specific.

At this handoff the downloaded temporary scrcpy tools were removed. Version and
verified hashes above allow a reproducible download when the phone is ready.
No QA viewer, helper or tunnel was started; the existing emulator is untouched.

## Physical baseline run 1

ADB now authorized: Samsung SM-G996B, Android 15/API 35, build G996BXXSJHZC2,
1080×2400 at density 450. Existing emulator left untouched. Built and installed
the previously absent permission-free input QA fixture; optional benchmark intent
animates for at most 130 seconds, default fixture behavior unchanged.

Native scrcpy 5.0.1, explicit Samsung serial, H.264/max-size 1280/8M/60 FPS,
VideoToolbox requested, audio and clipboard autosync disabled. Five-second warm-up
then 120 CPU/RSS samples. Viewer exited and fixture force-stopped after measurement.

```json
{
  "run": 1,
  "samples": 120,
  "fps_log_windows": 123,
  "fps_min": 52,
  "fps_median": 60,
  "fps_mean": 59.67479674796748,
  "skipped_logged": 46,
  "cpu_ps_mean_percent": 12.245,
  "rss_peak_mib": 90.609375,
  "decoder_evidence": [
    "INFO: Renderer: metal",
    "DEBUG: Trilinear filtering disabled (not an OpenGL renderer)",
    "INFO: Interop: videotoolbox",
    "INFO: Video decoding: videotoolbox",
    "INFO: Texture (VideoToolbox): 576x1280"
  ]
}
```

FPS values are scrcpy log windows (including warm-up), not per-frame latency or
frame-time p95. Metrics cover the host viewer only, not device encoder/ADB server.
No input latency measured and no visual/Iced qualification yet. Two repeated runs,
input trials, Google comparison feasibility and embedded prototype remain open.

## Google CLI feasibility preflight

Downloaded the official ARM64 launcher from
`https://dl.google.com/android/cli/latest/darwin_arm64/android` into the spike temp
directory. Launcher SHA-256:
`3a7992f7b56974cddadeaeac66b2282c097a41c7660b39ead21a1cc8cc195143`.
`--version` bootstrapped runtime **1.1.16919734** and `device hub --help` succeeded.
The help exposes target/codec, but no matching max-size/bitrate/FPS controls;
a direct equivalent comparison needs inspection of viewer settings/effective output.
No Google viewer has been launched yet. Use `--no-metrics` for subsequent commands.

Unexpected bootstrap side effects: runtime downloaded to `~/.android/bin/android-cli`,
bundle `~/.android/cli/bundles/744e776f6c66365186918a66c8f132b32e4c6c59`,
plus CLI analytics/update/TOS marker files at 15:12 local time. Existing
`~/.android/cli/studio` predates the trial and must be retained, along with all
AVDs, SDK keys and emulator metrics. Before running the viewer, isolate the runtime
using ANDROID_USER_HOME (supported by launcher strings) and verify cleanup scope.
No installer script was executed and no shell PATH changed.

The Google bootstrap/download overlapped native baseline run 2. Treat that run as
potentially contaminated by host activity; do not silently use it as a controlled
comparison. Finish the current three-run capture, then repeat that slot in an idle
host window before calculating qualification budgets.

## Baseline runs 2 and 3

Both viewers exited and fixture stopped. Run 2 requires an idle-host repeat.

```json
[
  {
    "run": 2,
    "samples": 120,
    "fps_windows": 124,
    "fps_mean": 59.46774193548387,
    "fps_min": 52,
    "fps_median": 60.0,
    "cpu_mean_percent": 12.179166666666667,
    "rss_peak_mib": 89.3125,
    "skipped_logged": 63,
    "contaminated_by_google_bootstrap": true
  },
  {
    "run": 3,
    "samples": 120,
    "fps_windows": 124,
    "fps_mean": 59.725806451612904,
    "fps_min": 54,
    "fps_median": 60.0,
    "cpu_mean_percent": 11.403333333333334,
    "rss_peak_mib": 81.375,
    "skipped_logged": 32,
    "contaminated_by_google_bootstrap": false
  }
]
```

Google bootstrap runtime/bundle and newly created CLI markers have now been moved
into `/private/tmp/android-hub-us056-tools/google-user-home` for the next isolated
trial. Pre-existing `~/.android/cli/studio`, AVDs and keys were retained. All native
scrcpy viewers from runs 1–3 have exited; fixture is stopped.

## Embedded prototype protocol work

Added the experimental Rust wire module under
`crates/desktop/examples/physical-spike/protocol.rs`, based on pinned v5.0.1 source.
Unlike older examples, v5 video carries explicit capture-session packets followed
by media packets with separate config/key flags. Tests cover codec rejection,
session dimensions, bounded payload/truncation, touch geometry/release and key wire
encoding. Five standalone Rust tests passed. The test executable was removed.

This is only the protocol component: no Iced example/decoder or real input round
trip has passed yet. Full repository checks and native prototype remain required;
no production code or dependency graph changed in this step.

## VideoToolbox bridge preparation

Added experimental `decode.swift`; optimized Swift build passed on the local Mac.
Process-level checks passed for empty EOF, truncated header, oversized payload and
malformed Annex-B input. No device stream has been fed into this decoder yet.
Hardware-required VT session creation, SPS/PPS parsing, AVCC conversion and BGRA→RGBA
copy are implemented but positive decode, frame ordering and Iced presentation still
need real evidence. The synthetic timestamp is not an end-to-end latency clock.
No production dependency or main app bundle was changed.

## Embedded Iced proof

- `cargo build --locked -p android-hub --example physical-spike` passed. The standalone
  example is not included in the production application flow.
- Initial server attempts with disabled device metadata/dummy byte crashed in the
  Samsung native `libsfplugin_ccodec.so` (`CCodec::customConfigVideoEncoder`, stack
  protector abort), before decoder output. Native scrcpy with the same requested
  video parameters still worked. Switching the prototype to the standard dummy-byte
  and metadata handshake and omitting redundant default options produced video.
  This is correlation, not an isolated root cause or general Samsung workaround.
- Phone encoder: `c2.exynos.h264.encoder`; actual output 576×1280. Host bridge requires
  VideoToolbox hardware decode. One 150-second bounded run decoded 7,806 frames and
  CPU-swizzled 23,020,830,720 bytes. Animation runs only 130 seconds; these counts are
  not a sustained 150-second FPS claim or presentation-time measurement.
- Native UI screenshot showed the real offline fixture inside the Iced window,
  including its animated magenta block and changing frame counter. Clicking and
  dragging within that embedded image changed the phone fixture from Down 0 / Up 0
  to **Down 2 / Up 2 / Moves 1 / Held false**. The same live frame displayed the
  updated counters. No external scrcpy viewer was used for this input trial.
- Clicked the example's Close button; runner observed exit 0 and removed its unique
  ADB forward and temporary server JAR, then force-stopped the QA fixture. Main Hub
  and the personal emulator were not targeted.
- CUA initially spent ~145 seconds attaching to the new bundle, missing the first
  bounded session. Reusing the binding in the next run allowed direct observation.
  This tool delay is not stream latency.

## Google viewer feasibility result

CLI 1.1.16919734 successfully downloaded its Device Hub pack. In this terminal,
`ELECTRON_RUN_AS_NODE=1` initially caused `Cannot find module electron`; launching
with that variable unset opened the official viewer and showed the phone lock
screen. The user was asked to unlock the phone rather than bypass its lock.
Observed agent arguments use AVC, max size 1080×1920 and bitrate 2,500,000, with
an audio stream. Public CLI help exposes codec/device but not matched FPS/size/
bitrate or audio-off controls. Therefore this launch is a feasibility observation,
not an equal-configuration performance comparison with the scrcpy/Iced trial.
The QA viewer was terminated. Its Electron profile used the normal Application
Support directory despite ANDROID_USER_HOME; inspect ownership before final cleanup.

## Verification after embedded trial

`CARGO_NET_OFFLINE=true bash scripts/check.sh` passed outside the filesystem sandbox:
workspace tests, all-target Clippy with warnings denied, metrics tests/Clippy,
36 Python tests, documentation checks (55 files), shell syntax and formatting.
The first sandboxed run failed only two backup fixture tests with OS error 1;
the unrestricted rerun passed them. No production devices were started by this script.

Post-trial process checks found no scrcpy/Iced decoder/Google viewer process.
ADB forward list was empty and no scrcpy/screensharing helper process remained on
the Samsung. The Google Electron profile born at 15:24:34 during this trial was
moved from Application Support into the QA directory (1.8 MB), preserving it for
scoped cleanup. The offline fixture APK remains installed for the remaining
benchmark trials, force-stopped between runs. QA tools/build output are still
needed by this in-progress spike; remove them at final handoff.

## User-confirmed connection

The user confirmed the Samsung is connected directly to the MacBook by USB, with
no intermediate hub. Cable model/specification and which host display was used
remain unconfirmed; direct connection does not establish USB link speed or screen
refresh rate. Current preflight sees only the Samsung (the personal emulator is no
longer present); no emulator lifecycle action was performed by the spike.

## Embedded resource trial 1

Configuration remains H.264, 576×1280 actual output, 8 Mbps, cap 60 FPS, no audio,
VideoToolbox-required decoder and CPU RGBA bridge. Iced is the Cargo dev profile
(optimized plus debuginfo); Swift is compiled with `-O`. These build modes differ
from the distributed native scrcpy binary and must accompany any comparison.

After five seconds warm-up, the collector recorded 120 samples over 120.005 seconds
for both owned PIDs (viewer and decoder). Mean summed ps CPU was **40.63%** (100%
means one core); peak summed RSS **171.75 MiB**. Summed RSS can double-count shared
pages and excludes GPU allocations. No concurrent compilation/download was started.
The host's ordinary applications were not stopped or treated as controlled load.

Within the counter window 5–125 seconds, measured over 118.975 seconds between
actual log timestamps, decode rate was **59.987 frames/s**, image-handle selection
rate **55.793 frames/s**. Selection is not measured display presentation/scan-out.
No stream error was reported (`failed=false`). Three native-UI click/drag pairs
were issued during this run, approximately 10/50/100 seconds into it; tool timing
was not precise and is not a latency measurement. The fixture stayed visible and
animation advanced. No frame-time p50/p95 or end-to-end latency is claimed.

The complete sample file was `embedded-measure-1.jsonl` in the temporary tools
directory. Close returned viewer exit 0; the runner removed its owned forward/JAR
and force-stopped the fixture. This is one measured prototype repetition only;
two additional repetitions and matched input timing remain required.

Repository checks were rerun after adding the counters and passed (including 36
Python tests and all-target Clippy). Process inspection found no QA viewer/decoder
left running; the disposable `Physical Spike QA.app` bundle was removed. Source,
measurement logs and tools needed for the remaining repetitions are retained in
the in-progress QA directory. No commit or production bundle update was made.

## Pending optical latency session

The user agreed they can help with a camera measurement later. This is not yet
measured and does not constitute performance acceptance. Use a second camera
(the Samsung being mirrored must remain on the QA fixture), fixed at 240 fps if
available, framing both the physical mouse button/finger and the host display.
Record the actual capture frame rate, not the slow-motion playback/export rate.

Use the same visible fixture, host display/refresh, wired mouse, USB cable and
video settings for native scrcpy and Iced. With the benchmark animation active,
press and hold within the fixture: the moving marker changes magenta to green
on DOWN and returns on release. Keep each press held at least 200 ms so the colour
transition is distinguishable. Collect 30 presses per viewer over a bounded
120-second run, alternating viewer order between sessions if repeated. No personal
apps or notifications should be in the recording. Avoid Home/Back during sampling.

For each event, count frames from visible button actuation to the first host-display
frame showing green, then compute `frame_count / capture_fps * 1000` milliseconds.
Report raw counts, median and nearest-rank p95 with at least ±one camera-frame
quantization uncertainty (4.17 ms at 240 fps), plus physical-actuation ambiguity.
Visible mechanical actuation is a proxy for the electrical input event; do not
claim sub-frame precision. Reject clips where either event cannot be identified.
This includes transport, Android rendering, encoding, decoding and host display;
Swift decode timing and Iced handle counters cannot replace this measurement.
A successful camera session remains required before claiming measured input latency.

## Repeated results and decision gate

Sanitized numerical evidence is retained in
[metrics JSON](physical-device-spike-20261119-metrics.json): 120 CPU/RSS samples
per run, native FPS windows and Iced counters, with source SHA-256 identifiers.
No phone serial, screenshots, frame pixels or personal application content is stored.

| Viewer / trial | Mean CPU, % one core | Peak summed RSS, MiB | Decode/native FPS | Iced selected FPS |
| --- | ---: | ---: | ---: | ---: |
| Native 1 | 12.245 | 90.61 | 59.667 | n/a |
| Native 3 | 11.403 | 81.38 | 59.725 | n/a |
| Native replacement | 11.608 | 91.33 | 59.808 | n/a |
| Iced 1 | 40.631 | 171.75 | 59.987 | 55.793 |
| Iced 2 | 46.995 | 162.98 | 60.005 | 55.512 |
| Iced 3 | 51.520 | 192.59 | 60.005 | 55.848 |

The native replacement finished exit 0 and replaces the earlier run contaminated
by the Google bootstrap download. Native FPS values here use the first 120 log
windows; they are not exactly synchronized with the CPU collector's five-second
warm-up. Iced rates use actual counter timestamps inside the 5–125 second window.
All collectors completed 120 samples. No Iced `failed=true` or unexpected steady
stream disconnect was logged. These are diagnostic results, not a rigorously
controlled display benchmark: host background applications and active display were
not fixed, and returned UI screenshots varied in pixel scale between sessions.

All three Iced runs used the same animated fixture and three click/drag pairs;
UI automation timing varied (roughly early/middle/late within the sample window),
so this is a repeated input pattern, not deterministic timing or latency evidence.
Trial 2 ended with Down 6 / Up 6 / Held false. Trial 3 also visibly advanced and
responded to input. Native repeats measured animation; they did not replicate the
same UI input timing. Do not treat these runs as a matched input-latency comparison.

**Decision:** embedded scrcpy v5.0.1 is feasible and remains the pinned candidate;
the CPU RGBA/pipe renderer is **no-go for shipping unchanged**. Its resource use
and handle-selection rate miss the future US-064 targets. Google Device Hub was
usable as an external viewer but its CLI did not offer matched benchmark controls;
QtScrcpyCore adds C++/Qt integration without evidence of a performance advantage.
ADR 0060 remains Proposed pending optical latency and a reviewed production path.
US-064 now states budgets before production implementation (CPU ≤25%, RSS ≤256 MiB,
58 FPS actual presentation, optical median ≤60 ms / p95 ≤100 ms, no stream errors).
These are targets, not achievements, and must not be relaxed to pass this prototype.

**Remaining acceptance evidence:** camera-assisted latency, confirmed host
screen/refresh and cable specification, and tighter matching of input timing if
making a comparative responsiveness claim. The user agreed to help with camera
measurement later. No further game/soak tests are part of this spike session.

## Cleanup at camera handoff

All QA viewer/decoder/scrcpy/screensharing processes were absent and ADB forwarding
was empty before cleanup. `adb -t 151 uninstall local.androidhub.inputqa` returned
Success; the empty Google `.studio` helper directory was removed with `rmdir`.
Removed the 2.7 GiB workspace target, metrics target, 730 MiB temporary tool tree
(including both temporary app bundles, Google runtime/profile, helper binaries, raw
logs and runner scripts), fixture build directory and fixture build log. Main Hub,
its release artifacts and personal phone/AVD data were not removed. No build was
rerun after cleanup. Source plus sanitized metrics/report remain for review.
The fixture/tools will need to be rebuilt or downloaded with the recorded pins for
the user-assisted camera session; this handoff is not US-056 completion.
