# Kiln Session Handoff

**Active focus:** Task 3 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Prefabs, scenes, and asset workflow. Task 4 has not started; 3D remains paused.

## Completed in Task 3

- Prefab definitions persist in the serde-defaulted `Project.prefabs` map. Project load hydrates the Prefab Editor; selection-based creation and panel edits/deletes synchronize with the project. ADR-0050 records this ownership.
- Scene instances serialize `Component::PrefabInstance` with prefab ID and override data. Instantiation is document-first, uses project-unique IDs, preserves authored components and resource references, and participates in per-scene history. Duplication retargets the marker. Editor rebuild restores runtime links by SourceId; snapshots preserve markers. ADR-0051 records this ownership.
- Inspector now exposes Revert Overrides for linked entities. It restores prefab components, clears overrides, preserves the instance ID/name, and is undoable.
- Prefab definition edits propagate into saved instances across project scenes while retaining instance ID/name, parent link, and supported overrides. Live ECS instances are also updated by the current editor path.
- Scene snapshots capture Transform and Sprite tint/texture differences into serialized overrides; placement offsets are initialized as overrides. Removing an inherited texture is not representable with the current override schema.
- Tests cover prefab manifest round-trip/legacy compatibility, instance creation/duplicate/undo/redo/save/reopen/runtime-link restoration, reversion/undo, and saved-scene propagation.

## Remaining in Task 3

- Applying an edited instance back to its prefab definition is not implemented. The current override schema cannot represent clearing an inherited texture.
- Prefab instantiation is toolbar-driven; drag/drop is pending.
- Nested-scene editing/loading transitions and asset import settings, stable IDs, pickers, rename/move/reimport tracking, and comprehensive missing-asset diagnostics remain pending.
- Full acceptance still requires reusing an enemy prefab in two levels, editing one instance, updating the definition, renaming its texture, and save/reopen/export verification.

## Validation

- `just check`: passed after this checkpoint's changes.
- Focused entity-command suite: 7 passed. `just test`: 541 passed, zero failed/skipped.
- Native GUI verification is unavailable in this executor (`DISPLAY` unset; Xvfb/xdotool absent). Prior Task 2 graphical acceptance is still pending.

## Next

1. Finish reliable instance override capture and apply-to-definition behavior, with edit/revert history and saved-scene coverage.
2. Add drag/drop prefab placement and complete its acceptance path.
3. Complete nested scenes and asset workflows; run full acceptance before marking Task 3 complete.
4. Only after Task 3 passes its full acceptance, begin Task 4 scripting and Play workflow.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
