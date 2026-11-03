# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Task 1 native/WASM baseline and strict quality gates remain passing locally. Remote CI is unverified.
- Added testable project file operations: create/load directories or manifests, atomic manifest replacement, and same-root Save As. Existing destinations are protected during creation; failed parse/write operations retain prior state. Saving scene edits during Play is blocked.
- Replaced dead Open/Save As behavior with visible path-entry dialogs; fixed New Project's directory/manifest confusion and discarded location edits. Project dialogs render once independently of hub tabs; repeated menu actions no longer spawn duplicate hubs/widget IDs. Errors are visible.
- Reconciled runtime snapshots with original authored metadata to preserve names, Parent, component ordering, nonvisual entities, unresolved/relative resources, and nested references. Other scenes are preserved. See ADR-0039 for the transitional architecture and editor-only tempfile dependency.
- Shared scene loading applies sprite layer/material and attaches tilemaps to existing SourceId entities, avoiding duplicate entities.
- Added nine disk-backed project round-trip regressions, including transform undo/redo and entity add/delete.

## Validation

- `just check`: passed after final code changes (format, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 464 passed, zero failed/skipped.
- `cargo build -p kiln-editor`: passed after final GUI changes.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: passed with shared scene-loading changes; actual pixels, assets, keyboard movement/release, and startup rejection verified.
- Rebuilt native GUI in Xvfb/software GL: New Project created and opened a fresh Empty 2D project; Open reopened a saved manifest; Save As created an identical manifest in the same root. Final dialogs displayed without duplicate-widget diagnostics. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- `git diff --check`: passed.

## Next

1. Continue Task 2: implement an authoritative editable scene/command model, actual scene-tab switching, and per-scene edits with preservation tests.
2. Complete selection/gizmos/parent transforms and shared undo/redo coverage; add dirty indicators, close prompts, autosave/recovery. Execute the full two-scene author/edit/restart acceptance before marking Task 2 complete.
3. Keep native and browser gates passing. Tasks 3–9 remain pending; validate real games and exports instead of relying on panel/type existence.

## Known limits and environment

Save As currently stays within the same asset directory; relocation is explicitly rejected. Dialogs accept typed paths rather than native OS pickers. Platformer/Top-down templates still provide empty starters; only Empty 2D was exercised. Scene snapshots are a bridge, not complete edit-command architecture. Scene tabs and full authoring/undo/recovery acceptance are unfinished.

Browser evidence covers Chromium software WebGL; physical WebGPU, other browsers, touch, complete CLI exports, audible audio, hardware budgets, clean-machine packaging, cross-platform execution, and remote CI remain separate gates. Optional audio/WASM warnings remain outside the strict default-native lint gate. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
