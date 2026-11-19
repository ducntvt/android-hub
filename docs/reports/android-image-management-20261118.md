# Android image management QA

US-052 adds on-demand image storage/reference inventory and explicit removal in
SDK setup. Versions 12–16 remain separate from downloaded archive cache. A stopped
device still protects its image; deleting a device does not automatically remove it.

## Contract and checks

Removal is limited to the current Hub root's managed SDK directory, matching SDK
receipt and image ownership. External SDKs are visible without a removal action.
Preview verifies the image identity and captures directory device/inode. Confirmation
rechecks ownership, identity, paths and references under SDK-image then Hub locks.
Registry, actual AVDs, archive receipts, backup manifests and pending journals are
included; unresolved or corrupt metadata blocks removal.

A durable journal precedes the rename out of the package path. Partial deletion is
never considered an installed image. Explicit retry rechecks the retained directory
identity and references before continuing; startup does not resume deletion. A
bounded unpublished journal stage can be discarded under the mutation locks because
no image move can have happened before publication. Historical optional-image SDK
receipt entries may be absent after removal; common tools remain cataloged.

Fixture tests cover unused owned images, external/unowned SDK images, replacement
payloads, symlink path escapes, stopped-device references and AVDs left outside the
registry, backups, archives without live AVDs, pending/unknown journals, unpublished
journal stages and partial deletion retries. SDK/Hub lock contention excludes a
competing install/create mutation. A stale removal preview is rejected after a new
reference appears. Download cache and other images survive removal. Managed SDK
catalog tests verify that removing its bundled optional image preserves common tools.

Desktop tests cover explicit confirmation, initial Cancel focus, Escape, stale and
duplicate completions, double-submit and failure/refresh. Creating a device also
refreshes the open SDK image inventory; the creation review is not duplicated in the
background SDK panel.

## Native QA

The app ran at 1000×640 on macOS Apple Silicon. Disposable data was under
`/tmp/android-hub-e01-qa-20261118/management/data`. Common installed tools were APFS
cloned into a separate directory. Its ownership receipt was explicitly labeled a
QA fixture; this is not evidence of a fresh common-SDK download or its provenance.
The Android 13 image itself used the production, consent-gated installer and the
previously verified official archive. The user's approved license hash matched.

Observed results:

- English SDK setup showed Android 13/API 33, revision 9.0.0 and about 3.31 GiB
  allocated after installation.
- Remove opened a centered confirmation with version and size. Cancel had initial
  focus; Escape closed it without removing the image. Tab focused Remove.
- Enter confirmed removal. The list changed to Not installed, the package and
  removal journal were absent, and the downloaded archive remained intact.
- Download and create reused the verified archive and created **QA Protected 13**.
  Refresh showed the device reference and Image is in use, with no Remove button.
  The device remained stopped throughout this test.
- Vietnamese showed the translated status, size and device reference. The same
  protection applies regardless of language.
- Native app inventory confirmed the QA process exited after Quit. No QA guest was
  started in this management session.

Native testing found a stale image list after download-and-create from the SDK
panel. Creation completion now refreshes that inventory; a state test verifies it.
A duplicate background review was also removed. These follow-up presentation fixes
do not change the verified removal transaction.

## Validation and limits

The full check script passed: 93 desktop, 5 domain and 138 infrastructure tests,
2 metrics tests, 32 Python tests, formatting, Clippy and documentation checks.
Twenty-five opt-in integration tests remain separate. Bundle build and strict
codesign verification passed. Final cleanup is recorded with the story evidence.

Allocated blocks are a measurement, not a promise of exactly reclaimed disk space
on APFS. Inventory covers the owning Hub data root; manually sharing its private SDK
with other independent roots/tools is outside the supported lifecycle contract.
Five-version guest qualification and game compatibility are separate US-053 work.

Final epic cleanup: the retained QA SDK/AVD inputs were subsequently removed; see
[US-053 qualification and cleanup](android-versions-qualification-20261118.md).
