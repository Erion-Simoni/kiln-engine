# A02 parented selection and group transforms — 2026-11-15

Branch `fix/a01-ci-baseline`, following `c9958d7`. A02 remains PARTIAL.

## Reproduced defects and fixes

Ctrl-clicking the last selected hierarchy row cleared the hierarchy selection,
but the app retained the previous inspector/gizmo target. The new real-widget
regression FAILED before the fix: `Empty hierarchy selection still edited the
old gizmo target`. Selection synchronization now propagates an empty selection.

Scaling a selected parent and child then exposed needless child-local float
changes: saved local (20, -20) became (19.999998092651367,
-20.000001907348633). Shared move/rotate/uniform-scale operations now transform
only selected roots. Selected descendants inherit the operation, preserving their
complete local documents. Ancestor lookup includes unselected intermediate
parents and is bounded for malformed cycles. General target ordering is unchanged.
No new dependency, project format or command architecture.

## Executed validation

- `just check` PASSED: formatting, strict workspace/all-target Clippy,
  workspace and standalone audio checks.
- Targeted gizmo math tests: 11 PASSED, including selected ancestors through an
  unselected intermediate parent and a child selected alone.
- Transform/entity-command/scene-workspace/two-scene integration: 35 PASSED,
  including exact child document preservation and expected inherited world pose.
- Python regressions: 38 PASSED.
- `parented_multiselect_smoke.py` PASSED through real native widgets: empty
  selection cannot edit; duplicate and parent under a rotated/scaled actor;
  descendant-first additive selection; snapped group move, rotation and scale;
  exact child properties; each multiframe gesture one undo; redo restores the
  complete scene; selection survives history without reselection; Escape restores
  the draft and retains prior redo; exact complete-project restart.
- Existing `local_axes_smoke.py` PASSED again, covering visible World/Local axes,
  constraints, snapping, free movement, undo/cancel and exact project restart.
- Inspected screenshots showing no selected entity/gizmo and both parent/child
  selected with parent scale 2.30. CI runs the regression and retains receipts,
  screenshots and editor logs.

Raw evidence: `target/a02-parented-selection/`,
`target/a02-parented-selection-logs/`, `target/a02-local-axes/`.
The initial empty-selection failure is in `before.log`; the intermediate scale
failure saved fixture is `run-olug2eqz/project/kiln.toml` under the selection
output directory. Results are Linux Xvfb/software graphics only. Full property
coverage, cross-platform GUI/device acceptance and nested-scene authoring remain
open. Inspector property edits retain their existing primary-entity semantics.

## Hosted source boundary

At observation on prior source `c9958d7`, general CI 43709956368, reference games
43709956415, Windows candidate 43709956361 and Linux candidate 43709956395 were
IN_PROGRESS. These observations do not accept this change. Earlier source
`ac0a86c` Linux candidate 43708624752 failed artifact-probe download before the
application build; no service root cause was established or preflight weakened.
Matching hosted source results remain required.
