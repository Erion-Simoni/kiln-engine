# Kiln 2D completion plan

## Goal and scope

A developer can create, edit, save, reopen, debug, and export a complete 2D game through the editor and scripts, without changing Rust code or hand-editing project files for ordinary authoring. This is the current 2D work program; 3D is paused. It supersedes earlier claims that only hardware testing remains, not the historical implementation record.

Work on branch `feat/2d-production-workflow`, based on master `cd62b63`. Complete tasks in order, in small reviewable changes. The user authorized continuing through this list sequentially. Existing implementations should be completed and integrated before replacing them. Do not mark a task complete because a panel, data type, or document exists.

## Definition of done

Each task needs implementation, meaningful regression tests, and an exercised editor/runtime workflow where relevant. Record exact commands and outcomes below; distinguish passed, failed, ignored, and unrun checks. Never simulate build success or suppress a required assertion to get a green run. Re-run the native editor after changes affecting startup/rendering. Every supported serialized property must survive save/reopen. Export validation must execute the exported game, not just create files. Hardware, cross-platform, and browser checks remain pending until actually run.

## Ordered work

### 1. Build and test baseline — COMPLETE (local Linux validation; remote CI pending)

- [x] Repair native workspace syntax/API/test-fixture blockers; subsequent browser checkpoint repaired the actual WASM target.
- [x] Make default native workspace builds and existing tests pass.
- [x] Repair audio feature compilation and exercise the standalone audio sample build (playback remains unverified).
- [x] Make the WASM target compile; add a real Chromium WebGL smoke test (other browser/device paths remain unverified).
- [x] Restore workspace formatting and strict native all-target lint checks without blanket lint suppressions or weakening CI.
- [x] Keep CI aligned with the native quality gate, browser smoke test, and Linux audio headers/configuration checks. Remote runner execution remains pending.

Acceptance: `just check`, `just test`, native editor build, audio sample check, and WASM build pass. Tests must execute nonzero cases. Graphical/browser/device checks have separate recorded evidence.

### 2. Dependable scene authoring and project lifecycle — IMPLEMENTATION COMPLETE; NATIVE ACCEPTANCE PENDING

- [x] Make project create/open/save/save-as and asset-relative paths work from the UI. Save As copies the project tree to a new/empty directory and preserves relative resource paths; a nonempty destination is refused.
- [x] Establish one authoritative editable scene model; route edits through commands shared by hierarchy, inspector, and viewport. ADR-0049 records the project scene as source of truth and the ECS world as a runtime projection.
- [x] Preserve supported components on load/edit/save/reopen; stable IDs and references survive duplicate/delete/reparent.
- [x] Complete selection, multi-selection, framing, move/rotate/scale gizmos, snapping, and parent/local transforms. Headless acceptance covers command math and persistence; final native GUI drag check remains pending.
- [x] Make scene tabs load actual scenes; preserve outgoing edits, isolate scene histories, create real scenes, show dirty indicators, and safely close/reopen saved tabs.
- [x] Guard project replacement and editor exit with Save/Discard/Cancel, including failed-save retry and project-wide dirty tracking.
- [x] Complete individual scene-tab Save/Discard/Cancel prompts and recovery/autosave. Dirty projects autosave a recoverable snapshot every 30 seconds.
- [x] Make undo/redo cover supported scene authoring commands, including inspector edits, entity/hierarchy commands, viewport transactions, and isolated per-scene histories.

Acceptance: headless two-scene acceptance now transforms/reparents/duplicates/deletes entities, undoes/redoes every operation in both isolated histories, saves/reopens, and verifies supported runtime values and asset references. Native editor restart/interaction acceptance still needs a graphical runner; the current environment has no display, Xvfb, or desktop automation.

### 3. Prefabs, scenes, and asset workflow — IN PROGRESS

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

### 2026-11-03 — Task 1 native checkpoint

