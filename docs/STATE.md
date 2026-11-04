# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Prior checkpoints retain native/WASM baseline repairs, strict local gates, real project workflows/atomic writes, authored metadata preservation, scene switching and independent histories, unique IDs, project transition Save/Discard/Cancel, and shared position commands with gesture undo. See ADR-0039–0042 and the execution log.
- Added validated authored component commands for Transform, Sprite, Role, and Tilemap (ADR-0043). Inspector actions queue outside dock rendering, record one actual change, rebuild the runtime projection, and remap selection by SourceId. Invalid/inactive/ambiguous targets fail before mutation; no-op add/remove preserves redo and existing properties.
- Component presence in Inspector comes from the authored document, distinguishing implicit runtime transforms and unresolved map references. Add Transform works on metadata-only entities; removing authored Transform survives save/reopen. Missing tilemaps have a visible warning and explicit removal. Adding Tilemap requires selecting an actual project map; empty/unknown resources are rejected.
- Role and Tilemap have remove controls. Root entity list includes nonvisual objects, authored names, and stable document order. A Sprite-less entity remains reachable after restarting the editor.
- Added eight integration regressions exercising component commands, actual scene projection, history, disk persistence, defaults, unresolved references, no-ops/validation, and metadata/identity preservation.

## Validation

- `just check`: passed after final code changes (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 499 passed, zero failed/skipped.
- `cargo build -p kiln-editor`: passed after the final entity-ordering change.
- Native GUI in Xvfb/software GL/Low tier: nonvisual selection; authored Transform add/save/undo/redo; Role remove/add and undo restoring its original value; unresolved Tilemap warning/remove/save/undo/redo; selecting an existing Tilemap; Sprite remove retaining selection, undo restoring original red/layer 7 metadata, redo/save and restart; nonvisual reselection/re-add with fresh defaults; authored Transform remove/save/undo/redo and restart with Add Transform still available. Disk contents/names checked, process exits normal. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- `git diff --check`: passed.
- Browser was not rerun for editor-only changes; prior Chromium smoke passed pixels/assets/input/release/startup rejection. Remote CI remains unverified.

## Next

1. Continue Task 2's shared edit-command model for entity create/delete/duplicate and hierarchy reparenting, preserving IDs, names, components, parent/nested references, and real undo/redo/save/reopen behavior.
2. Complete property-edit command/undo coverage beyond position/component presence (including current Sprite color typing), parent/local transforms, rotation/scale rendering/authoring parity, multi-selection/framing, and full gizmo behavior.
3. Add individual dirty scene-tab Save/Discard/Cancel semantics and autosave/recovery. Project replacement/exit already has choice prompts; dirty tabs still require a project save before closing.
4. Execute full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

The command model now covers position and inspector component presence; other panels still contain transitional snapshots/direct mutations. Runtime-only component helpers remain available for their existing callers/tests. Loader projections still supply implicit default transforms, which are now distinguished from authored components. Role value editing, map-resource creation/import, complete property editing, entity/hierarchy commands, and recovery are not completed by this checkpoint.

Undo histories last only for the session. Scene reload rebuilds authoring state rather than rereading disk. Startup scene uses sorted IDs; choosing a tab does not change it. Direct sprite picking, complete gizmo handles, rotation/scale gizmos, parent/local transforms, and complete nested-scene identity/ownership remain unfinished. Project guards cover serialized Project state, not unsaved external script buffers or forced termination.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised.

Native visuals cover software GL/Low tier; headless tests cover DPI/zoom/letterbox formulas. Other devices/tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
