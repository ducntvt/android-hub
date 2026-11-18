# 0039 — Stop managed engines before quitting Hub

- **Status**: Accepted
- **Date**: 2026-11-16
- **Supersedes**: Quit-keeps-engines behavior in ADR 0004/0005

## Context

The user explicitly requests that closing Hub also stop the background Android engines. Closing a window and macOS Quit use different native paths.

## Decision

Route window CloseRequested and AppKit terminate: through one asynchronous shutdown state. Disable Iced automatic close. Intercept only terminate: on NSApplication before starting Iced, preserving Winit's delegate and event dispatch; successful cleanup uses iced::exit(). No additional dependency or daemon.

Wait for in-flight mutations; request SDK setup cancellation through its existing cooperative control. Reject new actions while closing. Release display sessions and call Backend::stop_all, using existing per-profile identity/PID checks and bounded ADB operations. Continue attempting other registered engines after a failure. Exit only after a fresh snapshot confirms none remain. On error retain the window with Retry/Return to Hub.

## Consequences

Normal close/menu/Dock/Cmd+Q stop registered engines, including ones already running when this Hub opened. Shared SDK ADB server and unrelated emulators are not killed. No force-kill fallback. Stop may take time while Android exits. Force Quit, crash and power loss bypass this cleanup; another process concurrently starting a guest is not serialized with final application exit. Two Hub windows sharing a registry manage the same engines.

## Alternatives considered

A window-only handler misses native Quit. Replacing the AppKit delegate conflicts with Winit's lifecycle ownership. Killing children indiscriminately bypasses identity checks and can interrupt guest writes. A process watchdog for crashes is outside this request.
