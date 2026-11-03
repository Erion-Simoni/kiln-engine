# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Retained prior baseline repairs, native/browser gates, real project dialogs/atomic writes, metadata preservation, scene switching and independent histories, unique IDs, and project Save/Discard/Cancel protection. See ADR-0039/0040/0041 and the plan's execution log.
- Added shared validated SetTransform commands for the position inspector and viewport translation. The authored root document and runtime projection update together by scene/entity IDs; unrelated data remains intact. Invalid targets/values and ambiguous identities fail before mutation. Editing a metadata-only entity's default projection authors its Transform. See ADR-0042.
- Added gesture transactions for typed/mouse inspector edits and canvas translation, recording one undo step on completion. Inspector fields have separate gesture ownership. No-op gestures preserve redo; Escape cancels viewport translation without a history entry.
- Pending position edits finish before scene/history/project transitions, save, Play, focus loss, and explicit authoring operations. Undo/redo/cancel reloads preserve selection by remapping SourceId to the rebuilt runtime handle.
- Replaced raw window-coordinate dragging with egui canvas interaction. Use the letterboxed viewport, DPI, zoom, initial transform, and total displacement; held pointers no longer drift. World grid snapping works; outside releases finish; letterbox margins cannot start a move. Rendering and input share the effective camera.
- Added nine transform/viewport regressions, retaining prior project, lifecycle, and scene-workspace coverage.

## Validation

- `just test`: 491 passed, zero failed/skipped after final functional changes.
- `just check`: passed after final functional changes and again after public-item documentation comments (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `cargo build -p kiln-editor`: passed after final functional changes.
- Native GUI in Xvfb/software GL/Low tier: typed X edit/undo/redo, multi-frame canvas drag with no stationary drift and one history step, Escape cancellation, snapped save at (576, 288), release outside the canvas without a stuck gesture, margin-started drag ignored, inspector mouse-drag undo, scene switching/history retention, disk property/name/count verification, real restart, independent Y-then-X typed undo, clean shutdown. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- `git diff --check`: passed.
- Browser was not rerun for editor-only changes. Previous Chromium smoke passed real pixels/assets/input/release/startup rejection. Remote CI remains unverified.

## Next

1. Continue Task 2's authoritative edit-command model for component add/remove, entity create/delete/duplicate, and hierarchy reparenting; preserve existing properties and references with real undo/redo/save/reopen regressions.
2. Finish parent/local transforms, rotation/scale authoring and rendering parity, multi-selection/framing, and full gizmo behavior. Only selected-entity translation and inspector position edits were completed here.
3. Add individual dirty scene-tab Save/Discard/Cancel semantics and autosave/recovery. Project replacement/exit already has choice prompts; dirty tabs still require a project save before closing.
4. Execute the full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

Transform commands begin the shared model; remaining panels still use transitional scene snapshots/direct mutations. Undo histories last only for the session. Scene reload rebuilds authoring state rather than rereading disk. Startup scene uses sorted IDs; selecting a tab does not change it. Selected-entity translation uses the canvas; direct sprite picking, complete gizmo handles, rotation/scale gizmos, parent/local transforms, complete nested-scene identity/ownership, and recovery remain unfinished. The project guard covers serialized Project state, not external unsaved script buffers or forced process termination.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised.

Native visuals cover software GL/Low tier. Headless tests cover DPI/zoom/letterbox calculations; physical high-DPI devices and nondefault camera controls were not exercised natively. Other tiers/GPUs, browser/device breadth, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
