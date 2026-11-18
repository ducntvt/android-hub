# 0044 — Observe guest boot separately from process and display readiness

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub maintainers
- **Supersedes**: none

## Context

PID presence proves only that the engine exists. Receiving pixels does not prove
that Android has completed boot. Shutdown already stops registered engines with
identity checks, but its single result hides partial progress from users.

## Decision

Add a cancellable, read-only infrastructure boot observation separate from
Snapshot. Bind it to the expected profile, PID, exact online ADB transport and
guest boot ID. The emulator-console name command requires serial selection (`-s`), because
`adb emu` does not honor `-t` when several emulators exist. Recheck the exact
transport after the console query; all guest shell reads use `-t`.
Check engine identity before/after and boot ID around reading
`sys.boot_completed`. Each command has a three-second timeout and the whole
observation twelve seconds; cancellation kills the current child process.

Desktop owns observation subscriptions keyed by ID/PID/epoch. Retry at five-second
intervals while waiting, stop on confirmed readiness or a two-minute cap, and
offer an explicit retry after failure. A changed display connection invalidates
readiness and requires a new observation. Removing/stopping a guest or quitting
removes its subscription; late messages cannot update another session.

Ready-to-control requires confirmed boot, a current frame and the display input
gate. Hidden devices may show boot completed but never claim input readiness.
Start/stop action elapsed time and errors remain distinct from display errors.

Expose Waiting/Stopping/Stopped/Failed progress from stop_all while retaining the
existing identity-checked stop operation and final snapshot guard. Desktop keeps
per-device results through failure/retry; no force-kill or changed Quit semantics.

## Consequences

- No registry/schema or guest-setting changes; all new boot commands are reads.
- Ready probes stop consuming ADB/process resources after confirmation. A short
  reconnect revalidates once, rather than using a stale cached readiness result.
- Asynchronous subprocesses are confined to infrastructure; widgets run no shell.
- Progress events do not authorize exit. Only the final verified stop_all result
  can close Hub. A failed device does not prevent attempts to stop the others.
- Native layout/reconnect/quit smoke is still required by US-035; unit tests alone
  do not prove those user-visible outcomes.

## Alternatives considered

- Add ADB calls to every three-second Snapshot: rejected because slow devices
  would delay unrelated process/resource refreshes and run forever after boot.
- Treat the first display frame as ready: rejected because boot animations also
  produce frames.
- Clear all shutdown results on first failure: rejected because it hides which
  devices remain active and prevents useful partial progress feedback.

## References

- [US-035](../stories/US-035-device-lifecycle-feedback.md)
- [Stop-on-Quit](0039-stop-engines-on-quit.md)
