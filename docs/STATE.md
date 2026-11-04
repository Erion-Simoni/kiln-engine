# Kiln Session Handoff

**Active focus:** Task 3 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Prefabs, scenes, and asset workflow. 3D remains paused.

## Task 2 checkpoint

Task 2 implementation and headless acceptance are complete. Final graphical acceptance remains pending: cross-directory Save As and rotate/scale gizmo interaction could not be rerun because this executor has no display, Xvfb, or desktop automation.

## Task 3 progress

- Prefab definitions persist in the serde-defaulted `Project.prefabs` map. Project load hydrates the Prefab Editor, selection-based creation writes the project, and panel edit/delete state syncs before save. Existing manifests load an empty library. ADR-0050 records this ownership.
- Scene instances serialize `Component::PrefabInstance` with a prefab ID and keyed override record. Instantiation uses `EntityCommand`, assigns a project-unique ID, preserves authored components and resource references, and appears in scene history. Duplicates retarget the override record to their new entity ID. Editor scene rebuilds restore runtime prefab links by SourceId; scene snapshots preserve the serialized marker. ADR-0051 records this ownership.
- Prefab library TOML round-trip, old manifest compatibility, and instance instantiate/duplicate/undo/redo/save/reopen/runtime-link coverage pass. Inspector shows linked prefab names and reports missing definitions.
- Functional per-instance override editing/revert and propagation into saved instance data remain incomplete. Drag/drop instantiation, nested-scene workflow, and asset import/rename/move tracking remain pending.

## Validation

- `just check`: passed. `just test`: 539 passed, zero failed/skipped. Focused entity command acceptance: 5 passed.
- Native GUI remains unavailable in this executor (`DISPLAY` unset; Xvfb/xdotool absent). Physical devices, other render tiers, cross-platform runs, browser rerun, and remote CI are unverified for this checkpoint.

## Next

1. Implement functional instance override editing and revert, with history and serialized save/reopen behavior.
2. Propagate prefab definition changes into linked scene instances, then add drag/drop instantiation and acceptance coverage.
3. Complete nested scenes and asset workflows.
4. Return to Task 2's graphical acceptance when a GUI runner is available.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
