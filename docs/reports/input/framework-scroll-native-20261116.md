# Framework scroll QA — 2026-11-16

QA bundle: /tmp/android-hub-ui-qa-20261116/Android Hub QA.app; isolated data root. Guest Android 3 / hub-1794811158106 / emulator-5588, API 36. Main app and personal guests unchanged.

Initial native attempt failed before helper creation. Read-only command confirmed installed adb does not support get-transport-id. Replaced it with exact online serial selection from adb devices -l and numeric transport_id validation; added tests for offline, missing, prefix mismatch, zero and duplicate IDs.

After rebuild/relaunch, native wheel down one CUA page at host (1440,923) moved Settings from Google/Network/Apps to Passwords/Digital Wellbeing/Accessibility. Wheel up returned to Google/Network/Apps. Native touch drag from (1440,1060) to (1440,650) then moved the list down again. Framework helper injection therefore works on this API 36 guest, and touch drag still works after wheel.

Read-only guest process inspection after these events showed one app_process helper (PID 3594) with one parent shell (PID 3592). This observation supports persistent-channel setup; it is not a latency or throughput benchmark.

Remaining: exact nested target coordinates, horizontal axis, two guests and outside routing, natural trackpad behavior, reconnection and helper teardown. Story remains in_progress and ADR 0036 remains Proposed. QA is left running in Settings for the next checks; the main app bundle is unchanged.

## Independent target fixture

Built tests/android-input via scripts/build-input-qa-apk.sh. Installed only after
ADB emu avd name confirmed hub-1794811158106 at port 5588; opened through native
launcher UI. Fixture installation is not Hub APK UI test evidence.

Guest canvas host bounds approximately x1180..1680, y306..1195, guest 720x1280.
Native down at host (1510,1060) reported local view x475/y949, H0/V-10; only zone3
changed to V-10. Native up at (1300,600) reported x172/y287, H0/V10; only zone1
changed to V10, zone2 remained zero and zone3 retained V-10. Coordinates account
for the Android content inset. Touch-down counter stayed zero throughout.
This distinguishes correct event coordinates from the rejected center-pointer path.

Native right one page and left three pages at (1300,600) produced no observable
new event/counter change. Retrying with pixel distance was rejected by CUA:
"macOS scroll accepts pages, not pixels." Horizontal native validation is unresolved;
this is not yet attributed to the app or the tooling. Local Iced/Winit conversion
contains both axes; source support alone does not prove runtime delivery.

QA remains running with fixture visible for two-guest/outside/reconnect checks.

## Display-session reconnect

Switched from Android3 to stopped QA Lifecycle through the sidebar in single-device
mode. Read-only guest ps showed no ScrollBridge process. Selected Android3 again:
fixture values stayed zone1 V10, zone2 V0, zone3 V-10 (no replay observed).
New native down at host (1400,820) reached zone2, local x316/y603, V-10;
zone1/3 were unchanged and touch count stayed zero. This proves normal selection
teardown/reconnect, not forced transport failure during an in-flight event.

Read-only process check after new wheel: new helper app_process PID3896 (old PID3594
had exited). Added protocol tests covering consecutive reply boundaries, EOF,
truncated/invalid UTF-8, and overlong output without newline while writer stays open.
Workspace: 105 passed, 20 opt-in ignored; fmt and clippy -D warnings pass.
Bundled DEX rebuild exactly matched SHA256
f147777204b099f1bb4e618e288a293523d12771dffe7d6f9a271f2e784a124a.
Latest reply-reader refactor is in source only; the running QA bundle still uses
its equivalent earlier bounded reader. Main bundle remains unchanged.

## Two-guest native routing

Booted QA Lifecycle (hub-1794811024355, port5586) from Hub, confirmed identity and
boot completed, installed the same offline fixture, and launched it from app drawer.
Enabled Hai máy with QA Lifecycle selected. Left canvas: QA Lifecycle; right:
Android3. Baseline left zones V0/0/0; right V10/-10/-10; all H0 and touch counters0.

Native down at host (1100,920) changed only left zone3 to V-10, reported local
x233/y916. Right stayed V10/-10/-10. Native up at (1770,550) changed only right
zone1 to V20, reported local x504/y282; left stayed V0/0/-10. Sidebar selection
remained QA Lifecycle: routing follows the hovered canvas, not selected device.
Native down outside both canvases at (2100,900) changed neither guest counter.
No synthetic touch was observed. Both QA guests remain running with fixtures.

This closes the basic two-guest/outside routing smoke. Horizontal physical trackpad
and forced transport failure during active input remain pending. Two idle fixtures
are not evidence of two-game performance or 60-minute stability.

## Host cache robustness

Added atomic staged DEX cache publication after code review found a race where a
second caller could read a final path still being written. Fault-injection preserves
previous bytes and removes staging; eight concurrent callers receive complete DEX;
a partial regular cache repairs; a symlink is rejected without changing its target.
Workspace tests now 107 passed / 20 opt-in ignored; fmt and clippy pass.
These changes are not yet in the running QA bundle or main bundle.

## Latest QA package regression

Updated QA bundle with bounded-reader refactor, atomic host cache publication and
unique upload staging suffix (PID + timestamp + sequence). The previous upload
staging filename used only PID; cancellation after chmod444 could leave a path
that a retry could not write. Unique attempts avoid that reuse. Interrupted uploads
can still leave a small staging DEX in guest /data/local/tmp; no broad cleanup or
system-data deletion was introduced. Actual interruption during upload is not
claimed as natively tested.

Workspace 107 tests pass / 20 opt-in ignored; fmt, clippy, build and QA codesign
verification pass. After closing/reopening the updated QA app, both guests remained
running and fixture values were unchanged. Native down on left zone1 changed it
from V0 to V-10 (local x233/y282); native up on right zone3 changed it from V-10
to V0 (local x504/y916), leaving other zones unchanged. Touch counters remain zero.
QA bundle now contains all current scroll changes; main app remains unchanged.

## User trackpad confirmation and main bundle

User replied “Số H có thay đổi” to the explicit request to try horizontal two-finger
trackpad scrolling in the colored QA fixture. This is user-observed horizontal
runtime evidence, distinct from CUA's unsuccessful horizontal actions. It does not
prove both horizontal signs or every natural-scroll preference.

Ran fmt, workspace tests (107 pass / 20 opt-in ignored), clippy -D warnings,
build-hub-app.sh, and codesign --verify --deep --strict on main bundle successfully.
Previous main bundle retained at /tmp/android-hub-before-scroll-1794816166868400000.app.
Main Android Hub.app now includes framework scroll, bounded replies, atomic cache
and unique upload staging. Opened through CUA. Initial background screenshots showed
loading; after focusing the titlebar, full Android A/B/Android3 list and Sẵn sàng
were visible. A process sample collected during investigation does not establish
a persistent startup hang; do not report one from the early screenshots alone.
Personal guests stayed stopped; no personal guest input/data operations performed.
Main app is open; QA remains separate with two test guests running.