- `cargo build --workspace`: passed on Linux.
- `cargo nextest run --workspace --no-fail-fast`: 452 passed, zero failed/skipped. Includes a new multi-frame state/nonvisual-entity regression, existing editor/player sample execution, save/load, and packed-archive tests.
- Fixed actual runtime defects: role lookup required a Sprite unnecessarily; after command application, stale host state overwrote score, pickup, and player-position changes. Host state is now adopted before commands apply.
- Updated malformed player script-session calls and stale Sprite/instance-layout fixtures. Corrected integration inputs to follow the sample manifest's diagonal path to coin1; scoring assertions remain active. The worker readiness test now allows bounded wall-clock scheduling time instead of assuming 1,000 yields suffice.
- `cargo check -p kiln-editor --features audio` and `cargo check --manifest-path samples/audio/Cargo.toml`: passed. Updated Rodio's Player API, audio category equality, ownership, host Debug formatting, and public host access. No audible playback or complete sound-file loading was validated; task 7 remains pending.
- Browser APIs are explicitly confined to wasm32. A separate actual `cargo check -p kiln-web --target wasm32-unknown-unknown` was run and still fails with 35 API errors. Native tests are not proof of browser readiness.
- `just check`: fails at existing workspace formatting differences; strict lint/build stages in that recipe did not execute. No lint suppression, test exclusion, or CI weakening was introduced. Task 1 is still IN PROGRESS; next is actual WASM API repair and full formatting/lint restoration, before task 2.
- Native editor startup/workspace rendering was verified in the prior merged editor fix. This checkpoint changes runtime state, fixtures, and feature compilation, not the redraw loop; device, browser, and audio playback checks remain explicitly unrun here.

### 2026-11-03 — Task 1 browser checkpoint

- Actual `cargo build -p kiln-web --target wasm32-unknown-unknown`: passed, with generated JS bindings from matching wasm-bindgen 0.2.130.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: passed end to end. It builds WASM/bindings and installs locked development-only browser tooling. The test rejects a missing canvas, loads all sample resources, finds 392 hero pixels, observes rightward movement, and verifies key release stops movement. Browser errors and asset HTTP failures fail the test.
- `cargo nextest run --workspace --no-fail-fast`: 453 passed, zero failed/skipped, including a new platform sound-callback regression without an OS audio device.
- Fixed browser Web Audio promises and public host access; reused native scene spawning to retain SourceId/Role; enabled real WebGL fallback when WebGPU has no adapter; configured the canvas surface before rendering; replaced unsupported std Instant on wasm32 with web-time.
- Visual verification exposed a shared-buffer bug: HUD uploads overwrote world sprites before GPU submission. World/overlay now have separate renderers and UI/HUD are batched. The browser pixel test catches this regression.
- Input verification exposed an animation-callback lifetime bug. The callback now owns its Window and reports requestAnimationFrame failures, instead of retaining a scoped borrowed handle that stopped later frames.
- Added `tests/web/README.md`, pinned npm test dependencies, `just web-smoke`, and a GitHub Actions browser job. Local execution passed; remote CI execution has not been claimed.
- Remaining Task 1 work: full-workspace formatting and strict lint gates. Browser smoke is a software-WebGL sample test, not proof of complete CLI exports, physical WebGPU, other browsers, touch, or full audio/save support. Tasks 2–9 remain pending.

### 2026-11-03 — Task 1 quality gate completed locally

- `just check`: PASSED. This executes workspace formatting, strict native all-target Clippy with `-D warnings`, workspace check, and the standalone audio sample check. Optional audio compilation still emits warnings; it is not a claim of audio playback or all-feature lint cleanliness.
- `just test`: 455 passed, zero failed/skipped. New regressions exercise the exported plugin create/destroy ABI and saved plugin enable state.
- `cargo build --workspace`: PASSED.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: PASSED after the cleanup; real sample assets, visible hero pixels, keyboard movement, key release, and startup-error rejection were exercised.
- Native editor: rebuilt and visibly verified welcome/main workspace in Xvfb software GL; no panic/GPU validation error. Virtual display/framebuffer warnings remain. This does not establish that every editor panel or authoring workflow is complete.
- Formatted the workspace, applied compiler-supported lint fixes, removed unused duplicate state/legacy methods, preserved the currently reachable graph tab, corrected egui APIs, and connected existing plugin enable configuration to its checkbox. No blanket lint allowances or required-test exclusions were added.
- Plugin ABI v2 now uses plugin-owned opaque handles and matching destruction instead of exposing Rust trait-object layout through C. ADR-0038 documents the compatibility change and limits; this does not implement a plugin loader.
- CI preserves its strict native gates and adds Linux audio development headers plus editor/sample audio checks. The browser job remains enabled. Remote CI/matrix execution has not been observed here.
- Task 1's specified local acceptance commands now pass. Next is task 2: dependable scene authoring/project lifecycle. Hardware, WebGPU/other browsers, export packaging, release-size/performance budgets, and full audio/UI/save functionality remain separate gates in tasks 2–9.

