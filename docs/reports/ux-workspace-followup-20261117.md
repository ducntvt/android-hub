# Workspace and modal input follow-up — 2026-11-17

Scope: native follow-up for US-033–036 using disposable guests, followed by one
production fix. This report supplements earlier reports; it does not certify
every acceptance criterion or change the four stories from `in_progress`.

## Fix and verification

Canceling a confirmation restored the parent's Android focus indicator but
reset the canvas's local keyboard focus when its input epoch changed. Keyboard
input therefore required an extra canvas click. `screen.rs` now preserves
parent-authorized focus across an epoch reset only for the same device and
display generation. Pending gestures still reset; reconnects do not inherit
local focus. A regression test covers restored key delivery and a subsequent
parent focus revocation.

The offline Android input fixture now reports A–Z key Down/Up counts, in addition
to its existing pointer/scroll counters. It has no permissions or network code.

Final verification passed: 189 Rust tests (20 existing ignored), 26 Python tests,
formatting, clippy with warnings denied, shell checks, app build and strict deep
codesign verification. The existing `block 0.1.6` future-compatibility warning
remains. The main `Android Hub.app` contains the fix.

## Native observations

- At 1000 × 640 logical content size, paired portrait/landscape screens fit with
  the sidebar and with the configuration panel. Sidebar collapse/restore keeps
  the pair and target. At 1180 × 820, single portrait, single landscape and the
  paired layout fit; a history panel leaves the landscape view fitted.
- Empty registry and unmatched search have different VI/EN messages. Filtering
  the sidebar does not discard the live display pair.
- Manual zoom at 110%/130% relative to Fit exposes scrolling as expected. Return
  to Fit removes workspace overflow. Scrolling outside Android moves the
  workspace; scrolling inside reaches the corresponding guest zone. Enlarged,
  scrolled canvas edge coordinates and a portrait edge tap reached the intended
  guest. Native app-drawer drags worked in portrait and landscape.
- The explicit pair remained landscape-left/portrait-right, independent of
  registry order, through polling, boot, panel changes and zoom. A third stopped
  profile with a long name replaced the active slot without replacing the other
  slot. Removing that disposable registry entry reconciled the slot to empty;
  this is external-registry reconciliation, not archive-flow acceptance.
- Choosing a stopped device in the empty slot displayed the correct targeted
  Start action. Booting it kept the other guest running. Header, selector and
  keyboard-focus indication agreed; letters sent to one guest did not change
  the other's key counters. Switching to single mode displayed the selected
  device; hidden engines continued their existing lifecycle.
- Actual boot feedback advanced through waiting/booting to Ready. An initial
  display connection timeout recovered automatically. This is startup retry
  evidence, not a controlled loss of an established display session.
- After the focus fix, opening Stop named the intended portrait guest and kept
  both canvas frames in place. Clicking behind the modal and typing Q delivered
  neither touch nor key events. Escape canceled; typing B immediately, without
  another canvas click, reached that same guest with balanced Down/Up counts.
- Creation receipts survived a QA restart and appeared in the separate history
  panel. Opening history cleared its unread marker. This does not demonstrate a
  crash-recovery receipt.
- Closing QA with two running guests stopped both and exited Hub. Process
  inspection confirmed no matching QA Hub or guest processes remained.

## Remaining acceptance gaps

- US-033: native drag-resize and a reliable continuous drag at manual zoom remain
  unverified. Launching at two sizes is not equivalent to testing live resize.
- US-034: combined EN/long-name presentation and pending-input behavior while
  switching slots still need a focused native check. Automated transitions and
  the observed routing above do not replace those checks.
- US-035: controlled established-display loss/recovery and visible per-device
  shutdown results, including partial failure, remain unverified natively.
- US-036: native toast expiry, error-copy behavior and interruption-recovery
  receipts remain incomplete; the running-guest modal focus case is now verified.
- Minor follow-ups: an empty left slot still refers to choosing the “second”
  device; original startup diagnostics/receipt bodies may remain Vietnamese in
  English UI. These observations are not recorded as fixed.

## Isolation and cleanup

QA used its own data root and blank Android 16/API 36 ARM64 Google Play guests.
No user account/game runtime was modified. CLI APK installation only prepared
the input fixture; it is not evidence for the Hub APK install flow.

Owned paths: `/tmp/android-hub-final-workspace-qa-20261117`,
`/tmp/android-hub-final-input-20261117`,
`/tmp/android-hub-final-key-input-20261117`, the corresponding final-workspace
check/build logs, and workspace `target/`. QA processes and the owned Finder
window were closed before removing these disposable artifacts. The main app,
personal runtime, SDK, AVDs outside the QA root and backups were preserved.
