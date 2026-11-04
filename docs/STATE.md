# Kiln Session Handoff

**Active focus:** Task 2 implementation in `docs/2D_COMPLETION_PLAN.md` is complete; graphical acceptance remains pending.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Complete native GUI verification before beginning Task 3. 3D remains paused.

## Done

- Project create/open/save/save-as use the UI lifecycle. Save As can copy a project into a new or empty destination and keeps asset-relative paths valid; it excludes recovery/build/VCS state and refuses symlinks, nested destinations, and nonempty destination directories. Editor root, asset browser, and GPU texture cache update after success.
- ADR-0049 records the scene architecture: `Project.scenes[active_scene_id]` is the authoritative authored document, while the editor ECS world is a runtime projection. Hierarchy, Inspector, and viewport edits target stable scene/entity IDs through validated commands or shared scene transactions.
- Supported Transform, Sprite, Role, Tilemap, and Parent data survive load/edit/save/reopen. Resource references are project-backed/editable and missing references are visible or repairable. Duplicate/delete/reparent preserve authored properties, stable identity rules, and references.
- Scene tabs load actual scene documents, retain per-scene history, show dirty state, and support safe create/close/reopen. Project replacement and exit use Save/Discard/Cancel. Dirty scene close and periodic recovery are implemented.
- Selection supports additive multi-select, framing uses transformed sprite bounds, move/rotate/scale gizmos and world snapping are implemented, and local/world transforms respect hierarchy composition. Gestures are transactional and undoable.
- Scene toolbar entity creation, hierarchy operations, Inspector edits, and viewport transactions participate in scene undo/redo. The combined two-scene acceptance test now undoes and redoes transform, reparent, duplicate, and delete, checks history isolation for the other scene, and verifies save/reopen plus runtime projection.
- Earlier native Xvfb checkpoints verified create/open/save, tab switching, dirty prompts, hierarchy operations, inspector edits, and translation. Final GUI confirmation of rotated/scaled gizmo drags and cross-directory Save As remains pending because this executor has no display or GUI automation.

## Validation

- Current `just check`: passed (format, strict Clippy, workspace check, standalone audio sample check). The standalone audio sample still reports existing unused/dead-code warnings in runtime audio/slice code.
- Current `just test`: 535 passed, zero failed/skipped. Focused project-roundtrip and two-scene acceptance suite: 11 passed.
- `cargo build -p kiln-editor`: passed.
- `DISPLAY` is unset; Xvfb and xdotool are absent. This environment cannot complete the plan's final native GUI acceptance. Physical devices, other render tiers, cross-platform runners, browser rerun, and remote CI remain unverified for this checkpoint.

## Next

1. Run the final Task 2 native GUI acceptance on a graphical runner: cross-directory Save As with a relative texture, relaunch/reopen, and rotate/scale gizmo drag plus undo/redo.
2. After that evidence is recorded, start Task 3 of `docs/2D_COMPLETION_PLAN.md`.

## Known limits

Save As rejects symlinks and nonempty destinations to protect unrelated files. Physics still consumes local Transform and is not parent-aware; tilemap cells keep identity transforms. Other editor resource panels remain transitional direct-mutation paths and belong to later tasks. Undo remains session-only; recovery snapshots are periodic (30 seconds). Only Empty 2D has been exercised as a starter template. Other render tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, physical devices, and remote CI remain separate gates. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
