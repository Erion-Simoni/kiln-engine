# Kiln 2D completion plan

## Goal and scope

A developer can create, edit, save, reopen, debug, and export a complete 2D game through the editor and scripts, without changing Rust code or hand-editing project files for ordinary authoring. This is the current 2D work program; 3D is paused. It supersedes earlier claims that only hardware testing remains, not the historical implementation record.

Work on branch `feat/2d-production-workflow`, based on master `cd62b63`. Complete tasks in order, in small reviewable changes. The user authorized continuing through this list sequentially. Existing implementations should be completed and integrated before replacing them. Do not mark a task complete because a panel, data type, or document exists.

## Definition of done

Each task needs implementation, meaningful regression tests, and an exercised editor/runtime workflow where relevant. Record exact commands and outcomes below; distinguish passed, failed, ignored, and unrun checks. Never simulate build success or suppress a required assertion to get a green run. Re-run the native editor after changes affecting startup/rendering. Every supported serialized property must survive save/reopen. Export validation must execute the exported game, not just create files. Hardware, cross-platform, and browser checks remain pending until actually run.

## Ordered work

### 1. Build and test baseline — IN PROGRESS

- [ ] Repair workspace syntax and API mismatches, including stale test fixtures.
- [ ] Make default native workspace builds and existing tests pass.
- [ ] Repair audio feature compilation and exercise the standalone audio sample build.
- [ ] Make the WASM target compile; add browser smoke coverage before claiming web support.
- [ ] Restore formatting and strict lint checks without blanket lint suppressions or weakening CI.
- [ ] Keep CI aligned with the actual supported toolchain, system dependencies, features, and test commands.

Acceptance: `just check`, `just test`, native editor build, audio sample check, and WASM build pass. Tests must execute nonzero cases. Graphical/browser/device checks have separate recorded evidence.

### 2. Dependable scene authoring and project lifecycle — PENDING

- [ ] Make project create/open/save/save-as and asset-relative paths work from the UI.
- [ ] Establish one authoritative editable scene model; route edits through commands shared by hierarchy, inspector, and viewport. Document the decision before restructuring.
- [ ] Preserve all supported components on load/edit/save/reopen; stable IDs and references survive duplicate/delete/reparent.
- [ ] Complete selection, multi-selection, framing, move/rotate/scale gizmos, snapping, and parent/local transforms.
- [ ] Make scene tabs load actual scenes; add dirty indicators, close prompts, recovery/autosave.
- [ ] Make undo/redo cover every supported authoring command.

Acceptance: author a two-scene project, transform/reparent/duplicate/delete entities, undo and redo, save, restart the editor, and verify identical supported properties and asset references.

### 3. Prefabs, scenes, and asset workflow — PENDING

- [ ] Create prefabs from selection, instantiate by drag/drop, persist instances, apply/revert overrides, and propagate definition changes.
- [ ] Complete nested-scene loading and transitions without cyclic ownership or accidental cross-scene edits.
- [ ] Add import settings, stable asset IDs, resource pickers, rename/move/reimport tracking, and missing-asset diagnostics.

Acceptance: reuse an enemy prefab in two levels, override one instance, update its definition, rename its texture, then save/reopen and export with expected results.

### 4. General gameplay scripting and Play workflow — PENDING

- [ ] Specify entity-attached scripts, self/entity references, per-instance persistent state, inspector-exposed properties, and lifecycle callbacks.
- [ ] Add reusable functions/modules and collections as required by the reference games; preserve existing scripts or document migrations.
- [ ] Deliver events/signals, collision callbacks, input actions, useful diagnostics, and working debugger controls.
- [ ] Unify editor/player execution; add pause, step, restart, current-scene/project-start play, correct focus/timing, and explicit runtime-change application.

Acceptance: two independently scripted enemies, a projectile, a trigger, and a level transition run identically in editor and player; stop restores authoring state; errors point to the correct script location.

### 5. Physics and cameras — PENDING

- [ ] Implement actual ray/shape queries, collision enter/exit events, sensors, masks, body removal, joints, and fixed-step synchronization.
- [ ] Add reliable collider authoring and physics debug drawing.
- [ ] Finish camera follow, smoothing, bounds, zoom, pixel-perfect settings, and viewport/input coordinate mapping.

Acceptance: platformer grounding, slopes/moving platforms, triggers, collision filtering, camera limits, and multiple frame rates have gameplay regression tests and visible editor validation.

### 6. Sprites, animation, tilemaps, and VFX — PENDING

- [ ] Complete sprite drag/drop, pivots, sorting, import filtering, and pixel scale.
- [ ] Replace animation preview placeholders with actual texture playback; support frame editing, events, transitions, and runtime parity.
- [ ] Complete tile palette/brush/erase/fill, multiple layers, terrain painting, collision previews, and map persistence.
- [ ] Verify particle authoring, preview, serialization, and runtime execution agree.

Acceptance: create an animated character and a multilayer playable tilemap through the editor, save/reopen, play, and export without hand-editing scene files.

### 7. Audio, UI, saving, and settings — PENDING

- [ ] Wire real audio import/playback/preview, simultaneous effects, looping music, buses, and persisted volume settings.
- [ ] Complete UI layout, anchors, text/fonts, buttons, focus/controller navigation, and script events.
- [ ] Provide save/load APIs, versioned game data, and player settings independent of engine sample-specific score/coin concepts.

Acceptance: title/pause/settings/game-over screens, music and overlapping effects, controller navigation, and save/resume work in editor and exported game.

### 8. Real build/export and distribution — PENDING

- [ ] Replace the build panel's simulated success with a real asynchronous CLI export process, actual exit status, streamed diagnostics, output validation, and cancellation.
- [ ] Make export preserve project-relative resources and include the correct player/runtime without requiring end users to install Rust.
- [ ] Deliver one clean-machine desktop export first, then browser export with loading, input, audio-unlock, and save behavior verified.
- [ ] Add portable editor distribution and document supported OS/GPU requirements; validate other platforms on appropriate machines.

Acceptance: author and export from the editor, move the artifact outside the repository to a machine without Rust, and play the full game. A failing exporter produces failure, never a success message.

### 9. Reference games and release gate — PENDING

- [ ] Build a top-down action game covering enemies/projectiles/triggers/audio/UI/saving.
- [ ] Build a platformer covering grounding/animation/cameras/levels.
- [ ] Build a puzzle game covering reusable objects/mouse/UI/progress.
- [ ] Have another developer follow the tutorial unaided and fix every ordinary task that requires engine edits or manual project-file edits.
- [ ] Measure reference hardware frame time/memory/startup; test recovery, long sessions, large asset sets, and clean exports.
- [ ] Reconcile all roadmap and support claims against executed evidence; defer nonessential plugins, 3D, multiplayer, stores, and mobile breadth until this gate passes.

Acceptance: all three games can be authored, debugged, reopened, and shipped with documented supported workflows; release checks pass and remaining limitations are explicit.

## Execution log

### 2026-11-03 — Task 1 started

- Created this plan after reviewing real source gaps, including simulated build success, unfinished physics queries/events, scene-tab loading, prefab controls, and animation preview.
- Prior native editor startup fixes are already merged into master. Prior validation: 33 editor library tests passed; welcome/workspace rendering was visually verified with software GL. This does not establish workspace, audio, web, export, or device readiness.
- Current baseline blockers: unmatched closing delimiter in kiln-web, malformed player integration test calls, stale core test Sprite fields, optional audio compile errors, and existing lint/format failures. All remain unchecked until retested and repaired.