### 2026-11-03 — Task 2 project lifecycle and preservation checkpoint

- Implemented real New/Open/Save As path-entry dialogs. New Project now loads the created manifest instead of trying to read its directory as a file; location edits persist. Dialogs render once outside dock tabs, avoiding duplicate hub/widget IDs. File menu actions do not create unnecessary hub windows.
- Project creation stages a starter scene/script and refuses existing destinations. Manifest writes use sibling temporary files and atomic replacement. Parse/write failures are reported visibly; project/path state commits after successful I/O. Saving during Play is rejected to avoid persisting simulation changes.
- Save As supports a second manifest in the same asset directory. Relocation is explicitly rejected until asset copying/reference rewriting exists. Native OS file pickers and complete Platformer/Top-down templates remain pending; only the Empty 2D starter was exercised.
- Scene snapshots reconcile runtime edits with authored root metadata, preserving names, Parent, component ordering, unresolved references, relative textures, sprite material/layer/animation, and nonvisual entities. Nested instances are not baked into the root; other scenes remain intact. This is the transitional bridge in ADR-0039, not the complete shared command model.
- Shared runtime spawning now applies sprite layer/material and augments tilemap entities by SourceId instead of creating a duplicate identity.
- Nine disk-backed regressions cover create/load, invalid creation, overwrite refusal, safe failure, same-root Save As, metadata/nested-scene preservation, transform undo/redo through save/reopen, entity add/delete, and tilemap identity.
- `just check`: PASSED after the final GUI fix; strict native gates retained. `just test`: 464 passed, zero skipped. `cargo build -p kiln-editor`: PASSED.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: PASSED with the shared runtime changes (real rendering/assets/input/key release/startup rejection).
- Rebuilt native editor exercised in Xvfb/software GL: created a new Empty 2D project, reopened an existing manifest, used Save As to write an identical manifest, and verified New/Open dialogs without duplicate-widget errors. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain. Physical-device/cross-platform behavior and remote CI remain unverified.
- Task 2 remains IN PROGRESS: authoritative scene commands, actual scene-tab switching, complete selection/gizmo/parent behavior, broader undo coverage, dirty prompts, autosave/recovery, and full two-scene authoring acceptance remain ahead. Tasks 3–9 have not started.

### 2026-11-03 — Task 2 multi-scene workspace checkpoint

