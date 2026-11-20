# Setup and troubleshooting

English · [Tiếng Việt](SETUP.vi.md)

Build the local app using the [README](../README.md) first. This guide covers the
Android dependencies and data on your own Mac. Hub does not include an SDK/image.

## 1. Prepare Android SDK and Java

On an Apple Silicon Mac, use Android Studio's SDK Manager to install:

| Component | Expected location/package |
| --- | --- |
| Android Emulator | `emulator/emulator` |
| Platform-Tools | `platform-tools/adb` |
| Command-line Tools (latest) | `cmdline-tools/latest/bin/avdmanager` |
| Selected Google Play ARM64 image | Android 12/13/14/15/16 → API 31/33/34/35/36; `system-images;android-<API>;google_apis_playstore;arm64-v8a` |
| Java 17+ ARM64 | A Java home containing `bin/java`; Android Studio's bundled JBR is the default |

Default SDK: `~/Library/Android/sdk`. Default Java home:
`/Applications/Android Studio.app/Contents/jbr/Contents/Home`.
Review and accept SDK licenses yourself when prompted by SDK Manager.

Hub checks required files, nonempty payloads and executable permissions. These
checks do not prove a successful boot or that every game is compatible. Java is
needed to create devices; starting an existing device does not use avdmanager.

## 2. Select paths in Hub

Open **Hub settings → SDK setup**. With all devices stopped, enter the absolute
SDK path and choose **Verify and save SDK**. Enter the Java home and choose
**Verify and save Java**. Java validation requires version 17+ ARM64 and probes
the SDK tools using temporary test data.

Settings are stored in `hub-settings.json` under the app data directory. An invalid
selection keeps the previous setting. Existing AVDs/archives prevent switching to
a different SDK; SDK migration is not implemented.

Environment overrides apply only to the launched process:

| Variable | Effect |
| --- | --- |
| `ANDROID_SDK_ROOT` | Overrides the saved SDK; prevents saving a different SDK while set |
| `ANDROID_HUB_JAVA_HOME` | First-priority Java override |
| `P0_JAVA_HOME` | Legacy Java override, considered after `ANDROID_HUB_JAVA_HOME` |
| `ANDROID_HUB_ROOT` | Overrides the app data directory, primarily for isolated QA |

Java preference order: the two environment overrides above, saved Java home, then
Android Studio's default JBR. Finder launches do not automatically inherit shell
startup-file exports. Prefer the settings UI for everyday use.

## 3. Create and use devices

1. Choose **+**, enter a name, select Android 12–16 (default 16), and review configuration.
2. Create directly if the image is ready; otherwise review **Download and create**,
   size and license consent, then download. Select the device and start it.
3. Sign in to Google Play directly in Android if needed.
4. Create another device and select **2 devices** to control both in one window.

Each device starts clean. Accounts and apps are not copied between devices. Device
creation requires at least 12 GiB free on the AVD filesystem; this is an admission
threshold, not reserved space or a guarantee of room for future games.

Configure CPU/RAM/resolution/orientation/FPS while stopped. Available FPS limits
are 15/30/60/90/120, default 30. Actual game FPS may be lower. Start with a modest
limit and increase only after checking host load and display refresh rate.

Closing Hub normally stops registered engines; failed shutdown keeps Hub open for
retry. Force Quit/crash cannot run that cleanup. Stop devices through Hub before
changing data or SDK configuration.

## Android versions and images

Android 12/13/14/15/16 map to API 31/33/34/35/36. Android 12L, previews,
x86 images and Android 17+ are outside this catalog. Install only the versions
you need. Devices share tools and image files, while keeping separate writable disks.

Version selection exists only in the creation form. Existing devices keep their
version, revision and image fingerprint, including after restart and restore.
To use another Android version, create a new device; there is no in-place upgrade
or downgrade. Older registries receive a metadata-only migration with a saved
original; unresolved images stay unresolved instead of being assigned Android 16.

For a missing image, **Download and create** reviews a single image package and
requires its license consent. Cancellation preserves the form/cache; retry does
not create a second device. The image needs remaining download bytes plus 6 GiB
extraction headroom, 12 GiB guest headroom and a 256 MiB reserve. Hub checks again
before publishing and creating. It does not silently update a shared emulator or
replace an existing image revision.

**Hub settings → SDK setup → Android versions** shows installed images, allocated
size and references. Removal is available only for verified Hub-managed images
with no device, backup, archive or pending-operation reference. A stopped device
still counts. External SDK images are read-only here. Removal keeps downloaded
archives; it does not uninstall the shared tools or delete device data. An interrupted
removal requires an explicit retry. Do not share Hub's private managed SDK between
independent data roots or manage it manually with another tool.

