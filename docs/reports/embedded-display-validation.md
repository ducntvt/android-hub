# Embedded Android validation — 2026-11-13

## Scope

US-003 replaces the external-window-only manager with live Android inside Iced. Engine remains Android Emulator; this is not a new hypervisor or a game-performance benchmark.

## Automated checks

- cargo fmt --all -- --check: pass.
- cargo test --workspace: pass, 6 tests including coordinate scaling/clamping and press/move/release before parent focus update.
- cargo clippy --workspace --all-targets -- -D warnings: pass.
- scripts/build-hub-app.sh: pass; local ad-hoc signed bundle.
- Two authenticated RGBA probes passed: 480×854, 1,639,680 bytes, separate engine PIDs and console/gRPC ports. See [logs](embedded/probe-1.log) and [second probe](embedded/probe-2.log).

Cargo reports a future-compatibility warning from transitive `block 0.1.6`; current build/clippy succeed.

## Native UI checks (CUA)

Two separate disposable AVDs were created: hub-1794542403877 (5584) and hub-1794542709982 (5586), each 1 GiB/480×854. Both were confirmed booted using sys.boot_completed=1. Both engines used qemu-system-aarch64-headless and gRPC bound to 127.0.0.1 with token authentication.

Observed in Android Hub:

- Both Android screens displayed side by side, with no external emulator window.
- Tap opened Play Store; Escape returned Home. No login or agreement accepted.
- Drag from bottom to top opened the left app drawer; right guest stayed at Home.
- Tap opened Settings on the left; typing `display` produced local Settings search results, right guest unchanged.
- Home button acted on the correct device; switching single/split view and reopening Hub reconnected to existing engines.
- Both disposable engines were stopped through Backend::stop and removed from the main registry only after completion. Registry entries archived in runtime/hub-smoke-archive/embedded-profiles.json; AVD data retained. A/B were not stopped for testing.

A gesture bug was found and fixed: use CursorMoved event coordinates rather than the cursor snapshot, and preserve pointer state across event batches before the parent updates focus. Regression test covers this. Temporary pointer diagnostics were removed before final packaging.

## Resource observation and limits

One point-in-time sample with two test screens showed Hub ~10.9% CPU and ~81 MiB RSS after moving to wgpu/Metal. This excludes GPU allocations and is not a sustained benchmark or game FPS result.

Stream requested at most 480×854; UI samples latest frame at ~30 Hz. Old frames are replaced, not queued. Single touch pointer and ASCII keypress supported; no multi-touch/keymapping, clipboard, Vietnamese IME or rotation control yet. Existing old-mode engines require an explicit stop/start to enable embedded transport. No automatic restart of running user machines.

## Historical A/B handoff and launcher issue

A was started from the final Hub UI and remained running. B's legacy configuration open stalled inside macOS `open`, confirmed by a process stack sample. A bounded subprocess read returned a timeout instead of hanging the UI. An atomic normalization retaining every configuration value (backup runtime/p0-google-b-config-before-hub.ini) did not resolve the native-launch stall.

Start now applies settings via supported SDK flags rather than rereading config.ini. B's LaunchServices-launched emulator still stalled before logging, so that unbooted launcher was terminated and B was started through the SDK CLI with the same headless/authenticated arguments. Both A/B were then visually confirmed side by side inside Hub. No A/B guest disk/account reset or security setting changes were made.

**Status at that handoff (superseded by revalidation below):** B could be viewed/controlled, but its native-app launcher path had not passed. The exact cause of the macOS file-open stall was unconfirmed. That CLI-launched B belonged to exec session 59398. These process/session details are historical, not the current runtime state.

## Revalidation 2026-11-13 — new B session

User explicitly confirmed the current B session was started with the Hub Start button, not Terminal. Read-only process inspection found PID 97954 running `p0-google-b`, console 5582, token-protected gRPC 8582, headless host GPU flags. Hub PID 97940 started shortly before this engine. Parent PID 1 alone is not launch provenance; the user's confirmation supplies that evidence.

`python3 scripts/p0-emulator-probe.py b` passed: expected AVD, boot complete, Android 16/API 36/arm64-v8a, Play Store and Play Services present, 720×1280, verity enforcing. Current evidence is [emulator-b.json](p0/emulator-b.json). No input, installation, account access, reset or restart was performed.

This supersedes the earlier claim that B currently requires a CLI launch. It proves this launch succeeded, not that an intermittent macOS file-open issue is permanently fixed. Repeated native restart testing remains pending, without interrupting the user's current session.
