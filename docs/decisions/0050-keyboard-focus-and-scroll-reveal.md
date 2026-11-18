# 0050 — Share keyboard focus traversal across Hub controls

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub desktop
- **Supersedes**: none

## Context

Iced 0.13 text inputs participate in focus operations, but the stock button, checkbox and pick-list controls used here do not provide the required keyboard traversal. Command shortcuts alone do not satisfy US-040. Keyboard focus must not replay an action after its target changes, escape a modal or remain hidden in a scrolled panel.

## Decision

Wrap controls in the desktop layer using Iced's Focusable/Operation protocol. Enabled buttons and checkboxes activate once on Enter/Space release; value choices accept Up/Down with bounded options. Mouse menus and existing update/backend guards stay in place. Track the action identity to cancel armed activation after a rebuild changes its target; toggles and choices retain focus after a value update. A pointer press or window focus loss cancels keyboard activation.

Ignored Tab/Shift-Tab events traverse controls and release Android input first. Android's focused canvas retains its ordinary Tab behavior; Command-F explicitly returns focus to Hub search. Modal confirmation keyboard routing remains unchanged; creation-form traversal is scoped to the modal container. Text inputs continue consuming their normal typing events. Disabled controls are excluded from traversal.

After focus traversal, inspect the focused control's bounds and enclosing scroll containers, then adjust only those Hub scroll offsets that clip the control. No simulated OS pointer, guest scroll/input, new timer or helper process is involved. A black/white double outline distinguishes focus on both theme palettes and selected controls.

Opening the creation form focuses its name input with the standard global focus operation, clearing stale shell focus before scoped traversal. Checkbox padding keeps the outline away from its label/checkmark. Enabled text-input placeholders use the semantic secondary foreground, covered by the same 4.5:1 contrast floor as regular text.

## Consequences

- Keyboard actions reuse the same typed messages and stale-target validation as mouse actions.
- Tab order follows the rendered widget tree; hidden controls are absent, disabled controls skipped.
- Focus wrappers, modal scope and nested-scroll math need regression coverage and native keyboard smoke.
- This does not create a macOS accessibility tree. VoiceOver support remains a separate framework capability audit.
- An optional bounded `ANDROID_HUB_WINDOW_SIZE=WIDTHxHEIGHT` launch setting enables minimum-window QA without changing stored layout or data. The QA packager validates and optionally embeds it; normal defaults and minimum size remain unchanged.

## Alternatives considered

- Global shortcuts only: insufficient for visible control-by-control navigation.
- Emit guest keys for Hub actions: rejected because focus and guest identity must stay separate.
- Replace Iced/framework during this story: outside scope; native accessibility gaps must be recorded independently.

## References

- Iced 0.13 `widget::operation::focusable`, `operation::scope`, `Scrollable` APIs and shipped source.
- [US-040](../stories/US-040-desktop-visual-language-accessibility.md)
