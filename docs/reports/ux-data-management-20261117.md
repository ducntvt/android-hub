# US-042 — Data management presentation

Status: implemented (2026-11-17).

## Implemented

- Data controls are grouped separately from CPU/RAM. Backup, restore and archive retain stopped/dirty/stale/busy guards; storage measurement remains read-only.
- Archive/Lưu trữ replaces removal wording in desktop labels. The explanation says accounts/data remain and no disk space is freed; backend identifiers and persisted paths/schema stay unchanged.
- Backup discovery exposes source name, UTC creation time, metadata-declared logical size and a metadata-only verification warning. The unique backup name identifies the selected copy; full path is available through Details. Restore confirmation names the destination and warns that current data/configuration are replaced while old data is retained.
- Catalog errors, not loaded, loading and successful empty results are distinct. Retrying clears error/selection state. Broken symlinks and unreadable backup/archive roots cannot masquerade as missing/empty directories.
- Usage has typed managed/archive/backup/incomplete/pre-restore/log/other categories, measurement time and skipped count. Known registry IDs identify managed AVDs; unknown data is never called disposable. Full paths and APFS/shared-block caveats are behind Details. No deletion or automatic repair was added.
- Data UI is extracted to `app/ui/data.rs`; obsolete infrastructure Vietnamese-only BackupInfo display formatting was removed.

## Automated verification

Final `bash scripts/check.sh`: **181 Rust tests** (77 desktop + 4 domain + 100 infrastructure), **26 Python tests**, fmt, clippy and shell syntax passed; 20 existing ignored integration tests were not run. App build and strict codesign passed. Existing `block 0.1.6` future-incompatibility warning remains.

New evidence covers UTC leap-day/invalid values, catalog metadata with absent payload, invalid/missing manifests and broken-root symlink, failed versus empty lists/retry/stale response, registry-based categories, retained files and no-follow scanning. Existing restore/archive transaction tests still pass.

## Native fixture

`/tmp/android-hub-data-qa-20261117/Android Hub QA.app`, content size 1000×640, isolated data root, two stopped profiles and synthetic backup/archive/staging metadata. No Android engine, real account or personal runtime is used. The backup has no restorable payload intentionally; only list and confirmation/cancel are in scope.

Observed at the 1000×640 minimum content size:

- Empty backup result for profile B; replacing the fixture backup directory with a regular file produced an explicit read error instead of a successful empty list. Fixture directory was restored afterward.
- Profile A catalog showed one valid metadata entry and one skipped entry. Source name, UTC date, logical size and the metadata-only warning were visible in VI and EN. Details revealed the wrapped full path.
- Restore confirmation named QA Data A, the unique copy and replacement/retention semantics; Escape cancelled. Archive confirmation explained retained accounts/data and no reclaimed storage; clicking Cancel closed it. No transaction was executed.
- Archived device list, selected configuration and Details path were readable; VI/EN labels updated correctly.
- Storage showed seven categories, timestamp, skipped count and external SDK exclusion. Details exposed the runtime path, APFS/no-follow and incomplete-data limitations. Synthetic files are small, so GiB values round to 0.00; allocation/classification assertions are covered by automated fixtures.
- Initial backup picker combined date and size and clipped in the narrow panel. Changed the picker to a compact timestamp at 13 px; size remains in the selected description. Rebuilt and visually verified both dropdown and selected text fit without arrow overlap.

QA was quit through Cmd+Q, its Finder window was closed, and inventory plus an exact process-path/fixture-ID check showed no owned QA app/engine remained. The QA bundle/fixture, three owned logs and 2.8 GiB Cargo target were removed after verification. Main app, personal runtime, SDK and existing AVDs were preserved. No commit was made.

## Contract

See [ADR 0051](../decisions/0051-data-catalog-and-usage-presentation.md). Backup metadata size is logical, usage categories estimate allocated blocks, and neither indicates reclaimable APFS space. Grouping-directory overhead is omitted. Runtime can change while measuring; no atomic disk snapshot is claimed.
