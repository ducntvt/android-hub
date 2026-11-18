# Development

Android Hub is a macOS Apple Silicon Rust workspace. Current verification uses Rust/Cargo 1.96.1 with rustfmt and clippy, plus Python 3.9+ for script tests. This is a verified toolchain, not a claimed minimum supported Rust version. No cargo-scaffold installation is required.

## Get started

Clone the private repository and install the verified toolchain using the
[README](../README.md). Use `develop` for new work after the owner has created that
branch; use [GitHub setup](GITHUB.md) for initial authentication/bootstrap.

`rustup override set 1.96.1` selects the verified toolchain for this checkout.
The repository does not yet pin it with `rust-toolchain.toml`. Network access is
needed to install the toolchain and obtain uncached Cargo dependencies. Keep the
lockfiles and use `--locked` for normal checks/builds.

## Source map

```text
Cargo.toml                     workspace metadata, dependency versions, build profiles
Cargo.lock                     resolved versions; keep this for the application
crates/
  domain/src/lib.rs            Profile, Runtime, validation; no I/O
  infrastructure/
    src/lib.rs                Backend facade, shared engine/storage operations
    src/snapshot.rs           process observations and Android version parsing
    src/shutdown.rs           shutdown progress aggregation
    src/tests.rs              facade and process-identity regression tests
    src/sdk.rs                SDK module root and public exports
    src/sdk/                  plan, setup, download, extract, install, cache, recovery
    src/display.rs            authenticated emulator display/input transport
    src/...                   device storage, APK, backup/restore/removal
    resources/                canonical bundled SDK manifest
    android/                  scroll helper Java source + required embedded DEX
    proto/                    emulator protocol input for build.rs
    examples/                 explicit diagnostic programs
  desktop/src/
    main.rs                   bootstrap only
    app.rs                    state and initialization
    app/message.rs            application events
    app/runtime.rs            window bootstrap and subscriptions
    app/sdk.rs                SDK setup/selection event handling
    app/update.rs             state transitions and async dispatch
    app/ui.rs                 workspace composition and shared presentation
    app/ui/settings.rs        device-settings panel
    app/screen.rs             Android canvas and pointer/keyboard mapping
    app/tests.rs              state regression tests
    data_root.rs              storage-root selection
    i18n.rs                   EN/VI catalog and preference
    quit.rs                   macOS native Quit bridge
  desktop/assets/             app logo source; bundle icon derived during build
scripts/                      build, verification, SDK metadata and diagnostic tools
tests/android-input/          Android input QA source, not generated APK output
tests/android-connectivity/   guest connectivity helper fixture source
docs/                         design, setup, stories, historical evidence
tools/metrics/                separate metrics tool, excluded from app workspace
~/Library/Application Support/Android Hub/
                              REAL USER DATA; outside source
```

Dependencies flow `desktop → infrastructure → domain`, with desktop also using domain models. Keep widgets free of subprocess calls. SDK implementation helpers remain private; consumers use the infrastructure facade. Add a new crate only when there is a real independent boundary.

For UI work, reuse the control, icon and tooltip conventions in [UI style](UI-STYLE.md)
instead of introducing per-screen defaults.

Use kebab-case for Cargo packages, example targets and shell/Python command filenames; snake_case for Rust modules and Python test modules. A module directory supplies context: `sdk/download.rs`, not `sdk/sdk_download.rs`. Prefer `module.rs` with `module/` for child files; do not mix this with `module/mod.rs` in new code. Follow default rustfmt (no custom formatting config) and resolve Clippy warnings. Keep tests next to small modules; split large state tests into `app/tests.rs`. Historical report paths describe their original revision; use this map for current source.

## Commands

```bash
# Local checks: app + standalone metrics fmt/tests/clippy, Python tests, shell syntax
bash scripts/check.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py

# Build the locally ad-hoc-signed app when changing app behavior
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

The shared check includes the standalone `tools/metrics` metrics package. Set `ANDROID_HUB_ROOT` to an absolute disposable QA data directory, then run the optional display diagnostic with `cargo run --locked -p android-hub-infrastructure --example display-probe -- PROFILE_ID` (replace `PROFILE_ID` with an existing running QA device ID; this is not part of offline checks).

Checks do not open Hub, boot guests, install SDK packages or run opt-in ignored integration tests. Check failures stop the script. Android SDK/Java are required to operate Android guests or regenerate Android helpers, not to compile the Rust app with the checked-in resources. Build uses vendored protoc. Network may be needed for Cargo dependencies on a fresh machine.

For native minimum-window QA, package the existing binary with
`PYTHONDONTWRITEBYTECODE=1 python3 scripts/build-android-hub-qa.py /tmp/android-hub-your-task --window-size 1000x640`.
The destination must be new. The launcher isolates data, clears inherited SDK/Java/window-size overrides, and sets the validated initial size. Without this option, the app starts at 1180×820; minimum content size stays 1000×640. `ANDROID_HUB_WINDOW_SIZE=WIDTHxHEIGHT` is an optional launch override, not a stored layout preference. Invalid sizes fall back to the default. Packaging does not launch Hub or an Android guest.

Dependency versions and shared metadata live in root Cargo.toml. Crates inherit them with `workspace = true`; keep feature requirements at the relevant consumer where they differ. Preserve Cargo.lock and use `--locked`. Do not copy a generator's unrelated dependencies or introduce a new tool simply to run these commands.

## Resources and cleanup

The SDK manifest at `crates/infrastructure/resources/sdk-install-plan.json` is a build input. See its adjacent README for regeneration. The embedded DEX in `android/` is also a required resource; regenerate it with `scripts/build-scroll-bridge.sh` only when changing its Java source. Neither is cache. No source file may include a file from docs/reports or personal runtime.

`target/`, `tools/metrics/target/`, Python bytecode and task-specific temporary QA bundles/checkouts are disposable. After verification, close QA processes and remove only the task's known temporary outputs and local build target. Preserve the main Android Hub.app, SDKs, AVDs, registry, backups, historical evidence and Cargo.lock. Do not delete global Cargo caches. Check scripts retain build outputs so subsequent checks can reuse them; cleanup belongs at handoff.

Default data lives directly in `~/Library/Application Support/Android Hub`. `ANDROID_HUB_ROOT` selects an explicit data directory (QA uses `data/` beside its bundle). Neither mode appends a legacy suffix or depends on the source checkout. The old experimental runtime was deliberately reset with user authorization (ADR 0054), not migrated. Future moves of populated data still require migration.

## Project identity

Display name and project directory: **Android Hub**. Cargo packages: `android-hub`, `android-hub-domain`, `android-hub-infrastructure`; the excluded metrics package is `android-hub-metrics`. Main app bundle ID: `local.androidhub.desktop`.

Local packaging currently uses the optimized development profile, not `--release`.
Bundle version/build number are still literal fields in the packaging script;
[Releasing](RELEASING.md) describes keeping them consistent with Cargo. CI and
automatic tag publishing are not configured yet.

Product commands are `scripts/build-android-hub.sh`, `scripts/build-android-hub-qa.py` and `scripts/collect-android-hub-metrics.py`. Historical reports retain the command names used at the time. P0 research paths in old reports describe historical runs, not the current data layout.

Use `ANDROID_HUB_JAVA_HOME` for an explicit Java path. The legacy `P0_JAVA_HOME` fallback remains supported. Neither setting is written automatically into your shell configuration.

The retired VM path migration script and P0 setup/boot/launcher/probe scripts were
removed with the experimental runtime. Historical reports retain their original
commands; they are not current setup instructions. Use the app to create devices.
