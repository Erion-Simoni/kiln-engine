# Kiln Session Handoff

**Active focus:** Task 3 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Prefabs, scenes, and asset workflow. 3D remains paused.

## Task 2 checkpoint

Task 2 implementation and headless acceptance are complete. The plan retains final graphical acceptance pending: cross-directory Save As and rotate/scale gizmo interaction could not be rerun because this executor has no display, Xvfb, or desktop automation.

## Task 3 progress

- Added `Project.prefabs`, keyed by stable prefab ID, with serde default for old project files. Project loading hydrates the Prefab Editor, selection-based creation stores definitions in the project, and panel edits/deletes sync back before saving. ADR-0050 records this data ownership.
- Prefab definition TOML round-trip and legacy empty-library loading pass. Prefab Editor selection remains valid only while its selected definition exists after project replacement.
- Scene prefab instances and overrides are still runtime-only; drag/drop instantiation and functional apply/revert/definition propagation across saved scenes remain pending.

## Validation

- Focused checks: `cargo nextest run -p kiln-core --lib` passed 50 tests; `cargo nextest run -p kiln-editor --test project_roundtrip` passed 10 tests.
- Full validation: `just check` passed; `just test` passed 538 tests; `cargo build -p kiln-editor` passed. Native GUI cannot be exercised in this executor (`DISPLAY` unset; Xvfb/xdotool absent).

## Next

1. Persist prefab instance identity and overrides in authored scene data so instances survive save/reopen.
2. Implement functional prefab override editing, revert, and definition propagation; then add drag/drop instantiation and acceptance tests.
3. Return to Task 2's graphical acceptance when a GUI runner is available.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
