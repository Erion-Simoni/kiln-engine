# Kiln Session Handoff

**Active session:** Character scripting workflow and stable status bar, 2026-11-12.
**Branch:** codex/editor-polish, following editor polish commit ec03286.
**Boundary:** One focused editor workflow task. Task 8 distribution review remains separate; no additional 3D milestone.

## Done

- Removed the bottom-bar FPS text and its unused per-frame rate bookkeeping. Profiler tooling remains available.
- Added Script Editor as a dock tab, default workspace tab, View command and Quick Actions result. Double-click .ks assets or use Inspector Open Script to edit a resource.
- Added Inspector Character Script guidance and Create Movement Script. Creates a unique project-relative resource without overwriting files, adds the existing Script component through undoable scene history, supplies speed=180 and opens the source. Requires a selected entity with Transform; Play/lifecycle editing locks apply.
- Native multiline code editing, Validate, Save Script, Ctrl/Cmd+S while editing, Discard Edits, Reload from Disk and dirty tab/status indicators. Shared runtime compiler resolves imports and reports source locations. Save validates first and writes atomically, refusing external changes.
- Dirty source is retained when its tab closes and blocks other source opens, project replacement/exit, Save As, export and Play until explicitly saved/discarded. Loading a project or successful Save As clears the clean buffer to prevent cross-project writes. Source is bounded; resource paths must stay within the project. No new dependencies or format changes (ADR-0074).
- Rebuilt and reopened the updated native editor. Windows inventory reports Kiln Editor - Game Engine after launch. Updated EDITOR_DESIGN.md and KILNSCRIPT.md with the workflow.

## Validation

- All 73 editor library tests passed, including new movement execution, non-overwriting creation, compiler/import diagnostics, atomic save, external-change and dirty-buffer tests. The generated attached script moved an actual runtime entity from X=10 to X=28 with speed=180 and dt=0.1, preserving Y.
- Full just check recipe passed: fmt, strict workspace/all-target Clippy, workspace check and separate audio sample check. just is not installed, so exact underlying commands were run directly. Final checks and locked editor build passed after the final error-display change.
- Existing toolbar regression still covers real Play clicks at widths 640/800/1024/1440. Git diff whitespace check passed.

## Next

- In the open editor: select a character, Inspector -> Create Movement Script, adjust speed, edit/Validate/Save Script, save the project, then Play and focus Game for arrow/WASD movement.
- Visual acceptance on this PC remains manual. Existing pending release/platform and physical-device work remains under Task 8; continue only when requested.

## Known issues / limitations

- Desktop screenshot capture timed out again after launch. The native window is inventoried but no screenshot-based UI acceptance is claimed. The close request for the previous editor reported an activation error; a fresh window inventory confirmed it closed before rebuilding.
- Starter movement directly edits Transform. It does not implement physics collision sliding, jumping or a character controller. Source changes run on the next Play, with no hot reload or instruction debugger.
- Scene undo removes script attachments but retains generated source files. Source edits have text undo separate from scene history. Invalid source must be corrected or explicitly discarded before Save/exit; external edits require reconciliation or discard/reload.
- Prior Intel Vulkan driver warning remains; no driver settings changed. Previous local app.rs edit remains in stash Before latest master editor launch 2026-11-12 on fix/dependency-migration.
- Task 8 release/signing/provenance, macOS and physical-device limitations are unchanged. No remote push or release performed.
