# 0036 — Persistent framework scroll bridge

- **Status**: Accepted — API 36 personal build; validation limits below
- **Date**: 2026-11-16
- **Deciders**: Codex

## Context

ADR 0035 rejected emulator mouse/tablet configurations: they either lose wheel delivery or target the center rather than the host pointer. Touch must remain available for games. A shell command per wheel adds process overhead.

## Decision

Keep the original multitouch engine configuration and existing authenticated gRPC touch/key/clipboard transport. For wheel only, lazily start a small bundled Java DEX via adb shell app_process. It injects framework ACTION_SCROLL with explicit guest coordinates and both axes, without synthesizing a touch or swipe. Source lives in infrastructure/android; scripts/build-scroll-bridge.sh rebuilds the bundled DEX with the local Android SDK.

One persistent stdin/stdout channel belongs to one display session. Setup pins adb transport ID, checks guest boot identity inside the helper, and checks the expected engine PID before/after startup. Sequence acknowledgements confirm framework acceptance, not visual application behavior. No automatic replay after a failed send. Startup has an overall eight-second deadline; individual sends have a two-second deadline. Session cancellation drops the child and closes the guest channel.

Frame reception and serial input processing are separate async futures, so startup and acknowledgements do not suspend frame consumption. Existing bounded queue and generation/release gates remain. No additional network listener, root access, system-image modification, or backend server.

The DEX is content-addressed, cached locally, and uploaded to guest /data/local/tmp only when missing or different. It is read-only before loading. This small runtime dependency is intentional and must not be removed by generic cache cleanup while a session is being established.

## Validation required

Native QA must establish API 36 reflection/injection compatibility, exact target routing in nested scroll areas, both axes, natural direction, two guests, touch interaction, reconnect and helper teardown. Unit tests and compilation alone do not accept this architecture. Hidden framework APIs require revalidation when changing Android versions.

## Alternatives

Per-event adb input incurs process startup for every event. Fake swipes change gesture semantics. Engine pointer alternatives failed native targeting checks (ADR 0035).

## Cache publication follow-up

The first implementation created the final cache pathname before completing its
write. A concurrent session could see incomplete bytes; an interrupted write could
leave a permanently invalid cache. Cache publication now writes/syncs a unique
0600 staging file and atomically renames it into place. Concurrent writers publish
identical bundled bytes. Failed writes retain the previously published file and
clean their own staging file. Invalid regular cache files are replaceable; symlink
and non-file cache entries are rejected. Tests cover eight simultaneous callers,
partial cache repair, symlink preservation, and injected mid-write failure.

Upload staging paths also use PID/timestamp/sequence, so cancellation after making
a staging DEX read-only cannot block a later attempt by reusing the same filename.
An interrupted upload may leave a small guest staging file; no broad deletion is
performed. Normal helper teardown and latest QA two-guest wheel routing pass;
physical horizontal trackpad and forced in-flight failure remain unverified.

## Acceptance evidence and limits

Native Settings scroll down/up, touch afterward, exact upper/lower target coordinates,
two-guest isolation, outside-canvas routing, normal reconnect and helper teardown
passed. The user confirmed horizontal two-finger trackpad movement changes the H
counter in the fixture. This supports selecting this transport for the API 36 personal
build. It does not claim exhaustive Android widget compatibility, forced in-flight
failure recovery, all natural-scroll preferences, or two-game performance. These
remaining checks stay tracked in US-011/US-012 and the native report.
