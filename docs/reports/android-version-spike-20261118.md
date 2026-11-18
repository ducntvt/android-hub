# Android 12 and 16 compatibility spike

US-048 establishes a feasible shared engine for E01. It does not qualify Android
13–15, game compatibility or the future version-selection/install UI.

## Environment and provenance

Tested 2026-11-18 on macOS 27.0.1, Apple M2, Emulator 37.2.12 (16428233),
Android Studio OpenJDK 25.0.3. Both isolated guests used Pixel 5, 2 CPUs,
2048 MiB RAM, 720×1280, host GPU and a 30 FPS stream limit. The embedded stream
scales to at most 854 pixels. No personal Google accounts were used.

| Android | Package | Revision | Published archive SHA-1 | Minimum emulator |
| --- | --- | --- | --- | --- |
| 12 | `system-images;android-31;google_apis_playstore;arm64-v8a` | 9 | `8a1fa9a050a3894fb65c785d6b536e182be30e7f` | 31.2.7 |
| 16 | `system-images;android-36;google_apis_playstore;arm64-v8a` | 7 | `5b91ef80a5eb60d7b7167528db259c2a12d20e9c` | 35.4.9 |

Metadata came from the [official Google Play catalog](https://dl.google.com/android/repository/sys-img/google_apis_playstore/sys-img2-3.xml).
API 31 was downloaded to a dedicated QA SDK after explicit license consent;
size and published checksum matched. Computed archive SHA-256:
`9f4dd2b05076cddd1cca855e66d691e89aab9672b008b4f1c136c8e0e9748ceb`.
API 36 reused the existing revision-7 image read-only through the QA SDK;
its original archive checksum was not reverified by downloading it again.

## Results

| Case | Android 12 | Android 16 | Evidence and boundary |
| --- | --- | --- | --- |
| Boot | Pass | Pass | Backend boot observation, separate AVDs and ports 5650/5652 |
| Embedded display | Pass | Pass | Authenticated RGBA frames; both visible in native two-device layout |
| Touch and drag | Pass | Pass | Backend and native UI: Down=1, Up=1, Moves=1, Held=false |
| Horizontal and vertical wheel | Pass | Pass | Real ScrollBridge: fixture H=1.00, V=1.00 after separate axis events |
| Native vertical wheel | Pass | Pass | Visible fixture V=-20.00 / -10.00 in the targeted zone |
| Native horizontal gesture | Tool limitation | Tool limitation | Native automation did not generate an H change; backend proof above does not certify a physical trackpad |
| Native keyboard | Pass | Pass | KEYCODE_A: Down=1, Up=1 |
| Rotation | Pass, bounded | Pass, bounded | Sensor request changed stream from (480,854,0) to (854,480,1); not a four-orientation app-layout certification |
| Unicode clipboard | Pass | Pass | Session ACK and foreground fixture paste matched `Hub QA tiếng Việt` |
| APK install | Pass | Pass | Backend installed and launched the input fixture |
| Reconnect | Pass | Pass | New authenticated session received frame/input after dropping old session |
| Connectivity | Pass | Pass | Guest probe returned INTERFACE/DNS/HTTPS OK for its configured endpoint |
| Quit cleanup | Pass | Pass | QA quit stopped both registered guests; held runners observed Guest stopped |

Commands used the checked-in `device-qa` example and observable Android fixture;
see [fixture instructions](../../tests/android-input/README.md). Native automation
tested only the disposable QA app, not the user's main data.

## Compatibility changes

ScrollBridge previously required `InputManagerGlobal`, absent on older Android.
It now falls back to `InputManager` only when the global class is absent, and
retains explicit failure for injection/reflection errors. The tagged AOSP sources
expose the required `getInstance` and `injectInputEvent` methods:
[Android 12 InputManager](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-12.0.0_r1/core/java/android/hardware/input/InputManager.java),
[Android 13 InputManager](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-13.0.0_r1/core/java/android/hardware/input/InputManager.java),
[Android 14 InputManagerGlobal](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-14.0.0_r1/core/java/android/hardware/input/InputManagerGlobal.java).
ConnectivityProbe uses standard Java networking APIs rather than these hidden
input APIs. Both DEX helpers and the fixture now target min API 31; the actual
guest tests above validate that lowering the build floor alone was insufficient.

The first API-31 launch exposed SDK leakage from inherited `ANDROID_HOME`.
Launchers and avdmanager now set both Android SDK variables to the selected SDK.
A regression test executes the generated launcher against a fake executable,
including spaces/apostrophes in its SDK path and conflicting inherited variables.

## Verification and retained inputs

`scripts/check.sh` passed after the fixes: formatting, Clippy, 208 app tests,
2 metrics tests, 31 Python tests and documentation checks. The 22 explicitly
ignored integration tests were not bulk-run. App build and strict deep codesign
verification passed.

The QA app and guests were closed after the session. The disposable app bundle
and obsolete fixture build were removed. The stopped QA SDK/AVDs, current fixture
and bounded logs under `/tmp/android-hub-e01-qa-20261118` are retained only as
inputs for the remaining E01 stories; remove them at the final epic handoff.
No SDK engine upgrade or mutation of personal AVD data was performed.
