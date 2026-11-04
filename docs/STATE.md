# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Completed

- Earlier checkpoints cover native/WASM baseline repair, project lifecycle and atomic writes, authored metadata preservation, multi-scene tabs/history, project Save/Discard/Cancel, transform/property commands, component presence commands, and hierarchy entity commands. ADR-0039–0048 record the architecture.
- Hierarchy create/delete/duplicate/reparent and inspector rename are document-first commands with ID validation, cycle prevention, metadata preservation, and undo. Native GUI previously verified hierarchy create/delete/duplicate/reparent and saved undo/redo of Parent.
- Inspector edits for Transform position/rotation/uniform-scale, Sprite tint/layer/dimensions, Role, and Sprite texture/material/animation references update authored state through history commands. Project texture/material/animation choices are available; missing texture, material, or animation references are flagged. Texture edits refresh loaded scene textures. No-op edits do not create history. Sprite dimensions default to 32×32 in old projects.
- Sprite rendering supports entity rotation/scale and authored dimensions. Parent links compose local translation/rotation/scale into runtime `GlobalTransform`; editor/player/web/mobile/sample renderers consume refreshed global poses. Viewport drags convert to local space; reparent/delete preserve child world poses and nested scene offsets survive propagation.
- Scene viewport has move, rotation, and scale gizmos. Drags preview local Transform edits, commit one undo step on release, and cancel through Escape. Rotation wrap, uniform scaling, hit targets, and translation snapping have coverage.
- Hierarchy and scene-list selection support Ctrl/Cmd additive toggling while preserving a primary inspector target. **Frame Selection** and `F` fit selected sprite bounds after global rotation and scale, with position-only bounds for nonvisual entities.
- All gizmo modes apply to multi-selection around the primary entity. Selected ancestors are transformed before descendants so child-first selection order preserves world targets. Each gesture is one undo transaction.
- Closing a dirty scene tab offers Save, Discard, or Cancel. Save writes the project before closing; Discard restores the saved scene or removes a never-saved scene while preserving other open-scene edits.
- Dirty projects write atomic recovery sidecars every 30 seconds. Opening a project with a newer sidecar offers Restore or Discard Recovery; restored tabs compare against the disk baseline, and successful project saves clear the sidecar.
- Combined two-scene acceptance coverage includes per-scene transforms/history, reparent world-pose preservation, duplicate/delete undo-redo, save/reopen, and reconstructed runtime transforms while preserving authored metadata.

## Validation

- Previous checkpoints progressed from 514 tests to 531, all passing. Current inspector resource validation checkpoint passes `just check` and `just test`: 532 passed, zero failed/skipped.
- Strict workspace lint passes. The separate audio sample check emits existing unused/dead-code warnings in `kiln-runtime` audio and slice code.
- Native GUI confirmation of recent inspector, gizmo, scene-close, and recovery workflows remains outstanding. This environment has no display, Xvfb, or desktop automation tool. Physical devices, other render tiers, cross-platform runners, and remote CI are unverified.

## Next

1. Continue Task 2's remaining authoring acceptance gaps, including stronger resource validation and native GUI verification.
2. Complete Task 2 before starting Tasks 3–9.

## Known limits and environment

`Transform` is local; call `kiln_runtime::update_global_transforms` after direct gameplay changes when a world-space read is needed. Built-in sprite render loops refresh `GlobalTransform` before drawing. Physics systems still consume local Transform and do not yet account for parent relationships; tilemap cell instances retain identity transforms. Other panels retain transitional direct-mutation paths. Undo is session-only; recovery snapshots are periodic (30 seconds), not continuous. Native visuals cover software GL/Low tier only.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised. Other render tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, physical devices, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
