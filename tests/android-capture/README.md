# Physical capture QA

Offline, permission-free fixture for US-059. Reuses the disposable
`local.androidhub.inputqa` package; never install over an existing user-owned copy.
Build with `bash scripts/build-input-qa-apk.sh <new-output-dir> tests/android-capture`.

On an explicitly authorized physical transport, install the APK and launch
`.InputActivity` with boolean extras `landscape` and/or `secure`. Use `am force-stop`
before a new launch so extras apply. Default capture contains only synthetic text
and a moving rectangle, bounded to 120 seconds. `secure=true` sets Android's
FLAG_SECURE: the fixture's content must be black/absent in Hub, never bypassed.

Check portrait/landscape aspect, Fit/zoom, stream stop/start, 30/60 FPS options,
and secure content. No production input functionality is exercised. Uninstall
only this fixture after QA and remove its disposable build directory.