- Added an editor scene workspace with explicit active-scene identity, saved baselines, and isolated undo/redo histories. Snapshots, hierarchy edits, loading, saving, and undo use the active scene rather than whichever scene starts the game. Switching captures outgoing unsaved edits before replacing the world and clears transient selection/gizmo handles. ADR-0040 records the architecture.
- Tab actions execute outside dock rendering. The plus button and Scene Editor creation action insert real scene documents. Saved tabs can close without deleting scenes and reopen through the Scenes menu with their history retained. Dirty-tab closure is blocked visibly until saving; save baselines update after successful I/O.
- Play captures the active authoring document before starting the project's startup scene, then restores the active editor scene on Stop. Scene changes and scene undo/redo/reload are blocked during Play. Scene reload rebuilds the current in-memory authoring projection without silently discarding live edits; it does not mean rereading the manifest from disk.
- Wired the advertised Ctrl/Command+S/Z/Y actions while preserving text-field keyboard editing. Undo/redo remains limited to operations that already record snapshots; the shared command architecture and complete gizmo undo are not finished.
- GUI acceptance exposed an expanding FPS row that pushed tabs offscreen and a scene render pass that cleared the entire window after UI rendering. Constrained the toolbar row, bounded the entity list, made the canvas background transparent, and draw scene pixels before the editor UI. Viewport rectangles now come from the current frame, avoiding stale viewports when the scene tab is hidden. Visible native validation covers software GL/Low tier, not physical GPUs or every post-processing tier.
- Repeated untextured entity creation could reuse IDs because the UI's texture map omitted them. Entity/prefab ID generation now reserves all live SourceIds as well as supplied IDs; new scene-view entities also update the hierarchy handle map. A disk-backed repeated-add/switch/reopen regression covers the lost-entity case.
- Nine new headless workspace regressions cover two-scene edit/save/reopen with overlapping entity IDs and preserved names/resources, independent undo/redo stacks, dirty flags including undo/failed writes, safe close/reopen, scene creation/ID reuse, invalid transitions, empty-project creation, and repeated untextured entity IDs.
- `just check`: PASSED. `just test`: 473 passed, zero skipped. `cargo build -p kiln-editor`: PASSED after final source changes.
- Rebuilt native GUI in Xvfb/software GL: switched between one red entity and two blue entities; added entities independently; undid/redid an edit; started the startup scene and stopped back into the selected editor scene; saved and checked both documents on disk; restarted; verified Ctrl+Z/Y/S, unique repeated-add IDs, real scene creation, visible dirty-close warning, and clean tab close/reopen. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: PASSED (392 hero pixels, keyboard movement/release, assets, startup rejection). Initial sandbox execution could not bind localhost; the permitted elevated run passed.
- Task 2 remains IN PROGRESS. Next: shared authoritative edit commands, full transform/parent/selection behavior and undo coverage, project-close/save-discard prompts, and autosave/recovery. The full two-scene transform/reparent/duplicate/delete/restart acceptance is not yet complete. Tasks 3–9 remain pending.

### 2026-11-03 — Task 2 unsaved-project protection checkpoint

- Added an editor-only lifecycle guard comparing the complete authoring document against the last successful load/save. This covers live active-scene snapshots, other scenes, project settings, and serialized resource references; scene-tab dirty flags alone are insufficient. ADR-0041 records the decision. The menu bar shows an unsaved-project indicator.
- Actual Open/Create requests, recent-project opening, welcome Open, File Exit, and window-manager close use a single transition path. Opening a picker or New dialog does not itself discard anything. File Exit now exits through winit for normal cleanup rather than calling process::exit.
- Save writes before executing the pending action; failed Save keeps the dialog, action, and authoring document available for retry or cancellation. Discard continues without writing. Cancel/Escape leaves the document and disk unchanged. Repeated close/open requests cannot replace an existing pending action. Target parse/create failure retains the outgoing document and saved baseline.
- Stop Play before checking the authoring document, preserving pre-Play edits and excluding simulation changes. Background editor controls, scene shortcuts, and raw gizmo input are blocked while a choice is pending; transient drag state is cleared. Cancel returns to editing after Play has stopped.
- Added nine disk-backed regressions covering clean exit/no project, metadata/resources/scene removal, cancel preservation, discard without writing, actual failed-save/retry, repeated requests, save-before-open, create cancellation without filesystem side effects, and malformed target load after discard.
- `just check`: PASSED after final test changes. `just test`: 482 passed, zero skipped. `cargo build -p kiln-editor`: PASSED after final application changes.
- Rebuilt native GUI in Xvfb/software GL/Low tier: a real window-manager close prompted on live entity edits; Cancel and Escape retained edits with unchanged disk; background Add Entity was disabled; actual replacement failure displayed in the same prompt and retry saved before continuing; Discard switched between two real projects without writing the outgoing edits; canceling Create left no destination directory; File Exit with Save wrote the edited manifest and exited normally. After restart, closing during Play restored authoring state and prompted; Cancel retained it, Discard exited with the disk baseline intact. A subsequent clean restart/window close exited normally without prompting. All exit processes returned 0; no panic/GPU validation error, only expected virtual-display/framebuffer warnings.
- Browser smoke was not rerun for this editor-only lifecycle checkpoint; the preceding checkpoint's Chromium test passed. Cross-platform/physical-device validation and remote CI remain unverified.
- Task 2 remains IN PROGRESS: complete shared edit commands and transform/parent/selection/undo behavior, individual scene-tab choice prompts, and autosave/recovery before the full two-scene authoring acceptance. Tasks 3–9 remain pending.

