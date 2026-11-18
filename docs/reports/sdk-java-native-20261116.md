# Native SDK/Java smoke — 2026-11-16

Only the isolated `/tmp/android-hub-ui-qa-20261116/Android Hub QA.app` and its QA data root were operated. No personal guest or main Hub preference was changed.

## Observed through native UI

- With Android 3 running, SDK/Java save controls were disabled and the panel instructed stopping devices first.
- Stopped Android 3 through Hub and its confirmation. All three QA profiles then displayed stopped.
- Entered `/tmp/android-hub-missing-sdk-qa` and clicked save: rejected with `No such file or directory (os error 2)`.
- Restored `/Users/ducnt/Library/Android/sdk` and saved: ready/saved confirmation appeared.
- Entered `/tmp/android-hub-missing-java-qa` and clicked save: rejected with the same filesystem error.
- Restored `/Applications/Android Studio.app/Contents/jbr/Contents/Home` and saved: Java saved confirmation appeared.
- Quit QA using Cmd+Q, reopened the QA bundle and opened SDK settings. The SDK and Java fields retained the valid paths; status was ready. The wrapper unsets both environment overrides, so this launch exercises saved preferences.
- Startup history also displayed Android 3's previous successful restore. This verifies the successful receipt display, not failure/recovery banners.

## Limits and follow-up

- Long Java path exceeds the field's visible width; no claim that its whole value is simultaneously visible. The on-disk QA settings were read separately to verify the full saved value.
- Missing-path errors are raw OS text and could be made more informative.
- Empty-registry SDK selection and switching to another SDK were not tested here. Stories remain in progress pending their complete verification requirements.
- APK fixture build completed successfully at `/tmp/hub-apk-ui-qa-20261116/test.apk`. Native cross-window Finder-to-Hub drop was not completed; no APK install/cancel UI success is claimed.
- Final state: QA app open, all QA guests stopped, valid SDK/Java preferences restored.