If an external SDK manager changes a bound image, Hub blocks operations that need
that identity. Restore the original image/revision; changing the device's version
field is not a recovery procedure. Play Store presence does not guarantee account,
game or service compatibility on every Android version.

## Data, backups and removal

The default data root is `~/Library/Application Support/Android Hub`, independent
of the checkout. Under it are:

| Path | Purpose |
| --- | --- |
| `hub-profiles.json` | Registered device configuration |
| `hub-settings.json` | SDK/Java choices |
| `ui-language`, `ui-appearance` | Hub presentation preferences |
| `emulator-avds/` | Device disks and metadata |
| `backups/` | Verified offline backups |
| `<device-id>.log` | Engine output |

Backup/restore requires stopped devices, compatible SDK/image fingerprints and
sufficient disk space. Restore keeps previous data in `.previous-*` for recovery;
there is no general UI cleanup for every retained copy. The current restore UI
requires the destination device to remain registered.

Use the device row's **⋯ → Delete device…** for permanent deletion. Review the
confirmation. **Back up first…** does not automatically delete afterward. Old
archives are available in Hub settings for restore/removal. Deleting a device does
not delete shared SDK/image files or its separate backups.

Never delete the whole data root as a remedy for SDK, display or build errors.
Copying only `userdata` is not a complete Hub backup.

## Managed SDK installer

Hub has a pinned SDK manifest with package/license review, explicit consent,
download progress, cancellation, retry/resume, checksum verification and assembly.
The complete newly downloaded SDK installation/boot flow remains under validation;
an existing compatible SDK is the documented starting path.

In **SDK setup → Review SDK installation**, review licenses, select consent and
start setup explicitly. Opening the review alone downloads nothing. Java must
already be installed. Consent is not preselected or retained across review closure.

Canceled downloads retain partial data. A corrupt/unresumable cache can be reset
per package after confirmation; old data is quarantined rather than silently
removed. Download admission requires remaining archive bytes plus 6 GiB headroom;
extraction checks space again. Completed SDKs appear under **SDKs prepared by Hub**
and are selected only after explicit verification. Existing device/SDK compatibility
gates still apply. There is no UI to delete/undo all quarantine data.

## Recovery and logs

Interrupted device creation leaves a receipt. Ready metadata may be registered on
recovery if it still matches; incomplete files are retained. If creation succeeded
but refresh failed, refresh/reopen Hub rather than immediately creating a duplicate.
Do not force-overwrite AVD metadata.

In v0.1.0, creation can report an AVD/image conflict when `config.ini` omits
`target`, even with a valid image path, ABI and flavor. Version 0.1.1
accepts this omission while retaining image validation. After installing a build
containing the fix, reopen Hub and create a new device. Incomplete files from the
failed attempt remain retained, not automatically registered or deleted. Do not
reset the SDK or remove the whole app data directory to work around this error.

Before starting an engine, a log of at least 8 MiB rotates to `.log.1`; Hub keeps up
to three rotated logs. It does not truncate a running session's log, which can grow
beyond that size. Copy logs you need before subsequent launches rotate them away.

## Troubleshooting

| Symptom | Next step |
| --- | --- |
| Clone says not found/access denied | Authenticate the GitHub account invited to the private repo; see [GitHub](GITHUB.md) |
| Build cannot find compiler/SDK tools | Check Rust toolchain and `xcode-select -p`; Android SDK is not required just to build Hub |
| Device creation fails | Verify SDK/Java, free disk space and environment overrides |
| Display disconnects | Retry status; if necessary, stop that device in Hub and start it again |
| One Android app reports an HTTP error | Run the guest connectivity check; a successful probe does not prove all services work |
| Dock shows a generic icon | Rebuild to refresh bundle registration, then reopen the app when convenient; do not reset Android data |

For a report, include tag/commit, macOS/chip, SDK/image version, steps and a redacted
error summary. Do not attach credentials, tokens or full device disks.

## Isolated QA

