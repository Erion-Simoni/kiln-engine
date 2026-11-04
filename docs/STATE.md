# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Earlier checkpoints cover native/WASM baseline repair, project lifecycle and atomic writes, authored metadata preservation, multi-scene tabs/history, project Save/Discard/Cancel, position commands/gestures, component presence commands, and hierarchy entity commands. ADR-0039–0045 record the architecture.
- Hierarchy create/delete/duplicate/reparent are document-first commands. They preserve authored components, use fresh project-wide IDs, detach children on delete, and reject invalid/cyclic links. egui drag-and-drop now reliably sends reparent commands. Native GUI verified create/selection, duplicate/selection, delete/undo, reparent/save, and saved undo/redo of Parent.
- Added `SetSpriteVisual` to validate and update authored Sprite tint/layer and the runtime Sprite together. It preserves texture/material/animation references and fails atomically for invalid values or ambiguous identities.
- Generalized `TransformTransaction` to `SceneTransaction` and applied it to Inspector edits. Rotation (shown in degrees and saved in radians), uniform scale, tint channels, and layer share widget-scoped undo gestures; separate widget edits form separate history entries. Existing position gestures keep their behavior.
- Three property regressions cover document/runtime parity and metadata preservation, repeated preview history plus disk reopen, and invalid-value atomicity.

## Validation

- `just check`: passed (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 504 passed, zero failed/skipped.
- Focused property suite: 3 passed. Transform edit suite: 9 passed.
- `cargo build -p kiln-editor` and `git diff --check`: passed.
- Native GUI in Xvfb/software GL/Low tier showed and accepted typed tint, rotation, and scale values; Ctrl+Z restored the typed tint. The previous hierarchy acceptance separately verified create/duplicate/delete/reparent and saved Parent undo/redo. Browser smoke was not rerun; remote CI remains unverified.

## Next

1. Add Sprite transform support to the rendering instance/shader so Inspector rotation and scale visibly match gameplay, with renderer-level pixel tests.
2. Finish remaining Sprite properties and history coverage, then add parent/local transform behavior and rotation/scale gizmos.
3. Add multi-selection/framing, dirty scene-tab Save/Discard/Cancel, and autosave/recovery.
4. Run full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

The editor now authors Transform position/rotation/uniform-scale and Sprite tint/layer into scene documents with runtime updates and history. The current `SpriteInstance` and shader still ignore Transform rotation and scale, so those edits are saved and applied to the runtime component but do not yet rotate or resize the drawn sprite. Other Sprite properties, complete multi-selection/gizmos, parent/local transforms, and recovery remain incomplete.

The command model covers transform properties, Sprite tint/layer, component presence, and hierarchy entity edits. Other panels retain transitional snapshot/direct mutation paths. Loader-created default Transforms remain projection-only until authored. Undo is session-only. Scene reload rebuilds authoring state rather than rereading disk; startup selection uses sorted scene IDs. Project guards do not cover external script buffers or forced termination.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised.

Native visuals cover software GL/Low tier. Other devices/tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
