# Android 12–16 qualification

US-053 / Epic E01. Qualification completed for the host, image revisions and
checks listed below. Game compatibility and remote CI execution remain separate.

## Environment and scope

2026-11-18, Apple M2, macOS 27.0.1, Android Emulator 37.2.12 (16428233),
Android Studio JBR 25.0.3, Rust 1.96.1. Google Play ARM64 images, Pixel 5 profiles,
2 CPUs, 720×1280, host GPU, 30 FPS. Android 12/16 used 2048 MiB; 13/14/15 used
3072 MiB. Stream dimensions were 480×854 portrait and 854×480 landscape.

| Android | API | Image revision | Image fingerprint SHA-256 |
| --- | --- | --- | --- |
| 12 | 31 | 9 | `1bd2f8d55838636eb0c2ef9576d79bf21c75764fef39d6005ea6c3bdbbe35f87` |
| 13 | 33 | 9 | `ae384e5710c5b851f839163c4d864cb8e3acd5e183c5ddc956b3a5b916675a8d` |
| 14 | 34 | 14 | `c5fdd150054f798b4a0479271103b852a758ac94f6b75943fb49fdb13bba2ffd` |
| 15 | 35 | 9 | `0417c4c6b779f1a3f84e7e73d9f7643b2b4a86a763c40a97ae2b77ad939a53af` |
| 16 | 36 | 7 | `f5193c81e6c70e8a45dde384c4c9c48d1c0935373a41ef101ac0298eeb73cdb0` |

Every package is `system-images;android-<API>;google_apis_playstore;arm64-v8a`.
The checked-in catalog records download size/checksum/license. API 31 was installed
from the verified official archive after consent; APIs 33/34/35 used Hub's production
image installer. API 36 reused an existing installed image read-only; its source
archive was not downloaded again. QA used a separate SDK selection and data root.
No personal AVDs, accounts or SDK packages were modified.

## Per-version results

Backend checks use the checked-in `device-qa` example and the offline input fixture.
They exercise production Session, APK installation, network, backup and restore
APIs; they do not substitute for testing every macOS gesture or picker interaction.

| Case | 12 | 13 | 14 | 15 | 16 |
| --- | --- | --- | --- | --- | --- |
| Independent creation / verified binding | Pass¹ | Pass | Pass | Pass | Pass¹ |
| Boot and normal stop | Pass | Pass | Pass | Pass | Pass |
| Authenticated stream / reconnect | Pass | Pass | Pass | Pass | Pass |
| Touch down/drag/up | Pass | Pass | Pass² | Pass | Pass |
| Horizontal and vertical wheel | Pass | Pass | Pass² | Pass | Pass |
| Unicode clipboard ACK + fixture paste | Pass | Pass | Pass² | Pass | Pass |
| Rotation | Pass | Pass | Pass² | Pass | Pass |
| Install and launch fixture APK | Pass | Pass | Pass | Pass | Pass |
| Interface / DNS / HTTPS probe | Pass | Pass | Pass | Pass | Pass |
| Offline backup, verified restore, unchanged identity | Pass | Pass | Pass | Pass | Pass |
| Boot and fixture probe after restore | Pass | Pass | Pass² | Pass | Pass |
| Play Store package and launch activity | Pass | Pass | Pass | Pass | Pass |
| Reopen Hub: all profiles/version labels retained | Pass | Pass | Pass | Pass | Pass |

¹ API 31/36 guests originated in US-048 through SDK creation; US-049 migrated
and verified their bindings. API 33/34/35 guests were created from Hub's version
picker. Pure creation tests cover all five domain versions.

² One immediate post-restore Android 14 run observed zero input counters. After
restarting and observing a stable Home screen, the same probe passed completely.
No production input change was justified by that transient observation. Subsequent
matrix starts allow ten seconds after boot before launching the fixture. This is
not a cold-boot latency benchmark or a claim that transient failures cannot recur.

Successful probes observed `Down=1 Up=1 Moves=1 Held=false H=1.00 V=1.00`,
`Clipboard=matched`, a changed rotation frame, authenticated reconnect and
`INTERFACE OK / DNS OK / HTTPS OK`. Restored profiles matched the originals,
including version/revision/fingerprint. Each restored guest booted and reused its
installed fixture before disposable backup/previous-data copies were removed.

## Defect found and fixed

