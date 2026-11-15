# A02 transform snapping checkpoint — 2026-11-15

Branch `fix/a01-ci-baseline`, following `36d083d`. A02 remains PARTIAL.

The Snap toolbar control previously affected translation only; rotate and scale
ignored the enabled grid. It now also rounds the primary world rotation to 15°
and uniform scale to 0.1 steps. The scale ratio preserves proportions and avoids
collapsing to zero (smallest snapped magnitude 0.1). Disabled, zero, negative or
nonfinite grid settings retain continuous behavior. The toolbar tooltip and
EDITOR_DESIGN.md describe the separate move/rotation/scale increments. No new
package, project format, dependency, or architecture change was introduced.

The existing press-state gesture math, group-transform application, guarded
commands and scene history remain in use. Intermediate pointer frames do not
accumulate rounding; each result is evaluated from the gesture's original state.

## Executed checks

- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  check and standalone audio sample check.
- Gizmo math: 8 PASSED. Includes world-angle snapping from a nonzero rotation,
  position/scale preservation, continuous and invalid-grid behavior, proportional
  scale snapping and a large shrinking drag that cannot collapse the scale.
- Editor transform/property/scene-workspace/two-scene integration: 27 PASSED.
- Native Python regressions: 38 PASSED.
- `python3 tests/native/transform_snap_smoke.py`: PASSED real Linux Xvfb widgets:
  - Toggle actual toolbar Snap; multi-frame rotation saves 45°, scale saves 1.1,
    and move saves (64,96) on the existing 32-unit grid.
  - One Undo/Redo per complete gesture restores exact previous values. Escape
    cancels a scale preview and leaves the preceding gesture as the next undo.
  - Snap off saves continuous scale `1.1 * exp(0.13)`; Undo restores 1.1.
  - Copy transformed player into a real newly created second scene. The first
    scene remains unchanged as parsed data; copied rotation/scale persist.
  - Edit second-scene scale to 1.7 through the Inspector without saving. The real
    autosave timer stores the draft while disk remains at 1.1. Force-kill, reopen,
    Restore and Save recover 1.7; the first scene remains unchanged.
  - Restart and compare the entire saved project, including both scenes and all
    supported properties/references. Inspected recovery screenshot.

Raw receipt/screenshots: `target/a02-transform-snap/`. Logs:
`target/a02-transform-logs/`. CI runs the new widget acceptance and retains its
receipt, failure diagnostics and screenshots. No new hosted result, Windows/macOS
GUI or physical-device acceptance is claimed for this checkpoint.

## Remaining A02 scope

The toolbar's World indicator still describes viewport behavior; local values
are edited through the Inspector. Local-axis viewport tools, broad combined
hierarchy/reparent/duplicate/delete/property editing and full cross-platform
acceptance remain open. This checkpoint does not close A02 or start A03/A04.
