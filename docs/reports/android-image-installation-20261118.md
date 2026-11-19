# On-demand Android image installation verification

US-050 separates common SDK tools from optional Google Play ARM64 images for
Android 12–16. SDK setup prepares three shared tools; image setup adds one reviewed
package to the selected SDK without replacing the engine or any existing image.

## Installation contract

- Plans bind the exact package, revision, official Google archive, checksum,
  minimum engine and license text/hash. Image destinations come from the closed
  domain version enum. Each UI review requires explicit consent.
- Admission counts remaining download bytes, an estimated 6 GiB extraction area,
  12 GiB for a new guest and reserve space. Separate SDK/data filesystems are
  checked separately; the ZIP's actual extraction budget is checked before writes.
  These checks do not reserve disk or predict future game storage.
- An SDK-scoped lock prevents overlapping image installs. Short Hub locks protect
  selection/reference admission and publication; downloads do not hold the Hub
  mutation lock, so existing guests can continue receiving input or stop.
- Extracted contents and engine dependency are verified before publication. An
  ownership/provenance receipt travels with the image in one no-replace rename.
  Existing valid images are reused, including externally installed images; their
  ownership is not changed. Existing invalid packages fail closed.
- Missing images still referenced by devices, retained AVDs, backups or pending
  journals are not replaced with a new catalog revision. The UI does not update
  images, engines or the selected SDK automatically.
- Interrupted private staging stays outside package paths and is never considered
  ready. Cache is retained for explicit retry; a crash after publication is
  recovered by verifying/reusing the complete package on the next attempt.

## Verification

`bash scripts/check.sh` passed: 89 desktop, 5 domain, 129 infrastructure tests,
2 metrics tests and 32 Python tests, plus formatting, Clippy and documentation.
The native bundle build and strict codesign verification passed. The build uses
macOS iconutil outside the sandbox; its sandbox invocation reported Invalid Iconset.

Fixtures cover consent, pre-cancel, checksum failure and retry, conflicting image
locks, no-clobber publication even over an empty external destination, existing
revision references, per-filesystem space boundaries, common/image-only plans,
and retained registry/engine/userdata. Explicit SIGKILL tests passed before and
after publication; retry installed or reused the package respectively. The
localhost stalled-transfer test passed and confirmed cancellation reaped curl
while retaining the partial archive. A separate interrupted-response fixture passed:
only half of the declared bytes arrived, no complete archive was published, and
retry requested Range/If-Range and verified the final checksum. Opt-in network/crash tests are separate from
shared checks and do not download real SDK packages in CI.

Parallel checks exposed an existing flock lifetime issue: dropping the owning
File could leave a transient fork's descriptor holding the lock. Hub and SDK now
share an explicit unlock guard. Its duplicate-descriptor regression and the
previously failing registry/cache tests passed in the full parallel suite.

## Real image smoke in the isolated SDK

Host: Apple M2, macOS 27.0.1; shared Emulator 37.2.12. QA root:
`/tmp/android-hub-e01-qa-20261118`. The user approved the QA image license; all five
catalog entries have the same approved text SHA-256
`1889ae9e14dec242a8c5276dbed22a655a744497113ee23c13fdf88d2d0dbfa2`.

Android 13 / API 33 / revision 9.0.0 installed via the production image workflow:

- Official archive: `arm64-v8a-33_r09-darwin.zip`, 1,713,468,508 bytes.
- Published SHA-1: `97b81c9eb787667e53d5ae9de428284e4f230259`.
- Verified installed identity: `ae384e5710c5b851f839163c4d864cb8e3acd5e183c5ddc956b3a5b916675a8d`.
- First result: `reused: false`; second attempt: `reused: true`, same identity.
- Android 12 remained booted during installation. Afterwards its authenticated
  stream, drag, horizontal/vertical wheel, Unicode clipboard, rotation, reconnect
  and interface/DNS/HTTPS probe passed. It was stopped normally afterwards.

Native QA at 1000×640 confirmed the EN/VI image list, installed revisions, missing
image download size and scrollable license review. The download action was disabled
without consent. Back returned to the list without installing Android 14. The QA
app was quit and its bundle removed. SDK/images/AVDs remain isolated inputs for
US-051–053; the main SDK and device data were not changed by installation testing.

## Boundaries

This verifies package installation and reuse, not five-version guest qualification
or game compatibility. The creation selector and removal UI are separate stories.
Real disk exhaustion was not induced on the user's volume; admission and ZIP-budget
fixtures exercise that boundary. The shared filesystem remains subject to ordinary
external changes and free-space races; publication never overwrites existing data.

Final epic cleanup: the retained QA SDK/AVD inputs were subsequently removed; see
[US-053 qualification and cleanup](android-versions-qualification-20261118.md).
