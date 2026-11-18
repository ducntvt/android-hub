# UX workspace implementation — 2026-11-17

## Scope

Two agents analyzed the review: workspace/selection and management/feedback flows.
Their output is [UX roadmap](../UX-ROADMAP.md) and US-033 through US-042.
US-033/034 have initial implementation; US-035–042 remain planned.

Implemented in desktop only: responsive aspect-preserving Fit, zoom relative to
Fit, session-only sidebar collapse, explicit ordered device slots including
stopped/empty placeholders, selected mode buttons, input-focus border/label,
metadata-only selection after pointer-down, named stop action/confirmation,
new VI/EN labels, and distinct empty/search states. No stream rate, engine,
registry schema or guest data changes.

Prevent navigation/start from silently discarding a dirty draft. Same-device
selection preserves edits while still invalidating stale backup catalogs.
Authoritative refresh after restore and Save reloads persisted configuration.
Full Save/Discard/Cancel UX is US-037, not claimed here.

App packaging now stages and atomically replaces its executable. The existing
user process can continue using its old binary; the new build takes effect on
next launch. No QA app/guest was opened in this task, and no user process was
stopped or restarted.

## Verification

- `PYTHONDONTWRITEBYTECODE=1 bash scripts/check.sh`: exit 0.
- Rust: 129 passed (desktop 38, domain 2, infrastructure 89); 20 existing opt-in integration tests ignored.
- Python: 25 passed; shell syntax, formatting and clippy with `-D warnings` passed.
- `bash scripts/build-android-hub.sh`: exit 0.
- `codesign --verify --deep --strict 'Android Hub.app'`: exit 0.
- Existing dependency warning: block 0.1.6 future incompatibility, unchanged.
- Tests include three stopped profiles, pair ordering/duplicates/removal,
  mode/sidebar transitions, geometry across viewport/panel/orientation sizes,
  preserved gesture epoch, stale input, dirty draft/start guards, restored draft
  refresh and the existing canvas coordinate/drag regression suite.
- Cross-review corrected the control-height reserve, container centering that
  overwrote fixed dimensions, and same-device draft preservation. Existing
  catalog regression caught and fixed missing invalidation on reselection.

## Remaining acceptance

Native UI smoke was not run against this new build. Actual rendering at small
windows/long labels, live two-device gesture behavior, portrait/landscape and
VI/EN screenshots remain unchecked. Geometry and state tests do not establish
native visual acceptance or FPS improvement. US-033/034 remain in_progress.
The current user Hub and Android session were preserved; they still use the
previous executable until the user closes and reopens Hub.

Zoom, pair and sidebar visibility are not persisted across launches. Generic
busy locking, inline history/confirmations and full theme redesign remain in
later stories. Input cannot switch to another device during a serialized
mutation. These are explicit limits, not completed review items.

## Cleanup

Disposable `target/` and task logs under `/tmp/android-hub-ux-*20261117.log`
were removed after verification. Main Android Hub.app, runtime, SDK, AVDs,
backups and migration receipts are retained. No git commit.