### 2026-11-03 — Task 2 shared position commands and gesture undo checkpoint

- Added a validated SetTransform command shared by position inspector edits and viewport translation. It updates the serialized root scene and runtime projection together using scene/entity IDs, preserving other components, names, resources, and scene metadata. Missing/ambiguous identities, another scene, non-finite values, and nonuniform scale are rejected before mutation. Editing a default runtime transform now inserts the explicit authored component so the edit survives saving.
- Added transform transactions: typed entries and mouse drags capture the pre-edit document, preview updates, and record one undo snapshot at completion. Different inspector fields own separate gestures, including editing Y then X. No-op gestures retain redo. Escape cancels viewport translation without adding history. Scene/history/project transitions, save, Play, focus loss, and other explicit authoring operations finish pending position gestures. Rebuilt worlds remap the selected entity by SourceId instead of retaining a stale runtime handle.
- Replaced raw whole-window mouse dragging with egui canvas interaction so floating inspector windows and controls retain input. Translation uses the actual letterboxed viewport, DPI, and camera zoom; the initial transform plus total pointer displacement determines the preview, avoiding frame-by-frame drift. Snapping uses world grid units. Release outside the canvas ends the gesture; letterbox margins cannot start one. The effective scene camera is now shared with rendering instead of calculating it and rendering through an unused default camera.
- Nine new regressions cover document/runtime command parity and metadata preservation, repeated previews as one undo/redo step through disk reopen, no-op/redo retention, cancellation, invalid values/targets, ambiguous/missing identities, authoring a default projection, absolute/snapped translation, and DPI/letterbox/zoom mapping.
- `just test`: 491 passed, zero skipped. `just check`: PASSED, including a final rerun after documentation comments. `cargo build -p kiln-editor`: PASSED after final functional changes.
- Native GUI in Xvfb/software GL/Low tier: typed X entry 300 → 420 → undo 300 → redo 420; multi-frame canvas translation with stationary held pointer and one-step undo/redo; Escape restore without a new history entry; grid-snapped save at (576, 288); release in a letterbox margin followed by undo and pointer movement without a stuck drag; margin-started drag ignored; inspector mouse drag with one-step undo; history retained across scene switching; saved names/counts/other scene checked on disk; real restart restored the edited position; Y then X typed entries undid independently; normal clean shutdown. No panic/GPU validation error, only expected virtual-display/framebuffer warnings.
- Camera/DPI calculations are tested headlessly; physical high-DPI devices, nondefault native camera controls, other render tiers, cross-platform runners, and remote CI are unverified. Browser smoke was not rerun for this editor-only patch; prior Chromium checkpoint passed.
- Task 2 remains IN PROGRESS. This begins shared transform commands; it does not complete the authoritative command model, component add/remove semantics, rotation/scale gizmos, parent/local transforms, multi-selection/framing, dirty scene-tab choice prompts, or recovery/autosave. Tasks 3–9 remain pending.

### 2026-11-04 — Task 2 authored component commands checkpoint

