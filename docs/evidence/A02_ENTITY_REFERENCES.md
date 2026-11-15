# A02 reference-safe deletion and joint editing — 2026-11-15

Branch `fix/a01-ci-baseline`, following `bf206f9`. A02 remains PARTIAL.

## Reproduced defects and repair

Deleting entities detached children but left joint endpoints and camera follow
targets dangling. Joint component commands validated numeric geometry but accepted
missing or ambiguous endpoint identities. Three new integration regressions FAILED
before the fix: deletion with a joint/camera dependency, containing-scene dependency
checks, and joint endpoint validation.

A shared editor helper now resolves the active edited scene and nested composition.
Add/Set Joint require each endpoint to identify exactly one entity in that scope.
Deletion checks containing compositions for joint and camera dependents and refuses
before changing the staged document/history. Joints owned by the deleted entity
are removed with it and do not block deletion. Independent scenes can reuse IDs
without creating a dependency. Traversal is iterative and rejects repeated/missing
nested links rather than guessing an instance.

The inspector retains a joint form draft so typing intermediate endpoint strings
cannot publish invalid references or interrupt entry. Apply Joint validates and
commits all fields together as one history entry. Invalid Apply retains fields and
an inline error; Cancel discards the form without changing the project or redo.
Both endpoint inputs and numeric controls fit/wrap in the narrow inspector.
Unapplied form input is not a project edit: Apply is required before save/recovery.
No dependency, manifest migration, implicit dependency deletion or new undo stack.

Generic project errors now use a centered foreground window. An actual layer/hit
regression FAILED before the foreground change when a large floating editor was
raised, and PASSED after it. The real replay observes error text before saving
because a successful Save intentionally clears the prior operation error.

## Executed validation

- `just check` PASSED formatting, strict workspace/all-target Clippy and
  workspace/standalone audio checks.
- Full `cargo test --workspace --locked`: 777 PASSED, zero failures, including
  doc tests. Python regressions: 40 PASSED. Script compilation/diff checks pass.
- New integration coverage: missing/ambiguous endpoints fail atomically; a unique
  nested endpoint works; rejected deletes preserve entire project and existing
  redo; containing-scene cameras/joints protect a child entity; unrelated scenes
  remain independent; resolving references permits a normal undoable deletion.
- `entity_reference_smoke.py` PASSED actual widgets: attach joint; invalid endpoint
  Apply retains draft/document; correction plus distance Apply is one Undo;
  Cancel retains prior redo; joint and camera deletion guards produce specific
  diagnostics and visible foreground errors; remove joint/change camera through
  widgets; delete/Undo/Redo and exact complete-project restart. Screenshots inspected.
- `scene_settings_smoke.py` PASSED again: settings/camera/history, valid composition
  preview/removal/Undo, real-timer autosave, forced crash recovery and exact restart.
- Existing `local_axes_smoke.py` PASSED in an isolated serial rerun on the new
  source. This does not establish the cause/resolution of the prior hosted failure.
- The initial replay setup exposed an invalid self-joint test value, an inaccurate
  menu-row coordinate and a selected-row color in its popup observer. Those attempts
  were retained; the final observer uses measured popup background and actual choice
  positions. No command/history/data assertions were weakened.

CI runs the new widget workflow and retains receipts/screenshots/editor logs.
Raw evidence: `target/a02-entity-references/`,
`target/a02-entity-references-layout/`, `target/a02-entity-references-logs/`.
Linux Xvfb/software graphics only. Full combined all-property acceptance, scoped
repeated scene instances and cross-platform GUI/device acceptance remain open.

## Hosted source boundary

Prior `bf206f9`: Windows 43723198796 and reference games 43723198790 PASSED;
CI 43723198838 FAILED in `local_axes_smoke.py` with
`Saved constrained position (40.0, 60.0) timed out`. Linux candidate 43723198819
was IN_PROGRESS at observation. This is a retained GUI failure, not accepted source
or an established service root cause. No local-axis assertion was weakened.
Prior `9bc2ace`: CI 43722214097 and Windows 43722214082 PASSED; reference/Linux
were CANCELLED. None of those results accept the reference-deletion change.
Matching new-source hosted checks remain required.
