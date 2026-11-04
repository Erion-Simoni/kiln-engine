# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Earlier checkpoints cover native/WASM baseline repair, project lifecycle and atomic writes, authored metadata preservation, multi-scene tabs/history, project Save/Discard/Cancel, transform/property commands, component presence commands, and hierarchy entity commands. ADR-0039–0048 record the architecture.
- Hierarchy create/delete/duplicate/reparent are document-first commands with ID validation, cycle prevention, metadata preservation, and undo. Native GUI verified create/selection, duplicate/selection, delete/undo, reparent/save, and saved undo/redo of Parent.
- Inspector edits for Transform position/rotation/uniform-scale and Sprite tint/layer/dimensions update authored and runtime state with one undo step per field gesture. Sprite dimensions default to 32×32 in old projects.
- Sprite rendering supports entity rotation/scale; authored width/height are carried through loading, inspection, and save/reopen.
- Parent links now compose local translation/rotation/scale into runtime `GlobalTransform`. All editor/player/web/mobile/sample sprite renderers consume global transforms; parent motion is propagated before drawing. Inspector values remain local, viewport drags convert world poses back to local, and reparent/delete preserve child world poses. Nested-scene offsets remain separate from local values.

## Validation

- Previous Sprite dimensions checkpoint: `just check` passed; `just test`: 507 passed; editor build, WASM check, and Chromium browser smoke passed.
- Current parent/local checkpoint: `just check` passed; `just test`: 514 passed, zero failed/skipped. Added checks cover nested affine composition, inverse world-to-local conversion, runtime scene spawn and live parent movement, nested scene offsets through propagation, editor descendant projection, and world-pose preservation on reparent/delete. Editor build, WASM check, and Chromium smoke passed.
- Native app GUI confirmation of parented transforms remains outstanding. Physical devices, other render tiers, cross-platform runners, and remote CI are unverified.

## Next

1. Add rotation/scale viewport gizmos, then multi-selection and framing.
2. Add dirty scene-tab Save/Discard/Cancel and autosave/recovery.
3. Run full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

`Transform` is local; call `kiln_runtime::update_global_transforms` after direct gameplay changes when a world-space read is needed. Built-in sprite render loops refresh `GlobalTransform` before drawing. Physics systems still consume local Transform and do not yet account for parent relationships; tilemap cell instances retain identity transforms. The command model covers transform properties, Sprite visuals/size, component presence, and hierarchy entities; other panels retain transitional direct-mutation paths. Undo is session-only; crash recovery is absent. Native visuals cover software GL/Low tier only.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised. Other render tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, physical devices, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