- Added validated component commands for adding/removing Transform, Sprite, Role, and Tilemap. Inspector actions queue commands outside dock rendering, reconcile the outgoing root scene, record one actual change, rebuild the runtime projection, and remap selected SourceId. Inactive-scene/invalid/ambiguous targets fail before mutation. Duplicate add/absent remove are no-ops that preserve redo and original properties. ADR-0043 records the decision.
- Inspector component presence now follows authored data. The loader's implicit Transform no longer hides Add Transform or resurrects a removed authored Transform. Missing tilemap resources remain visible as authored components with a warning and Remove control. Add Tilemap offers existing project map IDs and rejects empty/unknown references rather than creating a placeholder.
- Role/Tilemap components have visible remove controls. Root entity selection includes objects without sprites, shows authored names, and follows document order rather than changing ECS archetype order. Removing a visual component no longer makes an entity unreachable after reopening.
- Eight new integration regressions exercise all four kinds through add/remove, history, actual runtime projection and disk reopen; default-transform authoring; intentional missing-map removal; validation/no-op redo preservation; fresh Sprite defaults after removal; unique tilemap identity with preserved role/transform; and invalid scene/entity/component duplicates. Runtime-only component helpers remain for existing callers; the Inspector no longer uses them.
- `just check`: PASSED. `just test`: 499 passed, zero skipped. `cargo build -p kiln-editor`: PASSED after the final entity-ordering change.
- Native GUI in Xvfb/software GL/Low tier: selected a nonvisual marker, added/saved its authored Transform and undid/redid it; removed and added Role and undid to the original role value; showed/removed an unresolved tilemap and verified save/undo/redo; selected a real map with Add Tilemap; removed Sprite while retaining selection, undid to original red color/layer 7, redid removal, and restarted; reselected the Sprite-less entity and added a fresh white/layer 0 Sprite; removed authored Transform, saved/undid/redid, restarted, and verified Add Transform remained available beside the retained Sprite/Role. Disk names/properties and normal process exits were verified. No panic/GPU validation error; expected virtual-display/framebuffer warnings remain.
- This editor-only checkpoint did not rerun browser smoke; previous Chromium evidence remains recorded. Physical devices, cross-platform runners, and remote CI are unverified.
- Task 2 remains IN PROGRESS: entity create/delete/duplicate and reparent commands, full property editing/undo, parent/local transforms, rotation/scale parity, selection/gizmos, dirty-tab choice prompts, and recovery/autosave remain ahead. Tasks 3–9 have not started.

### 2026-11-04 — Task 2 hierarchy commands checkpoint

- Added ADR-0044 and document-first commands for entity create, delete, duplicate, and reparent. Commands validate identities and scene scope before mutation, allocate project-wide fresh IDs, preserve full authored components on duplication, detach children when deleting a parent, and reject self-parenting, cycles, missing targets, and ambiguous parent links.
- Replaced the hierarchy's direct project/runtime mutations with queued commands applied after dock rendering. Successful edits rebuild the runtime projection from the authored scene, so duplicates retain Tilemap and other authored data instead of copying only Transform/Sprite/Role. Created/duplicated entities become selected by their new SourceId.
- Two new integration tests cover create/duplicate/delete/reparent, component preservation, ID collisions across scenes, child detachment, cycle rejection, no-op history, undo, and project save/reopen.
- `just check`: PASSED. `just test`: 501 passed, zero failed/skipped. Focused entity command suite: 2 passed.
- This editor-only checkpoint did not rerun native GUI or browser smoke. Previous GUI evidence does not cover the newly integrated hierarchy command path. Remote CI remains unverified.
- Task 2 remains IN PROGRESS. Manually verify hierarchy create/delete/duplicate/reparent and two-scene save/restart behavior next. Full property editing, parent/local transforms, rotation/scale parity, multi-selection/framing, dirty scene-tab choice prompts, and recovery/autosave remain pending. Tasks 3–9 have not started.

### 2026-11-04 — Task 2 native hierarchy drag-and-drop verification

- Native Xvfb testing first showed that hierarchy rows used a click-only interaction, so the old `drag_started` branch never generated reparent actions. Replaced it with egui's drag source/release payload API; dropping an entity on another row now queues the validated scene command.
- Exercised create and selection, duplicate and selection, delete followed by Ctrl+Z, then reparent by dropping `Created` onto `Hero`. Saved the Parent link, undid and saved to verify it disappeared from disk, then redid and saved to verify it returned. The GUI was software GL/Low tier; no panic or GPU validation errors.
- Added a positive reparent test that verifies an actual root-to-child link and history entry, while retaining cycle and no-op checks.
- The hierarchy path is now manually verified. Continue Task 2 with property-edit commands and one-step history, then parent/local transform parity, gizmo and multi-selection work, dirty-tab decisions, and recovery/autosave. Two-scene full acceptance is still pending.

### 2026-11-04 — Task 2 Inspector property transactions

