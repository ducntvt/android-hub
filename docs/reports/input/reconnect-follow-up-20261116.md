# Remaining reconnect review — 2026-11-16

Clipboard callbacks now capture input generation and validate under enqueue lock.

Follow-up investigation: canvas State.down/last are reset by desktop epoch and
profile ID, but a same-PID transport reconnect does not necessarily change either.
Frame/error notifications coalesce, so a reconnect could finish before the desktop
observes the error. Verify whether an active old drag can continue after ready and
whether canvas input should carry transport generation as clipboard now does.
This is a source-level concern, not a reproduced native failure.

Unrelated observed test failure: SDK assembly immediate retry once reported its
install lock busy during the parallel workspace suite. Subsequent full runs passed.
Inspect lock ownership/lifetime and concurrent process spawning before treating it
as merely test noise; preserve original clipboard report test log.

## Implemented follow-up

The canvas concern above is now guarded: optional transport generation resets
state and is attached to outgoing input, then checked atomically by the sender.
Regression covers same-ID/epoch reconnect without an intermediate error frame,
and rejects delayed old-generation touch/key/wheel at the ready queue. 110 tests
pass; native interruption remains unverified. Main bundle updated on disk.
