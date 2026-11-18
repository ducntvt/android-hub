# 0047 — Separate creation input from device identity and registration outcome

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub implementation
- **Supersedes**: none; extends ADR 0019 and ADR 0020

## Context

US-038 replaces immediate creation with a form. The existing API accepts no
settings and returns a snapshot; desktop chooses the last profile to infer the
created device. A failure reading the snapshot after registry publication can
therefore look like creation failed and encourage a duplicate retry.

## Decision

Use a domain `CreateRequest` containing only display name and supported resource
settings. Validate before acquiring the mutation lock. Allocate ID/port in the
backend under the existing lock; the request has no path, ID, port or image field.
Keep the pinned Google Play ARM64 API 36 image and all stopped/path/space guards.

Expose an explicit `CreatedDevice` with the registered profile, a separately
fallible snapshot and an optional receipt-publication warning. Registry publication
is the boundary: subsequent receipt/snapshot errors cannot turn this result into
an unqualified create failure. Pre-registration failures carry an optional retained
profile identifying the journal/data that may need recovery. Do not delete it or
automatically retry. Recovery journal schema remains unchanged.

Offer a read-only environment observation for the form: SDK/Java component
availability and free bytes on the destination filesystem (nearest existing
ancestor before AVD_HOME exists). It does not execute SDK tools, accept licenses,
install anything or create runtime/AVD/journal files. This is component availability,
not proof the tools will run; the mutation rechecks prerequisites and the 12 GiB
admission floor before creating a journal.

Desktop must submit once, keep input on failure, and start only the returned ID
after registration. Snapshot failure must show a registered-but-refresh-needed
state and prevent submitting the same form again. A failed optional start is a
start failure, never a reason to repeat creation. Initial auto-start is off.

## Consequences

- Domain and infrastructure can validate custom settings without UI or SDK writes.
- Errors after registration remain reviewable without misleading duplicate retries.
- Form observations can become stale; backend guards remain authoritative.
- The legacy `create()` wrapper remains for existing callers while desktop moves
  to the typed API. It must preserve an explicit already-created diagnostic.
- UI, async guards and native QA are required before US-038 is implemented.

## Alternatives considered

Accepting a full Profile would expose identity and port selection to presentation.
Returning only Snapshot would preserve list-order inference. Creating defaults
then saving custom settings would introduce a second mutation and ambiguous
partial success. All three are rejected.

## References

- [US-038](../stories/US-038-create-device-form.md)
- [Architecture](../ARCHITECTURE.md)
