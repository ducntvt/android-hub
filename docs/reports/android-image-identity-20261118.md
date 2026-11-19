# Immutable Android image identity verification

US-049 implements version-bound profiles and metadata-only legacy migration.
Android 12–16 image choices are represented by a closed domain enum. The reviewed
catalog records exact package, revision, archive checksum, license and minimum
engine; `prepare-sdk-manifest.py --images-only --image XML --output JSON` reproduces
it from saved official metadata without downloading or accepting licenses.

## Automated results

`bash scripts/check.sh` passed: 88 desktop, 5 domain, 123 infrastructure tests,
2 metrics tests, 32 Python tests, formatting, Clippy and docs. The 22 opt-in
integration tests remain ignored. Binary and diagnostic example builds passed.

The added fixtures verify:

- Exact five-version catalog, package restrictions and rejection of tampered licenses/destinations.
- SDK metadata/AVD disagreement, missing optional images, insufficient engine and changed image bytes.
- Legacy backup preservation, retry after interrupted staging, idempotent migration,
  per-device unresolved metadata and retry after the missing image becomes available.
- Backend rejection of changing or clearing an image binding and of unknown registry schemas.
- Archive/undo retains the exact binding. Compatible legacy backup gains the
  verified binding; different revision/binding or changed SDK fingerprint fails
  before any userdata or registry write.

## Isolated native smoke

The existing stopped E01 QA guests were migrated and the QA app reopened:

| Device | Before | After reopen | Native settings label |
| --- | --- | --- | --- |
| QA Android 12 | Legacy profile, API 31 AVD | Android12, revision [9,0,0] | Android 12 · API 31 · r9.0.0 |
| QA Android 16 | Legacy profile, API 36 AVD | Android16, revision [7,0,0] | Android 16 · API 36 · r7.0.0 |

Names, RAM 2048 MiB, CPUs 2, resolution 720×1280 and FPS 30 stayed unchanged.
Seven config/userdata files retained size, inode and nanosecond modification time;
config SHA-256s also matched. The registry's original bytes were preserved before
schema-1 publication. QA was quit and process exit confirmed; no guest was started
for this metadata smoke. The temporary `ui-identity` app bundle was removed.

The isolated SDK/AVDs remain inputs for US-050–053. The main data root was not
migrated by this test. On-demand installation and the creation picker are still
separate, unfinished work; this report does not claim five-version game support.
