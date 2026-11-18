# 0051 — Present data catalogs and storage without implying verification or reclaimability

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub

## Context

The backup catalog reads a manifest but does not verify payload checksums or restore compatibility. The previous disk-usage report grouped active AVDs and staging together, used Vietnamese category strings in infrastructure, and did not timestamp measurements. Failed catalog reads could resemble successful empty lists.

## Decision

Keep backup/restore/removal transactions and persisted schemas unchanged. Add the saved profile name to the in-memory backup metadata result. Desktop formats the source, unique backup directory name, UTC creation time, manifest-declared logical size and explicit metadata-only verification level. Full paths sit behind Details; restore confirmation retains the target name and replacement/retention explanation. Discovery never claims compatibility.

Return typed usage categories and measurement completion time. Read the validated registry to recognize managed `id.avd`/`id.ini` entries. Recognized `.removed-*` archives and `.previous-*` recovery data remain separate. Only `.restore-*` staging and hidden `*.partial` backup directories are grouped as incomplete operation data, never as safe-to-delete files. Unknown entries go to Other. Preserve bounded scanning, same-volume and symlink gates, and inode deduplication. Category totals omit the grouping directories themselves; APFS shared blocks are not reclaimable-space estimates. No cleanup action is added.

Desktop distinguishes not loaded, loading, empty success and failure, clears errors on retry and keeps the existing response epochs. Explicit Details controls reveal paths and measurement caveats. UI uses Archive/Lưu trữ while backend IDs, archive paths and JSON schema retain their names. UTC formatting uses the already-locked libc dependency and thread-safe `gmtime_r`; no timezone setting or new date-library dependency is introduced.

## Consequences

- Storage classification now fails if the registry cannot be read safely; it cannot silently call unknown AVDs managed devices.
- The display is a best-effort snapshot while guests may write, not an atomic disk inventory.
- Manifest metadata, catalog failures, stale responses, categories, no-follow scanning and UTC conversion need fixture tests.
- Native QA covers list/confirmation/cancel/labels; this presentation change does not require replaying destructive restore transactions.

## Alternatives considered

- Call every AVD/staging directory a managed device: misrepresents unregistered or recovery data.
- Verify every backup on discovery: potentially reads many GiB and blocks a simple list operation.
- Rename persisted archive paths with the UI term: unnecessary migration risk.

## References

- [US-042](../stories/US-042-device-data-management-presentation.md)
