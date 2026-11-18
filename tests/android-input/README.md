# Offline input QA fixture

Build with `bash scripts/build-input-qa-apk.sh /tmp/new-input-qa-directory`.
The output folder must not exist. This uses installed Android SDK 35/build tools
35.0.1 and Android Studio Java; minSdk/min-api 31 for Android 12 and later.
No Gradle download, permissions or network code.
Install only on an identified disposable QA guest, then launch **Hub Input QA**
from the Android app drawer. Installation via adb is fixture preparation and does
not count as verification of Hub's APK drag/drop interface.

The three colored zones independently accumulate horizontal/vertical ACTION_SCROLL
axis values. The header displays the most recent local view coordinates and axes.
Rows move according to the respective zone's accumulated axes. Touch down has a
separate counter; wheel must not increment it. This custom View is an input probe,
not proof of every Android widget's scrolling behavior.

Alphabetic keys A–Z update the event label with the Android key code and separate
Down/Up counts. The view requests keyboard focus on launch and touch; other keys
retain normal Android handling. This probe can distinguish a visible Hub focus
indicator from actual keyboard delivery to the intended guest.

Native checks:

1. Scroll over zone 3; only zone 3 changes, with matching local pointer coordinates.
2. Scroll over zone 1; zone 2/3 retain their values.
3. Scroll left/right; only the target zone's H value changes.
4. Scroll outside the Android canvas; no guest counters change.
5. Repeat in two simultaneously visible guests; the other guest stays unchanged.
6. Tap/drag then wheel; touch and wheel remain distinct.
7. Reconnect the display session; no old wheel event replays.
8. Type a letter in each guest; only the focused guest's key counts change.
9. Open a Hub confirmation, click/type behind it, then cancel with Escape. No
   touch/key events should reach the guest while the modal is open. Type a
   different letter without clicking the canvas again; the previously focused
   guest receives it with balanced Down/Up counts.

Use native UI tooling for test events. Do not inject directly through adb/shell,
which would bypass the host event routing being tested. Coordinates exclude the
Android status bar inset; derive expectations from the actual displayed canvas.

## Backend compatibility probe

`crates/infrastructure/examples/device-qa.rs` exercises Hub's real display/input,
APK and connectivity backend against this fixture on an already prepared guest.
It requires an explicit `ANDROID_HUB_ROOT`, `ANDROID_SDK_ROOT`, and an
`allow-input-probe` marker file inside the disposable data root. It never creates
AVDs or targets a guest outside that root's registry. It never accepts licenses
automatically. The `install-image RELEASE ACCEPTED_LICENSE_SHA256` mode requires
prior user consent and an SDK located inside the same isolated QA parent directory;
it invokes the production plan/installer without changing the selected SDK.

```bash
# After creating the isolated QA root/SDK/AVD and marker:
cargo run --locked -p android-hub-infrastructure --example device-qa -- start PROFILE_ID
cargo run --locked -p android-hub-infrastructure --example device-qa -- probe PROFILE_ID /absolute/path/input-qa.apk
cargo run --locked -p android-hub-infrastructure --example device-qa -- stop PROFILE_ID
```

Run `start` in a separate terminal; it stays attached until the guest stops.
The probe installs the fixture if absent, restarts its activity to reset counters,
checks input delivery through Session, compares Unicode clipboard in the fixture,
observes rotation/reconnect, and runs the guest network probe. Rotation can change
orientation; the guest stays running for native UI QA until explicitly stopped.
Always stop a guest after a failed probe too. A preinstalled fixture must be the
current build; the probe intentionally does not replace an existing APK.

For two already booted QA guests with the fixture installed, append the other
profile ID: `probe PROFILE_ID FIXTURE_APK OTHER_PROFILE_ID`. The probe seeds the
other guest with a distinct clipboard sentinel and reset fixture counters, then
checks that they remain unchanged after the selected guest's input/clipboard
checks. Run both directions. APK destination isolation is a separate check: start
with the fixture absent on both, install on one through Hub, verify it remains
absent on the other, then install on the second. Do not reset personal applications.

The view exposes only counter summaries through its accessibility description.
KEYCODE_PASTE checks whether the foreground fixture reads the known string
`Hub QA tiếng Việt`; it reports matched/mismatch and does not expose clipboard
contents. Backend probe results do not establish host UI event routing, drag/drop,
or navigation-button behavior; run the native checks above separately.

`play-store PROFILE_ID` checks the package and opens its launch activity without
signing in. `backup-restore PROFILE_ID` requires a stopped guest, verifies the
backup and restores through production gates, asserting unchanged profile/image
identity. Boot the restored guest and verify the fixture before removing the QA
backup/previous-data directories printed by that command. These commands operate
only on disposable data; they are not user-data maintenance shortcuts.
