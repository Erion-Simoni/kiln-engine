# A02 World/Local move axes — 2026-11-15

Branch `fix/a01-ci-baseline`, following `20ab011`. A02 remains PARTIAL.

## Result

The disabled World indicator is now an Edit-mode World/Local selector. Local move
handles follow the primary entity's world rotation, including its parent chain.
The same projected handle endpoints are used for drawing and hit testing, keeping
screen-space handles usable across zoom, DPI and unequal world-units-per-pixel.
Red selects X and green selects Y. Pressing an axis constrains the entire gesture;
pressing the center or elsewhere retains free planar movement.

World snapping uses absolute scene-grid coordinates, preserving the perpendicular
coordinate for a constrained drag. Local snapping rounds displacement from the
press position in the captured local basis. In both cases the gesture produces
world targets, then uses existing world-to-local conversion, parent-before-child
group application, command validation and scene history. Space/orientation/axis
are captured per gesture; switching space finishes the active edit through the
existing tool-switch path. Rotation and uniform scale retain the shared 2D tools.
Space is transient workspace state; no project-format or dependency change.

## Validation

- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  and standalone audio checks.
- Gizmo tests: 10 PASSED. New checks distinguish rotated Local X/Y from World X,
  constrain off-axis pointer motion, distinguish local displacement snapping from
  world absolute snapping, leave center/off-handle movement free, and ensure the
  displayed handles match hit testing with unequal per-axis projection scales.
- Editor transform/property/workspace/two-scene integration: 28 PASSED, including
  a new rotated/scaled-parent child case. A local-axis snapped world target is
  converted back to stored local values and reprojected exactly, retaining its
  original world rotation and scale.
- Python regressions: 38 PASSED.
- `tests/native/local_axes_smoke.py`: PASSED real widgets on the 1440×900 editor:
  author 90° in Inspector, observe red Local X pointing upward, drag diagonally
  and retain world X=40 while Y advances. Local Y constrains the other coordinate.
  Observe red World X pointing right and retain world Y=60 during a diagonal drag.
  A local snapped drag yields (40,92), while World X snap yields (64,60) from the
  same initial (40,60) and grid 32. Free-center drag, single-step Undo/Redo,
  Escape preserving redo, and exact full-project restart pass. Pixel/world
  conversion uses the fixture's 883×497 viewport, not a 1:1 assumption.
- Existing `tests/native/transform_snap_smoke.py`: PASSED again with the new axes:
  move/rotation/scale snapping, continuous scale opt-out, cancellation/history,
  two-scene clipboard, actual autosave/forced-crash recovery and exact restart.
- Inspected Local and World screenshots. Native CI runs the new widget acceptance
  and retains receipts, diagnostics and screenshots. Raw local evidence:
  `target/a02-local-axes/`, `target/a02-local-logs/`, `target/a02-transform-snap/`.

These are Linux Xvfb/software-graphics results, not new Windows/macOS GUI or
physical-device certification. Broader combined hierarchy/property editing,
multiselection with parented actors through widgets, failed-save and cross-platform
acceptance remain open; this checkpoint does not close A02 or start A03/A04.

## Previous-source hosted observation

On `20ab011`, Windows candidate 43701746095 PASSED. General CI 43701746146 FAILED
native UI and browser jobs; its public annotation reports the previously observed
`Save connected real host-call graph` timeout in graph authoring. Job-specific
logs/artifact contents were not downloaded; no broader root cause is inferred.
Reference games 43701746096 and Linux candidate 43701746100 were still in progress
at observation. This source predates the local-axis changes. A01 acceptance remains
pinned to the explicitly recorded green `6de428b` baseline; prior greens do not
validate newer code. Graph timing repair and matching new-source hosted checks
remain necessary before broader acceptance.
