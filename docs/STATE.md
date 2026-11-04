# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Earlier checkpoints cover native/WASM baseline repair, project lifecycle and atomic writes, authored metadata preservation, multi-scene tabs/history, project Save/Discard/Cancel, transform/property commands, component presence commands, and hierarchy entity commands. ADR-0039–0048 record the architecture.
- Hierarchy create/delete/duplicate/reparent are document-first commands with ID validation, cycle prevention, metadata preservation, and undo. Native GUI verified create/selection, duplicate/selection, delete/undo, reparent/save, and saved undo/redo of Parent.
- Inspector edits for Transform position/rotation/uniform-scale and Sprite tint/layer/dimensions update authored and runtime state with one undo step per field gesture. Sprite dimensions default to 32×32 in old projects.
- Sprite rendering supports entity rotation/scale and authored dimensions. Parent links compose local translation/rotation/scale into runtime `GlobalTransform`; editor/player/web/mobile/sample renderers consume refreshed global poses. Viewport drags convert to local space; reparent/delete preserve child world poses and nested scene offsets survive propagation.
- Scene viewport now has visible move axes, a rotation ring, and a scale handle. Drag gestures preview local Transform edits, commit one undo step on release, and cancel through the existing Escape path. Rotation angle wrap, uniform scaling, hit targets, and translation snap math have unit coverage.
- Hierarchy and scene-list selection now support Ctrl/Cmd additive toggling while preserving a primary inspector target. The scene view's Frame Selection button and `F` shortcut center and fit selected entity positions.
- All gizmo modes now apply to multi-selection around the primary entity: translation shares a world-space offset, rotation turns positions and orientations around the pivot, and scale changes offsets and sizes. Each resulting world transform converts back to local space, and the gesture is one undo transaction.

## Validation

- Parent/local checkpoint: `just check` passed; `just test`: 514 passed. Editor build, WASM check, Chromium smoke, nested-offset, runtime propagation, editor descendant update, and reparent/delete pose-preservation checks passed.
- Current gizmo checkpoint: `just check` passed; `just test`: 518 passed, zero failed/skipped. `cargo build -p kiln-editor` passed.
- Multi-selection/framing checkpoint: `just check` passed; `just test`: 519 passed, zero failed/skipped. Additive selection state has a unit test.
- Group-translation checkpoint: `just check` passed; `just test`: 520 passed, zero failed/skipped. A regression test moves a selected parent and child together while preserving world spacing. Runtime GUI drag behavior remains to be exercised in the native editor.
- Group rotation/scale checkpoint: `just check` passed; `just test`: 522 passed, zero failed/skipped. Unit and integration tests cover pivot behavior and parented world targets.
- Native app GUI confirmation of parented transforms and gizmo dragging remains outstanding. Physical devices, other render tiers, cross-platform runners, and remote CI are unverified.

## Next

1. Add multi-entity rotation/scale and frame by authored sprite bounds.
2. Add dirty scene-tab Save/Discard/Cancel and autosave/recovery.
3. Run full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

`Transform` is local; call `kiln_runtime::update_global_transforms` after direct gameplay changes when a world-space read is needed. Built-in sprite render loops refresh `GlobalTransform` before drawing. Physics systems still consume local Transform and do not yet account for parent relationships; tilemap cell instances retain identity transforms. The command model covers transform properties, Sprite visuals/size, component presence, and hierarchy entities; other panels retain transitional direct-mutation paths. Undo is session-only; crash recovery is absent. Native visuals cover software GL/Low tier only.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised. Other render tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, physical devices, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
