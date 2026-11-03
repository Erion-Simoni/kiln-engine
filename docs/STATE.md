# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Prior baseline and project lifecycle fixes remain: strict native gates, actual WASM/browser smoke, real New/Open/Save As dialogs, staged creation, atomic manifest writes, same-root Save As, and authored-metadata preservation. See the execution log and ADR-0039.
- Added `SceneWorkspace`: explicit active editor scene, open tabs, saved baselines, and independent scene histories. Switching captures outgoing edits, loads the selected document, and clears stale selection/gizmo state. Snapshots, hierarchy edits, save/reload, and undo now target the active scene. ADR-0040 documents the architecture.
- New Scene creates a real document. Saved tabs close without deleting documents and reopen through the Scenes menu with their undo history. Dirty tabs require saving before closure, with a visible message. Successful saves clear dirty baselines; failed writes do not.
- Play captures live authoring edits, starts the project's startup scene, and restores the selected editor scene on Stop. Scene transitions/undo/redo/reload are blocked during Play. Current-scene Play remains future work.
- Fixed the FPS row consuming panel height and hiding tabs. Bounded the entity list; compose scene pixels below the UI so canvas clearing cannot erase menus/dialogs. Use current-frame viewports, discarding hidden-tab viewport state.
- Wired advertised Ctrl/Command+S/Z/Y shortcuts outside text-field editing.
- Entity/prefab creation reserves all live SourceIds, preventing repeated untextured additions from collapsing during snapshots/save. New entities update the hierarchy handle map.
- Added nine multi-scene workspace regressions; prior nine project-file regressions retained.

## Validation

- `just check`: passed after final source changes (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 473 passed, zero failed/skipped.
- `cargo build -p kiln-editor`: passed after final source changes.
- Rebuilt native GUI in Xvfb/software GL/Low tier: visible red/blue scene switching and entity counts, independent entity additions, menu undo/redo, Play startup/Stop restoration, real new scene, dirty-close warning above canvas, saved-tab close/reopen, and editor restart. Final Ctrl+Z/Y/S verification changed entity counts 3 → 2 → 3 and wrote unique IDs to disk; both existing scene names/content and the new scene remained intact. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: passed. Real asset/render/input/startup checks found 392 hero pixels and verified movement/key release. The default sandbox initially blocked the localhost server; the permitted elevated test run passed.
- `git diff --check`: passed.

## Next

1. Continue Task 2's authoritative edit-command model. Finish transform/gizmo/parent/selection behavior and undo coverage, especially current drag recording and component add/remove semantics.
2. Add full project/scene close prompts (save/discard/cancel) and autosave/recovery. Opening another project or exiting still has no unsaved-project guard. Execute the full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2.
3. Keep native/browser gates passing. Tasks 3–9 remain pending; validate real games and exports instead of panel/type existence.

## Known limits and environment

Scene snapshots remain an incremental bridge, not the full shared command model. Undo histories live for this editor session only. Scene reload rebuilds current authoring state, not disk state. Startup scene selection retains the engine's sorted-ID convention; selecting a tab does not change it. Full project-close prompts, recovery, complete nested-scene identity/ownership, full gizmo undo, and parent/local transforms remain unfinished.

Save As stays within the same asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates still provide empty starters; only Empty 2D has been exercised.

Native visual evidence covers software GL/Low tier. Other post-processing tiers, physical GPUs, browser/device breadth, audible audio, complete exports, hardware budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