Real API-33 backup initially rejected an unchanged image. `avdmanager` wrote a
trailing slash in `image.sysdir.1`; the inventory's serialized path spelling differed
from the catalog's path and therefore produced a different fingerprint. Identity
hashing now normalizes path components. Existing backup manifest spelling remains
unchanged, preserving prior restore comparisons. A regression test creates a bound
image/AVD with the trailing slash, backs up and verifies restore, then changes the
payload and checks that backup still rejects it. The real API-33 drill passed after
this fix, without rebinding the device or changing its image.

## Two-device and native checks

Android 12 and 16 ran concurrently on separate ADB/gRPC ports. Installing the
fixture on Android 12 left it absent on Android 16; installing it on Android 16
then left both present. Authenticated input and Unicode clipboard probes passed
in both directions: the other guest retained its distinct clipboard and unchanged
input counters. The probe accounts for rotated scroll axes; the first pair attempt
had a portrait-only expectation and was corrected before both isolation passes.

The first stop observation involved two CLI-held guests and both exited; its cause
was not established and that attempt is not counted as a pass. Both guests were
then started directly from Hub. Native Stop on Android 12 left Android 16 running
with its own emulator PID; a click opened its Play Store sign-in screen. Quit Hub
then closed the remaining Android 16 guest. Process inspection confirmed no QA
Hub, emulator, device-qa helper or netsimd remained. Reopening Hub earlier retained
all five profiles and their version labels.

Earlier native touch/drag, keyboard, wheel and two-screen evidence is in
[US-048 report](android-version-spike-20261118.md). Native horizontal trackpad
automation remains limited; backend two-axis results do not certify physical
trackpad behavior on every version.

## Prior lifecycle and safety evidence

- [US-049](android-image-identity-20261118.md): metadata migration, immutable
  binding and backup compatibility gates.
- [US-050](android-image-installation-20261118.md): consent, cancellation,
  retry, publication/recovery and existing-image protection.
- [US-051](android-version-creation-20261118.md): native selection, download/create,
  canceled review and prevention of stale completion/duplicate creation.
- [US-052](android-image-management-20261118.md): native remove/cancel/reinstall,
  references, external SDK read-only behavior and interrupted-removal fixture cases.

## Build and CI

The final shared check passed: 93 desktop + 5 domain + 139 infrastructure tests,
2 metrics tests, 32 Python tests, rustfmt, Clippy and documentation/shell checks.
25 opt-in integration tests remain ignored by the shared command; the deliberate
real-guest matrix supplies separate evidence. App packaging and deep strict
codesign verification passed. No commit, push, tag or remote CI run was performed.

Both helper scripts reproduced their checked-in DEX byte for byte using platform
35/build-tools 35.0.1 and min API 31. SHA-256:

- ScrollBridge: `777c6b85de04823e8c71dd4b501bb65ac753ef9585967377841bfbfe1e854b46`.
- ConnectivityProbe: `5da20a92c9f6f78079d93fcc0b744a997b089c5b36cf9632b64a60f729dddb1f`.

The added Ubuntu CI job rebuilds helpers, runs the Java connectivity fixture,
checks DEX drift and builds the input APK using preinstalled tools. It downloads
no Android image and boots no guest. Remote runner execution remains to be
confirmed after the user commits/pushes.

## Limits and cleanup

Play Store checks did not log in, download games or accept account terms. Top Heroes
and Hunt Royale were **not tested on any version in this matrix**. Earlier personal
Android 16 gameplay is historical evidence only. This report does not guarantee
Play Integrity, service availability, game FPS, 90/120 Hz, other hosts or revisions.
Fresh-machine common-SDK installation and long two-game benchmarks remain outside
this epic's qualification.

Final process inspection found no QA Hub, guest, device-qa helper or netsimd.
Removed `/tmp/android-hub-e01-qa-20261118` (about 27 GiB allocated), including its
isolated SDK, five disposable AVDs, archives, logs, fixtures and QA bundle. Removed
workspace `target/` (about 7.6 GiB) and `tools/metrics/target/` (about 4.6 MiB).
Directory absence and the preserved main app executable were verified afterwards.
External SDK symlink targets and personal app data were not deleted. No generated
Python/cache directories remained in the source scan. Final rustfmt, Clippy,
documentation checks, diff whitespace and bundle signature checks passed before
build-cache removal; the app was not rebuilt merely to verify cleanup.

The prior local test logs were reviewed before disposal. Durable results, including
failed attempts and limitations, are recorded above. The `device-qa` example and
fixture sources remain for intentional future qualification; SDKs, AVDs and QA
bundles are not source deliverables.
