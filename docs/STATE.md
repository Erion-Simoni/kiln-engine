# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Previous checkpoints retain strict native gates, real WASM/browser smoke, actual project dialogs/staged creation/atomic writes, authored metadata preservation, real scene switching, per-scene history, safe saved-tab close/reopen, and unique live entity IDs. See ADR-0039/0040 and the execution log.
- Added `ProjectLifecycle`, a window/GPU-independent guard that compares the full authoring document with the last successfully loaded/saved document. Includes live scene edits, inactive/closed scenes, settings, and serialized resource paths. Menu bar shows unsaved project changes. See ADR-0041.
- Actual Open/Create/recent/welcome requests and both File Exit/window-manager close route through Save/Discard/Cancel. New/Open dialog launch alone remains reversible. Exit uses winit cleanup instead of process::exit.
- Save must finish before the pending transition executes; a failed write leaves the same prompt/action/document available for retry. Discard never writes. Cancel/Escape preserves edits and disk. Failed target loads/creation retain the outgoing project. Repeated requests cannot replace the pending action.
- Pending prompts disable background controls/scene shortcuts/raw gizmo input. Stop Play and restore authoring state before prompting, so simulation mutations are not saved. Cancel returns to editing with Play stopped.
- Added nine disk-backed lifecycle regressions; prior project-file and scene-workspace tests retained.

## Validation

- `just check`: passed after final test changes (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 482 passed, zero failed/skipped.
- `cargo build -p kiln-editor`: passed after final application changes.
- Native GUI in Xvfb/software GL/Low tier: live edit + actual window-manager close, Cancel/Escape, disabled background editing, real save failure and retry, Discard opening another project with unchanged outgoing disk, canceled Create with no destination, File Exit saving before normal shutdown, restart/Play/close restoration, Discard exit, and clean restart/exit without a prompt. Manifest contents were checked on disk. Exit processes returned 0; no panic/GPU validation error. Expected virtual-display/framebuffer warnings remain.
- `git diff --check`: passed.
- Browser was not rerun for editor-only lifecycle changes. Prior checkpoint passed Chromium smoke with real pixels/assets/input/key release/startup rejection. Remote CI remains unverified.

## Next

1. Continue Task 2's authoritative edit-command model. Finish transform/gizmo/parent/selection behavior and undo coverage, especially current drag recording and component add/remove semantics.
2. Add individual dirty scene-tab Save/Discard/Cancel semantics and autosave/recovery. Project replacement/exit now has choice prompts; dirty tabs still require a project save before closing.
3. Execute the full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Keep native/browser gates passing. Tasks 3–9 remain pending.

## Known limits and environment

Scene snapshots remain an incremental bridge, not the full shared command model. Undo histories last only for the editor session. Scene reload rebuilds the current authoring projection rather than rereading disk. Startup scene uses sorted IDs; selecting a tab does not change it. Recovery, complete nested-scene identity/ownership, full gizmo undo, and parent/local transforms remain unfinished. The lifecycle guard covers serialized Project state, not unsaved external script buffers or OS-forced process termination.

Save As stays within the same asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates still provide empty starters; only Empty 2D has been exercised.

Visual validation covers software GL/Low tier. Other post-processing tiers, physical GPUs, browser/device breadth, audible audio, complete exports, hardware budgets, clean-machine packaging, cross-platform runs, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
