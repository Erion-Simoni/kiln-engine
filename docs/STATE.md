# Kiln Session Handoff

**Active focus:** Task 3 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Prefabs, scenes, and asset workflow. Task 4 has not started; 3D remains paused.

## Task 3 progress

- Prefab definitions persist in the serde-defaulted `Project.prefabs` map. Project load hydrates the Prefab Editor; creation and panel edits/deletes synchronize with the project. ADR-0050 records this ownership.
- Scene instances serialize `Component::PrefabInstance` with prefab ID and overrides. Instantiation uses document-first commands, project-unique IDs, history, and retained resource references. Duplicate retargets the link. Editor rebuild restores runtime links; snapshots preserve them. ADR-0051 records this ownership.
- Inspector supports undoable Revert Overrides. Scene snapshots capture supported Transform and Sprite tint/texture differences as overrides; placement offsets are initialized as overrides. Prefab definition changes propagate into saved scene instances and live ECS instances, preserving instance identity/name and scene parent links.
- Prefab list entries now drag onto the scene canvas. Drops convert pointer coordinates to world coordinates and route through the same validated Instantiate Prefab command as toolbar placement.
- Remaining: apply an instance back to its prefab definition, support clearing inherited texture overrides, nested-scene authoring/loading transitions, asset import settings and stable IDs, resource pickers, rename/move/reimport tracking, and comprehensive missing-asset diagnostics.

## Validation

- Latest code: `just check` passed; focused prefab/entity command suite: 7 passed; pointer mapping unit test passed; `just test`: 542 passed, zero failed/skipped; `cargo build -p kiln-editor` passed.
- Native GUI drag acceptance is unavailable in this executor (`DISPLAY` unset; Xvfb/xdotool absent).
- Previous checkpoint passed `just test` with 541 tests and `cargo build -p kiln-editor`.
- A prior push attempt failed because the GitHub proxy was unreachable; retry after this checkpoint is committed.

## Next

1. Run full workspace tests and editor build for prefab drag-and-drop.
2. Implement apply-instance-to-definition with history-safe behavior, then complete nested scenes and asset workflows.
3. Complete the two-level prefab/texture rename/save/reopen/export acceptance before marking Task 3 done.
4. Only after Task 3 acceptance, begin Task 4 scripting and Play workflow.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
