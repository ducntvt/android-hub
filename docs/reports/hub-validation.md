# Android Hub validation — 2026-11-13

US-002: implemented on local Apple Silicon macOS.

## Checks

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: pass, 4 unit tests (domain validation, process isolation/path with spaces/port mismatch, duplicate registry ports, shell quoting).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo build -p android-hub` and `bash scripts/build-hub-app.sh`: pass, ad-hoc signed local .app.
- `cargo run -p hub-infrastructure --example smoke`: pass; creates independent AVD, saves 1 GiB/2 CPU/480×854/15 FPS, reloads registry, starts, rejects duplicate start and edit while running, stops and asserts original profiles/PIDs unchanged. [Log](hub/smoke.log).

## UI observations through CUA

1. Hub displayed adopted A/B, running A CPU/RSS in initial observation.
2. Created `hub-1794541891152` through “Tạo máy Android”; selected new Android 4.
3. Edited name to “Android Test”, saved; UI showed saved notice and new sidebar/detail name.
4. Started via UI; running status and changing CPU/RSS appeared. ADB on serial emulator-5586 returned `sys.boot_completed=1`.
5. Closed Hub via window close button: Hub process exited while test engine PID 68361 remained.
6. Reopened .app: saved name and active test engine detected.
7. Stopped test via inline confirmation: stopped state displayed. Earlier 1 GiB smoke instance was also stopped successfully via UI.
8. Archived only the three newly created test profile entries in `runtime/hub-smoke-archive/profiles.json`; all test data retained in emulator-avds, all test engines stopped. Final UI showed A/B only. No A/B stop or restart performed by these test steps.

## Findings and limits

Initial unoptimized tiny-skia debug build became unresponsive; enabled dev optimization for application/dependencies, rebuilt and completed UI interactions. This is not a sustained CPU benchmark.

Initial integration smoke tried stopping before console listening and received connection-refused. No wrong engine was stopped. Smoke now waits/retries bounded to 120s. UI explicitly says engine start does not imply guest boot finished; attempting stop immediately during boot can still report a retryable connection error.

Settings save/reload were checked in the backend smoke; name save/reload and lifecycle were also checked via real UI. All SDK binaries/images are prerequisites; missing SDK paths produce an error, not an automatic download. Retained emulator windows are external. No embedded screen, keymap, clone/delete UI, or completed game-performance P0 benchmark.
