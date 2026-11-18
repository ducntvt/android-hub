# 0043 — Keep explicit display slots and fit the available stage

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub maintainers
- **Supersedes**: none

## Context

Fixed display widths leave unused space in large windows. The first two running
profiles and BTreeMap ordering currently determine the dual view, while pointer
focus and inspector selection can disagree. Changing selection through the old
`choose()` path during pointer-down invalidates the gesture epoch.

## Decision

Keep two ordered, distinct profile IDs in desktop session state independently of
live display sessions. Render stopped/empty slots too. Picking an already visible
device swaps slots; sidebar selection replaces the active slot. A successful
pointer-down activates its existing slot and inspector without changing input
epoch. Changing the pair releases input before session reconciliation. Preserve
unsaved settings and ask the user to save before switching or starting.

Use Iced's `responsive` widget to size the stage from actual layout constraints,
with aspect-preserving fit and zoom relative to fit. Sidebar visibility, pair and
zoom last for this process only. No engine configuration or runtime migration.

## Consequences

- Session order no longer changes when a guest stops or a third guest starts.
- Existing ID/PID/generation guards and serialized backend mutations remain.
- During a mutation, input switching to another device is blocked; existing
  selected-device interaction remains available. No concurrency claim is made.
- Enabling Iced `lazy` adds its six transitive packages to Cargo.lock; no custom
  unsafe layout widget is introduced. Frame delivery rate remains unchanged.
- Controls need a reserved vertical budget; native layout and gesture smoke is
  recorded separately from pure geometry/state tests in US-033/034.
- Full Save/Discard/Cancel navigation and preference persistence are later work.
- App packaging replaces the binary inode atomically so building does not
  truncate a running user's executable. No automatic app restart.

## Alternatives considered

### Keep fixed width and auto-select the first two engines

Simple but does not use available window space or provide stable action targets.

### Estimate viewport from window resize events

Rejected because dynamic toolbar, history, panel and confirmation heights change
the true stage constraints independently of the window's outer size.

### Reuse choose() during pointer-down

Rejected because release_inputs increments input_epoch, invalidating the current
drag/release. Metadata synchronization must not restart the stream or gesture.

## References

- [US-033](../stories/US-033-adaptive-workspace.md)
- [US-034](../stories/US-034-explicit-device-pair.md)
- [UX roadmap](../UX-ROADMAP.md)
