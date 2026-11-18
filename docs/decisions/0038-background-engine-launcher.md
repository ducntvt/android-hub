# ADR 0038 — Engine launcher as a background agent

- Status: Accepted
- Date: 2026-11-16

Per-device engine processes remain independent of Hub. Retain the existing Launch Services launch mechanism, but generate `runtime/launchers/Android-{port}.app` with `LSUIElement=true` and launch using `open -g` so the helper does not request foreground focus. Apple documents LSUIElement as an agent app that does not appear in the Dock: https://developer.apple.com/documentation/bundleresources/information-property-list/lsuielement

Use `local.androidhub.engine.{id}` instead of the prior helper identity, avoiding reuse of the old foreground bundle registration. Keep `-no-window`, authenticated local display transport, profile/port isolation and explicit stop behavior unchanged. The main Hub bundle remains a normal foreground app.

Do not rewrite/delete a running old launcher or restart an existing guest. New helpers take effect on the next start from the updated Hub. Old helper directories may remain until a separate safe cleanup; they are not guest data and are no longer used by the new start path.

## Invocation update

ADR 0042 replaces LaunchServices open with a child process of Hub after a confirmed Desktop-access denial from the separate helper identity. Generated headless flags remain unchanged.
