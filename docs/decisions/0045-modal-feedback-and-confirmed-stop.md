# 0045 — Keep confirmations outside stage layout and bind stop to a session

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub desktop implementation
- **Supersedes**: none

## Context

Inline confirmations and APK results reduce the Android stage height. A single
status string cannot preserve errors alongside progress. A stop confirmation
previously referred to the current selection rather than the process seen when
the user opened it. The asynchronous stop command can also wait for a lock.

## Decision

Use a stack above the unchanged workspace for notifications and one modal. Keep
typed Success/Info/Error/Unknown notifications, monotonic IDs for toast expiry,
and a persistent queue for errors/uncertain outcomes. A successful poll must not
clear these alerts. Keep receipts in a separate history panel with an unread mark;
acknowledging them does not delete their in-memory or persisted history.

Capture mouse, wheel and keyboard events in a modal widget, and independently
reject background commands in the root update handler. Opening/closing a modal
releases input and advances the input epoch. The initial outlined action is
Cancel; Tab changes the outlined action, Enter/Space invokes it, Esc cancels.
Restore Android keyboard focus only if ID, PID and display generation still
match. Continue accepting asynchronous results while a modal is open.

Snapshot stop ID/name/PID in the desktop confirmation. Add `Backend::stop_session`
so the expected PID is checked after acquiring the lock and before ADB commands.
Existing `stop`/`stop_all` retain their normal lifecycle behavior and existing
identity checks. No force kill, changed guest protection, or storage migration.

Raw diagnostics remain in notification Details. Copy error uses an allowlisted
summary (Hub version, severity, translated operation); it excludes raw diagnostics,
paths, filenames, device names and guest/clipboard content. It is not a full log
export and the UI states that limitation.

## Consequences

- Notifications and confirmations no longer participate in stage sizing.
- Dismissed toast timers cannot remove newer feedback. Repeated identical poll
  failures are deduplicated; unresolved distinct alerts remain until dismissed.
- Iced `advanced` is enabled for the small modal event wrapper. This is necessary
  because `opaque` captures mouse presses only, not wheel or keyboard input.
- Native layout/focus behavior must still be checked; state tests alone do not
  establish accessibility or visual correctness. See US-036 for evidence.

## Alternatives considered

**Keep inline confirmations:** rejected because adding/removing them resizes the
live Android canvas and changes the pointer mapping during use.

**Only draw a dimmed layer:** rejected because queued messages, focused text
fields and Android keyboard/wheel events can still reach the underlying view.

**Check the PID only in the desktop:** rejected because the queued worker can
observe a replacement process after confirmation. The backend must check it too.

## References

- [US-036](../stories/US-036-feedback-and-confirmations.md)
- [ADR 0043](0043-adaptive-device-workspace.md)
- [ADR 0039](0039-stop-engines-on-quit.md)
