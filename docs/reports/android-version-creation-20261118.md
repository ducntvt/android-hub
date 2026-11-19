# Android version selection at creation

US-051 adds Android 12–16 to the new-device form, defaulting to Android 16.
The picker selects a closed domain version, not an arbitrary package/path. Existing
settings expose version/revision read-only and explain that another Android version
requires a new independent device. Sidebar labels do not rename custom devices.

## State and backend contract

Creation readiness distinguishes common SDK/Java prerequisites, an absent image,
an invalid installed image and insufficient space. Changing version invalidates the
previous asynchronous probe. A valid installed image allows Create; an absent image
allows Download and create only when the other prerequisites are ready.

Review keeps the entire request and start-after choice. Consent remains scoped to
the reviewed plan. During download the target is frozen. Completion is tagged with
the command epoch; stale/duplicate results cannot submit creation. Success passes
the verified image identity to the backend, which rechecks that identity under the
Hub lock before creating any AVD or journal. Existing creation transaction/recovery
and created-but-refresh/start-failed behavior remain in use.

Canceling a download disables follow-on creation even if publication wins the
cancellation race. The installed image can be reused, but no device is claimed to
have been created. Failure preserves the form and explicit retry; Back clears
review consent and probes prerequisites again. Quit does not start a new creation.

## Automated checks

The full check script passed with 92 desktop, 5 domain and 130 infrastructure tests,
2 metrics tests and 32 Python tests, formatting, Clippy and docs. Native bundle
build and strict codesign verification passed. Twenty-five opt-in integration tests
remain separate from CI/shared checks.

New tests cover version changes/stale probes, preserving custom fields through
review, consent, frozen targets, double-submit, stale/duplicate install completion,
failed download/retry, late cancel after publication and image changes before AVD
creation. Existing tests cover retained/unknown creation failure and committed
creation followed by refresh/start failure.

## Native QA evidence

QA uses `/tmp/android-hub-e01-qa-20261118`, shared Emulator 37.2.12 and a 1000×640
window. The initial long form hid its footer; changing the scroll area's allocation
keeps Cancel/Create visible while the content scrolls. The corrected footer was
observed in both English and Vietnamese.

- English: selected Android 13 from the installed image and created **QA Android 13**.
  Registry bound API 33 revision 9.0.0 with the expected fingerprint. The sidebar
  and workspace showed Android 13. Reopening Hub retained the new device.
- Vietnamese: selected Android 14, observed the 1.55 GB download requirement, entered
  review and returned. Name, version and resource settings were preserved.
- The first native download attempt was blocked because the temporary QA launcher
  passed a symlink as the data root. The QA-only launcher was corrected to pass the
  existing physical data directory; production reference checks were not weakened.
- Android 14 download-and-create completed with API 34 revision 14.0.0 and fingerprint
  `c5fdd150054f798b4a0479271103b852a758ac94f6b75943fb49fdb13bba2ffd`. Exactly one
  device appeared. Settings showed the version read-only; reopening Hub retained it.
- Tab focused the version selector; Up changed Android 16 to 15. Escape canceled
  the form without creating another device. Both new devices remained stopped.
- Quit closed the QA process, confirmed by native app inventory. The isolated SDK
  and four device fixtures are retained only for the remaining epic qualification.

No personal Google account, production SDK or production device data is used.

Final epic cleanup: the retained QA SDK/AVD inputs were subsequently removed; see
[US-053 qualification and cleanup](android-versions-qualification-20261118.md).
