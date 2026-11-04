# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Prior checkpoints retain native/WASM baseline repairs, strict local gates, project lifecycle and atomic writes, authored metadata preservation, scene switching and independent histories, unique IDs, project Save/Discard/Cancel, position commands/gesture undo, authored component commands, and hierarchy entity commands. See ADR-0039–0044 and the execution log.
- Added document-first hierarchy commands for create, delete, duplicate, and reparent. Commands validate the active scene and entity identities, preserve full authored components and metadata, allocate IDs unique across project scenes, detach children when deleting a parent, and reject missing links/cycles. The editor queues hierarchy commands after dock rendering and rebuilds runtime state from the updated document. Duplicate preserves authored component values and the existing parent relationship.
- Replaced the hierarchy’s click-only drag source with egui’s drag-and-drop payload API after GUI testing showed it never began a drag. Drops now reach the validated reparent command. Added a positive reparent regression alongside create/duplicate/delete, ID collision, full component copying, child detachment, cycle rejection, no-op history, save/reopen, and undo tests.

## Validation

- `just check`: passed (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 501 passed, zero failed/skipped.
- Focused `cargo test -p kiln-editor --test entity_command`: 2 passed.
- `git diff --check`: passed.
- Native GUI in Xvfb/software GL/Low tier exercised hierarchy create and selection, duplicate and selection, delete and Ctrl+Z restore, drag reparent, save, then Ctrl+Z/save and Ctrl+Y/save. The authored Parent link was absent after saved undo and restored after saved redo. The editor started and exited normally; expected Xvfb/linear framebuffer warnings remain. Browser smoke was not rerun; remote CI remains unverified.

## Next

1. Expand property-edit command and undo coverage beyond position/component presence, starting with Sprite color typing and one-step gesture history.
2. Extend command/undo coverage beyond Sprite color, including parent/local transforms, rotation/scale rendering/authoring parity, multi-selection/framing, and full gizmo behavior.
3. Add individual dirty scene-tab Save/Discard/Cancel semantics and autosave/recovery. Project replacement/exit already has choice prompts; dirty-tab closure still requires a project save before closing.
4. Execute full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

The command model now covers position, inspector component presence, and hierarchy entity operations; other panels still contain transitional snapshots/direct mutations. Runtime-only component helpers remain for their existing callers/tests. Loader projections supply implicit default transforms distinguished from authored components. Role value editing, map-resource creation/import, complete property editing, rotation/scale parity, multi-selection/framing, and recovery are not complete.

Hierarchy commands mutate the authored scene and rebuild the runtime projection. Delete detaches direct children to root; duplicate retains the same parent link. These semantics are covered by headless command tests. Native GUI behavior was manually exercised with a disposable fixture, including saved undo/redo of a Parent link.

Undo histories last only for the session. Scene reload rebuilds authoring state rather than rereading disk. Startup scene uses sorted IDs; choosing a tab does not change it. Direct sprite picking, complete gizmo handles, rotation/scale gizmos, parent/local transforms, and complete nested-scene identity/ownership remain unfinished. Project guards cover serialized Project state, not unsaved external script buffers or forced termination.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised.

Native visuals previously covered software GL/Low tier; headless tests cover DPI/zoom/letterbox formulas. Other devices/tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
