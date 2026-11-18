# Scrolled viewport drag — native QA, 2026-11-16

Current main binary copied into the isolated QA bundle; codesign verification
passed. QA restarted through native UI; both test engines remained running.

## Single device

Selected QA Lifecycle (hub-1794811024355), scrolled the stage using its scrollbar
until the Android top was clipped, then pressed Hub Home. Drag from (1430,920)
to (1430,460) opened Android's app drawer. The stage stayed scrolled throughout.

## Two devices

Switched to split mode. Left QA Lifecycle stayed in the app drawer. Pressed right
Android3 Home, then dragged from (1680,930) to (1680,480). Right app drawer opened;
left drawer remained unchanged. Sidebar selection still named QA Lifecycle.
This verifies routing to the interacted canvas at split scale and nonzero offset.

All actions used native CUA. Results were inspected in screenshots after actions.
No personal guests or data were modified. QA is left open with two app drawers.

This closes the single/split scrolled-drag smoke for the cursor-coordinate fix.
It does not establish release ordering under saturation, selection changes during
an active hold, or game performance. US-012 stays in_progress for those checks.