After packaging the app:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/build-android-hub-qa.py /tmp/android-hub-my-qa
```

The destination must be new. Open the generated **Android Hub QA.app** manually.
It uses `data/` beside the QA bundle, clears inherited SDK/Java overrides and does
not use personal Application Support data. Packaging does not launch Hub or boot
Android. Do not relocate a populated QA directory: Android metadata includes
absolute paths. Close test apps/guests and remove only disposable QA data afterward.

## Uninstall (v0.2.0 and later)

Open **Hub settings → Environment and data → Uninstall Android Hub…** before
deleting the app in Finder. Wait for any active write/download to finish or cancel
it first. Review the exact paths and allocated sizes. Optionally select **Keep
backups**, then type `DELETE` and confirm. Changing the backup option clears the
confirmation. Devices are stopped before a helper is prepared; Hub then closes
before deletion begins. Other Hub/Android processes block uninstall.

Only the running app in `/Applications/Android Hub.app` or
`~/Applications/Android Hub.app` and the default Application Support data root
are eligible. Development checkouts, arbitrary data overrides and translocated
app copies are refused. Isolated temporary QA bundles support this flow for tests.
External SDKs/Java, Android Studio, source, downloaded DMGs and other app copies
are retained. Keeping backups leaves only `backups/` at its original location;
restoring still requires compatible SDK/images. Allocated size is an estimate,
not guaranteed reclaimed space on APFS.

No administrator elevation or automatic retry is performed. If cleanup fails
after Hub exits, a text report opens and remains in the helper's private temporary
directory; some confirmed data may already have been deleted. The app is removed
last, but a failure during that step can also leave a partial bundle. If a crash
leaves `uninstall-pending.json`, normal mutations remain blocked for inspection;
do not delete unrelated paths or restart the helper blindly. Deleting the app in
Finder does not trigger this workflow and leaves its data behind.

## USB phones (feature branch, US-058–061)

Available in source on `feature/physical-devices`; not yet released.
USB discovery needs an existing Android SDK Platform-Tools installation, not Java,
Emulator or an Android system image.

1. Download and extract [Platform-Tools from Google](https://developer.android.com/tools/releases/platform-tools), reviewing its license yourself.
2. In **Hub settings → Phones via USB**, enter the absolute path to `platform-tools/adb` and choose **Verify and save ADB**.
3. If the shared server is not running, run the displayed `'<absolute adb path>' start-server` command in Terminal. Hub reads the default loopback server on port 5037. It never stops/replaces the server or supports custom server endpoints in this version.
4. Connect a data-capable USB cable. Unlock the phone, enable **Developer options → USB debugging**, and accept its RSA authorization prompt yourself.
5. Select the phone under **Connected devices**. Hub shows model, Android version and authorization/connection state. Unplugging and reconnecting updates automatically. **Refresh USB devices** retries failed metadata/discovery.

Network and unidentified transports are labeled separately; known external
emulators are not imported as phones. Discovery does not start a stream or change
phone settings. To view an authorized USB phone, choose 30/60 FPS and a maximum
long edge (720/1280/1920px), then **View display**. Stop before changing stream
caps. Fit/zoom and rotation preserve the image aspect ratio. The caps do not
guarantee delivered FPS or change the phone's system resolution.

The bundled scrcpy 5.0.1 server is uploaded to a unique temporary phone path for
this session and cleaned on stop/quit. VideoToolbox hardware H.264 decode
is required. Lock/protected content may appear blank; Hub does not bypass capture
restrictions. Click the display to focus phone input: click/drag, two-axis scrolling,
Back/Home/Recent and printable ASCII keyboard input use the session control channel.
IME/Unicode and held gaming keys are not supported. Hub settings temporarily block
phone input. In two-device mode, click the intended display before typing; the
phone can remain visible beside an emulator. System rotation controls are not
exposed for phones; rotate the phone itself when its app supports it. If video
works but control does not, check the manufacturer's USB input permission guidance.
Audio remains future work. APK and clipboard support (US-061) has passed native Samsung Android 15 and
emulator destination checks, including reconnect and interrupted installation.

Choose **Install APK** on a ready USB phone, or drop a single APK and choose its
destination. Review the model and file before confirming. Hub binds the action to
that USB session, does not replace existing apps, grant runtime permissions or
launch the installed app. After an interrupted installation, check the phone
before trying again: an unknown result does not mean installation failed.

With the phone display open, the clipboard icon sends macOS text only when clicked
(up to 64 KiB). It verifies the phone clipboard by acknowledgment and read-back;
it does not paste into an app or synchronize automatically. Unicode text can be
sent this way even though direct keyboard input is ASCII-only. Reconnecting or
switching the target invalidates the pending action; send again explicitly if needed.
Viewing must be restarted explicitly after a connection failure; queued input is
never replayed into a new connection.
Only the ADB executable path is saved; phone metadata stays in memory. On version
conflicts, coordinate Platform-Tools versions with other ADB applications instead
of stopping their server from Hub.