- Added a validated Sprite visual-property edit for tint and sorting layer. Authored scene and runtime Sprite update together; unrelated texture, material, animation, transform, and role data remain intact. Invalid color/layer values and missing/ambiguous entities fail without partial mutation.
- Reused a general scene transaction for position, rotation, uniform scale, Sprite tint, and layer. Inspector typing and dragging update authored data and runtime state; per-widget gestures commit as one undo snapshot. Rotation is shown in degrees and stored in radians.
- Added three property regressions covering runtime/document parity and preserved metadata, multi-preview transaction history plus save/reopen, and invalid-value atomicity.
- `just check`: PASSED. `just test`: 504 passed, zero failed/skipped. Focused property suite: 3 passed; transform edit suite: 9 passed. Native editor build passed.
- Native GUI showed typed tint/rotation/scale fields and verified Ctrl+Z restored the tint. Rotation and scale are authored, but scene rendering still ignores them because SpriteInstance/shader do not carry transform rotation/scale. Adding renderer support and pixel tests is the next step; visual parity is not claimed here.
- Previous native hierarchy acceptance remains verified. This editor-only checkpoint did not rerun browser smoke; remote CI remains unverified.

### 2026-11-04 checkpoint — sprite rotation and scale rendering

- Shared sprite instances now carry rotation, and the WGSL shader rotates quads around their centers. Entity scale multiplies sprite dimensions before submission in editor, player, browser, mobile, and sample paths; UI/tilemap cell instances keep identity transforms.
- GPU readback regression confirms 32×8 baseline bounds, 8×32 at 90°, and 64×8 at 2× horizontal scale.
- Validation: `just check`, `just test` (506 passed), focused renderer GPU test, editor build, WASM check, and Chromium browser smoke all passed. Native editor visual confirmation is still outstanding.

### 2026-11-04 checkpoint — authored Sprite dimensions

- Sprite components now serialize width/height with a backward-compatible 32×32 default; runtime loading, editor snapshots, component addition, and Inspector property edits preserve them.
- Width/height use the same validated, undoable Sprite edit transaction as tint and sorting layer. Dimensions outside 0.001–8192 world units or nonfinite values are rejected atomically.
- Validation: `just check`, `just test` (507 passed), legacy deserialization, project round-trip, Sprite transaction/save-reopen, invalid-value tests, editor build, WASM check, and Chromium browser smoke passed.

### 2026-11-04 checkpoint — parent/local transforms

- Added core hierarchy transform resolution for local translation, rotation, and uniform scale, with rejection of cycles, missing parents, duplicate IDs, and duplicate Parent components.
- Runtime scene spawning now keeps `Transform` local, attaches runtime parent links, and carries `GlobalTransform` for resolved world poses. Render loops in editor/player/web/mobile/samples recompute global poses before drawing; separate nested-scene root offsets survive propagation.
- Editor transform commands update local authored values and affected runtime globals. Inspector shows local values; viewport drags convert world-space changes back to local. Reparenting preserves world pose; deleting a parent detaches children without moving them.
- Validation: `just check` passed; `just test`: 514 passed. Editor build, WASM check, Chromium smoke, nested-offset, runtime propagation, editor descendant update, and reparent/delete pose-preservation checks passed. Physics synchronization for parented entities is still pending.

### 2026-11-04 checkpoint — rotation and scale viewport gizmos

- Scene viewport now draws move axes, a rotation ring, and a scale handle at the selected entity's world-space origin. The move tool retains canvas dragging; rotation starts on the ring and uses angular pointer change, while scale starts on its handle and scales uniformly from vertical pointer movement.
- Every gizmo computes from press state, converts the resulting world pose back to the authored local Transform (including parent rotation/scale), previews through the existing SceneTransaction path, commits once on release, and supports Escape cancellation.
- Validation: `just check` passed; `just test`: 518 passed; editor build passed. Pure tests cover translation snapping, rotation/wraparound, uniform positive scale, and visible handle hit targets. Native visual drag acceptance remains outstanding.

### 2026-11-04 — Task 2 implementation and headless acceptance completed

