# A02 entity tree and combined recovery — 2026-11-15

Branch `fix/a01-ci-baseline`, following `bdab5e4`. A02 remains PARTIAL.

## Implemented behavior

The hierarchy previously displayed scene entities as a flat list. Its parent
indicators checked nested-scene ID coincidences rather than entity Parent links,
and expansion buttons had no action. It now builds a stable parent-first entity
tree, indents children and uses functional, font-independent triangle expanders.
Sibling order follows the source entity order. A bounded iterative traversal also
keeps entities inspectable for cyclic/missing-parent data; it does not repair or
accept invalid authored hierarchy data.

Folding hides descendants without changing selection or the document. Rebuilding
the same scene after edits retains branch state and selected-node flags. Search
includes ancestor context and reveals matching paths even under collapsed branches;
clearing it restores the stored fold state. Scene switches reset branch expansion,
and opening a project clears the old hierarchy view/search/selection.

Dropping an entity onto the SCENE heading now detaches it to the root. Root and
entity drops both use the existing guarded Reparent command, preserving world pose
and scene-scoped Undo/Redo. No new dependency, project format or command architecture.
The numeric widget test driver now terminates xdotool options before typed values,
so negative transforms are entered as text instead of being parsed as options.

## Executed validation

- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  check and standalone audio sample check.
- Hierarchy tests: 5 PASSED, including actual egui selection/reparent widgets,
  out-of-order parent/child/grandchild ordering, fold/search context, selection and
  fold preservation on rebuild, scene isolation and cyclic/missing-parent traversal.
- Entity-command/two-scene/workspace/project-lifecycle integration: 30 PASSED.
- Python regressions: 38 PASSED.
- New real widget `tests/native/hierarchy_tree_smoke.py`: PASSED:
  - Duplicate an actor, move it to (80,100), and set its prospective parent to
    position (40,60), rotation 90°, scale 2. Actual hierarchy drop preserves child
    world pose through local (20,-20), rotation -90°, scale 0.5.
  - Fold the parent with unchanged manifest bytes. Search the child, retain parent
    context, edit child local X to 22 and clear search. Fold state survives edits.
  - Drop the child onto SCENE to obtain root pose (80,104), rotation 0, scale 1.
    Undo/Redo restores exact relationship/properties. Delete/Undo retains identity
    and all authored components. Reparent again through widgets.
  - Copy the parent subtree into a newly created second scene. Parent IDs remap
    correctly; child local properties remain exact.
  - Enter unsaved child local Y=-15. Actual timed autosave captures the draft while
    the manifest remains at -20. Force-kill/reopen/Restore/Save recovers the property
    and copied parent reference; the first scene remains unchanged.
  - Restart and compare the complete saved project, including both scenes and all
    supported properties/references. Inspected folded, searched and recovered views.
- Existing entity clipboard smoke: PASSED same/cross-scene paste, identity/history
  and complete restart. Existing scene-controls smoke: PASSED selection/alignment,
  hide/lock, two-scene crash recovery and restart after this hierarchy change.

CI runs the new workflow and retains its receipt/screenshots/diagnostics. Raw local
receipts: `target/a02-hierarchy-tree`, `target/a02-entity-clipboard`,
`target/a02-scene-controls`; logs: `target/a02-hierarchy-logs`.
This is Linux Xvfb/software-graphics evidence, not new Windows/macOS GUI or device
acceptance. Nested-scene authoring and full component/property/cross-platform
coverage are not established by this entity-parent tree checkpoint.

## Previous-source hosted observation

On `bdab5e4`, Windows candidate 43703292617 and reference games 43703292602 PASSED.
General CI 43703292628 FAILED its browser job with the known graph-authoring
`Render constant parameter dialog before editing` timeout. Linux candidate
43703292634 remained IN_PROGRESS at observation. Public summaries/annotations only;
no job-specific logs/artifact contents were downloaded. This source predates the
entity-tree changes. Matching new-source checks and graph dialog timing repair
remain open; no earlier green is promoted to current-source acceptance.
