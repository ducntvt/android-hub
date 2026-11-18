# Feedback acceptance follow-up — 2026-11-17

US-036 is implemented. US-033–035 retain their own acceptance gaps.

## Native evidence

Isolated QA: `/tmp/android-hub-feedback-followup-20261117`, 1000 × 640 content
window, two stopped registry profiles, no Android engine or personal data. QA
used the preceding main binary, with identical feedback/modal code. The only
production change in this follow-up is empty-slot wording.

- **Startup recovery:** prepared a synthetic restore journal at post-OldMoved,
  with distinct original/staged directories and their real filesystem identities.
  Opening QA ran normal backend recovery. Read-only assertions confirmed the
  original marker returned to the live directory, staged/previous directories
  disappeared, the pending journal became a rolled-back receipt, and the original
  profile remained in the registry. `History · New updates` remained until opened.
  History displayed the recovered-after-interruption message in VI and EN;
  opening it cleared the unread mark. This verifies startup recovery presentation,
  not bootability of a restored real Android payload.
- **Toast:** saving configuration showed success, correct device name and the
  next-start explanation above the footer. A later observation without dismissal
  showed it gone, with device/stage geometry unchanged. The production timer is
  six seconds; observation verifies expiry, not millisecond timing.
- **Error/Details:** backing up the deliberately incomplete fixture failed on its
  absent config. The translated alert survived polling and closing Details. The
  raw diagnostic/path wrapped into two lines in a compact modal; underlying
  geometry stayed fixed. VI/EN labels and explicit dismissal were observed.
- **Copy error:** clicked Copy error in Details and pasted through the normal
  macOS clipboard into a new TextEdit document. Its accessibility value was:

  ```text
  Android Hub 0.1.0
  Cần xử lý: Không sao lưu được dữ liệu
  Bản tóm tắt không chứa đường dẫn hoặc nội dung Android.
  ```

  No device name, raw diagnostic, path or Android content was copied.
- **US-034 evidence:** the 45-character EN device name was readable in sidebar,
  wrapped toolbar heading, single-device heading and paired selector. Selection
  and pair agreed. This does not establish live slot-switch input release.

TextEdit launch initially timed out; inventory confirmed it had opened, and
rebinding the existing app completed the check. Its QA document was saved only
under `/tmp`. A remaining window-less TextEdit process was terminated after
checking the exact owned PID/path. Hub QA exited normally; final inventory showed
both closed. No guest engine was started.

## US-036 completion audit

| Contract | Evidence |
|---|---|
| Toast outside layout; persistent failures | Native save/error/expiry above; matching-ID expiry, poll preservation and deduplication tests |
| Consistent stop/APK/restore/archive overlay | Shared stack in `ui/feedback.rs`; original feedback report, US-039 install/cancel, US-042 restore/archive dialogs, running-guest follow-up |
| Named destination; identity snapshot | Native named dialogs; replacement-PID stop, stale APK selection/epoch and restore/archive identity regression tests |
| Block/release Android input; restore focus; safe Enter/Esc | Running-guest keyboard/touch follow-up; original Tab/Enter/Esc checks; modal gate and canvas epoch regression tests |
| Typed feedback, Details and safe copy | Actual clipboard above; Unknown APK result and VI/EN allowlisted-summary tests |
| Separate history; persistent recovery receipt | Normal initialize/recovery on isolated journal above; retained-history and stale-receipt tests |
| VI/EN presentation; original diagnostics in Details | Native VI/EN alert/recovery text and original diagnostic. Historical creation/archive backend strings remain operation details, not translated status labels. |

References: [initial feedback](ux-feedback-20261117.md),
[running-guest focus fix](ux-workspace-followup-20261117.md),
[APK/clipboard](ux-apk-clipboard-20261117.md),
[restore/archive presentation](ux-data-management-20261117.md).

## Change, checks and cleanup

Empty slots now say “Chọn một máy để hiển thị tại đây.” / “Choose a device to
display here.”, valid for either slot. Verified in source/build; the earlier QA
binary did not contain this wording-only change.

Final `scripts/check.sh`: 189 Rust and 26 Python passed, formatting/clippy/shell
checks passed; 20 existing integration tests remain ignored. App build and strict
deep codesign passed. Existing `block 0.1.6` future-compatibility warning remains.
Main app rebuilt; no commit.

Owned disposable artifacts removed after closing QA: the QA root above,
workspace `target/`, `/tmp/android-hub-feedback-clipboard-20261117.rtf`, and
`/tmp/android-hub-feedback-followup-{check,build}-20261117.log`. Main app, personal
runtime, SDK, existing Android data and backups are preserved.