- Added ADR-0049: `Project.scenes[active_scene_id]` is the authoritative editable document; ECS worlds are projections. Hierarchy, inspector, and viewport mutations share validated scene/entity identity, document-first commands or `SceneTransaction`, then rebuild/update runtime state. The scene toolbar's Add Entity now follows the same command path.
- Save As now relocates by copying the project directory to a new or empty destination, preserving every project-relative asset/resource path. It excludes recovery snapshots and local build/VCS state, rejects symlinks and nonempty destinations, refuses nesting a copy inside its source, and changes editor asset root/browser/texture uploads only after a successful save. The original project remains untouched.
- Reworked the combined two-scene acceptance test to undo and redo Transform, Reparent, Duplicate, and Delete in sequence; it verifies independent undo/redo in the second scene, then saves/reopens and checks authored metadata, references, local/global runtime transforms, and entity state.
- Save As regressions cover same-root copies, relocation with a real relative texture asset, copied scripts, project round-trip, omitted recovery sidecars, occupied destination preservation, and rejection of nested copies without creating partial directories.
- `just check`: PASSED. `just test`: 535 passed, zero failed/skipped. Focused project-roundtrip/two-scene acceptance: 11 passed. `cargo build -p kiln-editor`: PASSED.
- Native GUI confirmation of the Save As relocation and final rotation/scale drag workflow could not run in this executor: `DISPLAY` is unset and Xvfb/xdotool are unavailable. Earlier checkpoints contain native verification of project create/open/save, scene tabs, hierarchy, inspector, and transform editing. Task 2 is implementation-complete; its graphical acceptance remains pending before marking the task fully complete. Tasks 3–9 remain pending.

### 2026-11-04 — Task 3 prefab-library persistence started

- Added optional `Project.prefabs`, keyed by prefab ID, so reusable definitions are saved in the project manifest. Project load hydrates the Prefab Editor state, the selection-based create action inserts definitions into the project, and panel edits/deletes sync back before project save.
- Added ADR-0050 and tests for full TOML prefab definition round-trip, legacy manifests without a prefab map, and Prefab Editor selection behavior after project replacement.
- Focused validation: `cargo nextest run -p kiln-core --lib` (50 passed) and `cargo nextest run -p kiln-editor --test project_roundtrip` (10 passed). Full `just check` passed; `just test` passed 538 tests; `cargo build -p kiln-editor` passed.
- Task 3 is IN PROGRESS. Persisted scene-instance links, drag/drop creation, functional apply/revert overrides, and propagation across save/reopen remain unimplemented.

### 2026-11-04 — Task 3 serialized prefab instances

- Added `Component::PrefabInstance` scene metadata containing prefab ID and overrides, and routed Instantiate Prefab through the validated document-first entity command. Instances use project-unique IDs, preserve authored components/references, start as scene roots at the requested position, and receive a correctly keyed empty override record.
- Editor scene rebuilds restore runtime prefab links from authored markers by SourceId. Scene snapshots preserve those markers; duplicate updates the override record's entity ID. Inspector identifies linked prefabs and warns on missing definitions. ADR-0051 documents the serialization decision.
- Regression covers instantiate, duplicate, undo/redo, save/reopen, relative texture/animation retention, runtime link restoration, and scene snapshot parity. `just check`: PASSED; `just test`: 539 passed, zero failed/skipped.
- Task 3 remains IN PROGRESS: functional per-instance override editing/revert, propagation when a definition changes (including saved instance data), drag/drop instantiation, nested scenes, and asset workflows remain ahead.

### 2026-11-04 — Task 3 override reset and saved propagation

- Added a Revert Overrides action for linked scene entities. The document-first command restores the prefab's authored component set, clears its overrides, keeps the placed entity's ID/name, and records the change in per-scene undo history.
- Prefab definition edits now propagate into saved instances across project scenes, retaining instance IDs, names, parent links, and supported overrides. Runtime instances continue to update through the existing projection path.
- Regression coverage verifies revert/undo and saved-instance definition propagation with an overridden transform and parent link. Focused command suite: 7 passed. `just check` passed; `just test`: 541 passed, zero failed/skipped.
- Scene snapshots now capture transform and Sprite tint/texture differences from prefab definitions into each instance's serialized override map; placement offsets are recorded at instantiate time. Texture removal overrides and applying an instance back to its definition remain unfinished. Drag/drop, nested-scene lifecycle, and asset import/rename/move workflows also remain unfinished. Task 4 has not started.
