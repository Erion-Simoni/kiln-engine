# Kiln Session Handoff

**Active focus:** Task 3 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Prefabs, scenes, and asset workflow. Task 4 has not started; 3D remains paused.

## Task 3 progress

- Prefab definitions persist in the serde-defaulted `Project.prefabs` map. Project load hydrates the Prefab Editor; creation and panel edits/deletes synchronize with the project. ADR-0050 records this ownership.
- Scene instances serialize `Component::PrefabInstance` with prefab ID and overrides. Instantiation uses document-first commands, project-unique IDs, history, and retained resource references. Duplicate retargets the link. Editor rebuild restores runtime links; snapshots preserve them. ADR-0051 records this ownership.
- Inspector supports undoable Revert Overrides and Apply Overrides to Prefab. Apply updates the definition from the selected instance, clears that instance's overrides, propagates to saved instances, and exposes project-scoped undo/redo guarded against intervening project changes.
- Scene snapshots capture supported Transform and Sprite tint/texture differences as overrides; placement offsets are initialized as overrides. Definition changes preserve other instances' overrides and parent links.
- Prefab list entries drag onto the scene canvas. Drops map to world coordinates and use the same validated Instantiate Prefab command as toolbar placement.
- Scene composition rejects direct/indirect cycles. Scene Editor lists nested links and allows offset/additive edits and safe link removal. Add Nested Scene chooses the next available scene in sorted order.
- Asset Browser can move/rename selected images within existing project folders, rejects traversal/symlinks/overwrites, rewrites Sprite texture references in saved scenes and prefab definitions, and reloads the active scene.
- Remaining: stable asset IDs/import settings/resource pickers/cache reimport tracking, texture-clear override representation, broader missing-resource diagnostics, and full GUI/two-level/export acceptance.

## Validation

- `just check` passed; `just test`: 546 passed, zero failed/skipped; `cargo build -p kiln-editor` passed. Focused tests cover asset moves/ref rewrites, Apply to Prefab propagation, overrides, and nested-scene cycles/config/removal.
- GUI verification is unavailable here (`DISPLAY` unset; no Xvfb/xdotool).
- A prior GitHub push attempt failed because the Git proxy was unreachable; retry after committing this checkpoint.

## Next

1. Commit this checkpoint and retry the push.
2. Add import settings/stable asset IDs, resource picking and cache rename/reimport tracking; verify the asset browser end to end.
3. Complete the two-level prefab, asset rename, save/reopen/export acceptance before marking Task 3 done.
4. Only after Task 3 acceptance, begin Task 4 scripting and Play workflow.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
