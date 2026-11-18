# 0046 — Preserve settings drafts and compare the saved profile under lock

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub desktop implementation
- **Supersedes**: none

## Context

US-037 requires unsaved edits to survive navigation and polling, and prevents a
stale editor from silently overwriting a configuration changed by another Hub.
Comparing only the last desktop snapshot leaves a race while the save is queued.

## Decision

Keep an original profile alongside the editable draft. Dirty compares draft to
that original; conflict compares the original to the latest registry snapshot.
Clean drafts follow refreshes; dirty drafts keep their content. Reload/discard
explicitly adopts the latest saved profile.

Intercept context changes with a Save / Discard / Stay modal. Retain a typed
navigation intent, never an Android input event. Save continues that intent only
after a successful backend result; failure keeps the draft and cancels the intent.
Switching by clicking another device requires the same decision, then a fresh
click to send input. No event is replayed across the modal.

Add `Backend::save_if_unchanged(expected, profile)`: compare the entire saved
profile under the existing Hub lock before the normal stopped/port/write guards.
The ordinary `save` API remains for existing controlled backend workflows.
External writers that ignore Hub locking remain outside the concurrency guarantee;
the desktop still detects their changes when polled. No registry schema migration.

## Consequences

- Dirty edits are never implicitly applied to another device or lost by polling.
- A conflict disables Save and offers Reload; the app does not merge guesses or
  stop a running guest to save.
- Navigation waits for confirmation and, when requested, the save result.
- Desktop state tests, a stale-editor backend fixture, and isolated native QA
  are required before US-037 can be marked implemented.

## Alternatives considered

**Keep a warning in the status bar:** insufficient because the user cannot choose
discard/stay and context changes can still overwrite the draft.

**Compare only in the desktop:** rejected because another writer can change the
registry before the background save acquires the lock.

**Persist drafts or merge fields automatically:** deferred; not required by US-037.

## References

- [US-037](../stories/US-037-device-settings-drafts.md)
- [ADR 0045](0045-modal-feedback-and-confirmed-stop.md)
