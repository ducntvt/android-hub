# 0056 — Bind each device to its creation-time Android image

- **Status**: Accepted
- **Date**: 2026-11-18
- **Deciders**: Android Hub maintainer; product scope confirmed by the user
- **Supersedes**: none; extends creation and backup contracts after implementation

## Context

Hub currently creates Google Play ARM64 API 36 devices. The user requested Android
12–16 with the version selected only when creating a new device; existing devices
must not switch versions. Image selection currently appears as literals across
creation, SDK readiness, install plans and UI. Profiles do not record the selection.

## Decision

Each device retains its creation-time Android version; another version requires a
new independent device. US-048 verified a shared Emulator 37.2.12 and helper
strategy on API 31/36. US-049 implements the catalog and immutable binding below;
US-050 adds the on-demand installer and US-051 the creation picker. Image removal
and five-version qualification remain separate stories.

Registry schema 1 is `{"schema":1,"profiles":[...]}`. Each profile has nullable
`image`: `{"version":"12","revision":[9,0,0],"fingerprint":"<sha256>"}`.
Version is a closed domain enum mapping to the exact API/Google Play/ARM64 package;
it is not a user-supplied path. Fingerprint hashes a deterministic inventory of
the six image files already protected by the backup contract. Existing installed
stable revisions can be bound from verified SDK metadata; catalog download
revisions never replace an existing binding. `null` explicitly means unresolved.

The fingerprint normalizes SDK-relative path components so avdmanager's optional
trailing slash cannot change identity. Backup manifests retain their original path
spelling for existing restore comparisons; payload/revision checks are unchanged.

Legacy arrays remain readable. Initialization resolves pending transactions first,
then enriches only unbound profiles from matching AVD target/path/ABI/flavor and
validated SDK revision/dependencies/payload. A byte-exact
`hub-profiles.pre-image-schema.json` is atomically preserved before replacing a
legacy registry, including when transaction recovery is the first writer.
Registry publication uses fsync and rename under the Hub lock. Interrupted staging
is discarded in favor of the last published file; a conflicting preserved backup
fails closed. Missing/conflicting device metadata stays unresolved and is retried
on a later initialization without blocking other profiles. Userdata and AVD
configuration are never rewritten by migration.

Start checks the bound package, revision and fingerprint; settings cannot change
or clear the binding. Hashes are cached under `image-fingerprints/`, keyed by
canonical image location and invalidated by device/inode/size/mtime/ctime (including
nanoseconds) of every protected file. Metadata is revalidated on every gate;
snapshot polling never hashes image files. The cache is disposable and concerns
ordinary external SDK edits, not an adversary able to modify Hub metadata/cache.
Backup and restore retain full payload/image checks; compatible legacy schema-2
backups inherit the current binding only after all original path/fingerprint gates.

- Keep Android 16 as the new-device default. Initially offer APIs 31, 33, 34, 35, 36.
- Reuse compatible common SDK tools; install optional images on demand. No separate
  full SDK per Android version and no new network service are required.
- Validate choice in the backend; settings updates cannot mutate image identity.
- Enrich legacy metadata only from verified AVD/SDK facts using recoverable writes;
  unresolved identity remains explicit. Never infer API 36 solely from old schema.
- Preserve existing backup fingerprint/path checks. Restore does not convert Android.
- Do not silently upgrade image revision or engine. Mismatches caused by external
  SDK edits are reported and affected operations blocked rather than auto-repaired.
- Before deleting a Hub-owned image, inspect device/archive/backup/pending-operation
  references under a lock. External SDK image deletion is outside Hub ownership.
- Treat catalog availability and tested Hub compatibility as different states.

## Image installation

Common tools and single-image plans share the existing verified downloader, ZIP
extractor and cancellation controls. Consent is scoped to the reviewed plan. An
SDK-scoped image lock serializes installs across Hub data roots; short Hub locks
protect admission/publication without holding guest operations during downloads.
Before download, validate selected SDK, engine, references and disk estimates;
recheck engine and references before publication. SDK-local private staging keeps
rename on one filesystem. Verify payload and write provenance `.hub-image.json`
inside it before atomic `RENAME_EXCL` publication. A crash leaves private staging
or a complete, verifiable package, never an intentionally partial destination.
Existing valid packages are reused without claiming ownership or changing revision.
A missing referenced image requires restoring its original identity, not installing
the latest catalog revision. An invalid existing path is never overwritten.

## Download and create

The creation editor keeps its request while reviewing an absent image. Async
readiness is invalidated on version change; install completion carries a command
epoch, and the target cannot change while downloading. Only a matching successful
completion with an active create intent proceeds. Backend creation checks the
returned image identity under the Hub lock before AVD/journal writes. A late
cancellation can keep an already published image but cannot create a device. Once
registered, the existing result contract keeps the explicit created ID even if
refresh or optional start fails; those failures never reopen a retryable create.

## Image removal

US-052 scopes removal to a real SDK directory immediately under the current Hub
root's `managed-sdks`, with a matching ready SDK receipt and image provenance.
Images in external SDKs remain visible but cannot be removed through Hub, even
when the optional installer originally downloaded them. Installed images and
archive download cache are independent; neither device deletion nor cache cleanup
implicitly removes an image.

On-demand enumeration reports allocated blocks and conservative references from
registry, retained AVDs, archive receipts, backup manifests and pending journals.
Unreadable or unresolved references block removal. Review hashes the image and
captures its directory identity. Confirmation rechecks ownership, payload identity,
references and paths while holding the SDK image lock then the Hub lock. Device
creation, backup and restore already use the Hub lock, so they cannot enter the
same data root during this mutation.

A durable per-version journal precedes renaming the image to a private sibling.
Deletion occurs outside the package path. Interrupted removal is shown explicitly,
blocks install/start readiness and requires another confirmed retry; restart never
automatically resumes destruction. Retried removal validates SDK, references and
directory inode/device again, including partially deleted payloads. Unrecognized
paths or replacements fail closed. The managed SDK's initial receipt is historical;
its optional image entries may now be absent without invalidating common tools.

This inventory covers Hub-managed data in the owning root. Manual external tools
or separately copied Hub roots are not registered consumers of its private SDK;
sharing that SDK outside its owning root is not a supported lifecycle contract.

## Consequences

- Independent Android versions can coexist while device data stays isolated.
- Schema compatibility, dependency validation and reference tracking are required.
- Older Android versions need helper compatibility work; boot alone is insufficient.
- Users create a new device and sign in/install apps again to use another version.
- Automatic image revision updates, simultaneous revisions of the same package and
  in-place OS upgrades remain outside this implementation; they need a later design.
- API 31/36 feasibility is checked first. If no shared engine/helper strategy works,
  revise this proposal explicitly before widening the implementation.

## Alternatives considered

### Mutable Android selector on existing devices

Rejected by the user's explicit requirement; changing a system image is not a
safe general migration for app/user data.

### Hardcoded branch per Android version

Rejected because package validation, readiness, labels and creation would drift.
Use catalog-backed data and small capability-specific helper paths instead.

### A complete SDK for every version

Not selected for the initial design: it duplicates common tools/storage and expands
engine lifecycle management. Reconsider only if the compatibility spike proves a
shared engine cannot satisfy the selected versions.

## References

- [Epic E01](../stories/epics/E01-multiple-android-versions/README.md)
- [Create request contract](0047-create-request-and-registered-result.md)
- [Current architecture](../ARCHITECTURE.md)
- [Google image catalog](https://dl.google.com/android/repository/sys-img/google_apis_playstore/sys-img2-3.xml)
