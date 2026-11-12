# Kiln 2D completion plan

## Goal and scope

A developer can create, edit, save, reopen, debug, and export a complete 2D game through the editor and scripts, without changing Rust code or hand-editing project files for ordinary authoring. This is the current 2D work program; 3D is paused. It supersedes earlier claims that only hardware testing remains, not the historical implementation record.

Work on branch `feat/2d-production-workflow`, based on master `cd62b63`. Complete tasks in order, in small reviewable changes. The user authorized continuing through this list sequentially. Existing implementations should be completed and integrated before replacing them. Do not mark a task complete because a panel, data type, or document exists.

## Definition of done

Each task needs implementation, meaningful regression tests, and an exercised editor/runtime workflow where relevant. Record exact commands and outcomes below; distinguish passed, failed, ignored, and unrun checks. Never simulate build success or suppress a required assertion to get a green run. Re-run the native editor after changes affecting startup/rendering. Every supported serialized property must survive save/reopen. Export validation must execute the exported game, not just create files. Hardware, cross-platform, and browser checks remain pending until actually run.

## Ordered work

2026-11-05 readiness audit: **2D is not release-complete.** GitHub master
`76da93f` has the same source tree as feature commit `20291fa`. Fresh `just check`
and all 592 tests pass, but Tasks 7–9, native workflow acceptance and additional
scripting/spawn/browser integration gaps remain. See
[`2D_READINESS_AUDIT.md`](2D_READINESS_AUDIT.md) for source evidence. Earlier
implementation checkboxes describe their tested contracts, not full traditional
engine feature parity. This audit does not start a new implementation task.

2026-11-06 update: Task 7 software implementation and native/editor/KAR/browser
acceptance now pass; general saves and browser UI/audio/storage are integrated.
The dated audit and checkpoints below remain a historical record. Tasks 8–9,
physical-device checks and broader release acceptance remain pending.

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

### 3. Prefabs, scenes, and asset workflow — IMPLEMENTATION COMPLETE; GRAPHICAL ACCEPTANCE PENDING

- [x] Create prefabs from selection, instantiate by drag/drop, persist instances, apply/revert overrides, and propagate definition changes.
- [x] Complete nested-scene loading and transitions without cyclic ownership or accidental cross-scene edits.
- [x] Add import settings, stable asset IDs, resource pickers, rename/move/reimport tracking, and missing-asset diagnostics.

Headless acceptance passes: reuse an enemy prefab across two scenes, apply an instance's edits to the definition while preserving the other instance's override, rename its texture, save/reopen, and verify the exported archive contains the renamed texture. Native visual acceptance of prefab and asset drag/drop is pending because this executor has no display server or Xvfb.

### 4. General gameplay scripting and Play workflow — IMPLEMENTATION COMPLETE; GRAPHICAL ACCEPTANCE PENDING

- [x] Specify entity-attached scripts, self/entity references, per-instance persistent state, inspector-exposed properties, and lifecycle callbacks.
- [x] Add reusable functions/modules and collections as required by the reference games; preserve existing scripts or document migrations.
- [x] Deliver events/signals, collision callbacks, input actions, useful diagnostics, and working debugger controls.
- [x] Unify editor/player execution; add pause, step, restart, current-scene/project-start play, correct focus/timing, and explicit runtime-change application.

Acceptance: two independently scripted enemies, a projectile, a trigger, and a level transition run identically in editor and player; stop restores authoring state; errors point to the correct script location.

Headless acceptance passes through the shared native host loader/scheduler:
two isolated enemy scripts with distinct properties, a projectile, backend
collision delivery, a trigger signal, and a transition request produce identical
results from source resources and deserialized player project/byte assets.
Stop callbacks run without mutating authoring documents; imported-function
compile/runtime errors identify their source file/line/column. Geometric
collision production is Task 5; the acceptance injects a backend event.
Live debugger controls operate at gameplay-frame granularity and inspect actual
instance state. Source/instruction breakpoints and arbitrary watch evaluation
are not exposed. Native visual interaction remains unverified without a GUI
runner; see ADR-0052 and `docs/KILNSCRIPT.md` for the supported contract.

### 5. Physics and cameras — IMPLEMENTATION COMPLETE; NATIVE ACCEPTANCE PENDING

- [x] Implement actual ray/shape queries, collision enter/exit events, sensors, masks, body removal, joints, and fixed-step synchronization.
- [x] Add reliable collider authoring and physics debug drawing.
- [x] Finish camera follow, smoothing, bounds, zoom, pixel-perfect settings, and viewport/input coordinate mapping.

Acceptance: platformer grounding, slopes/moving platforms, triggers, collision filtering, camera limits, and multiple frame rates have gameplay regression tests and visible editor validation.

Headless regressions exercise real Rapier contacts, sensor transitions and
geometry queries, all three joint types, moving platforms, hierarchy/offset
conversion, body recreation/removal, command history and save/reopen. The
checked-in `samples/physics2d_editor` scripts jump, cross a trigger and an
excluded collision layer, and clamp the gameplay camera identically from source
at 30 FPS and player assets at 144 FPS. Native visual acceptance is pending:
the cloud runner has no display server. Follow `docs/PHYSICS_2D.md` to record it.

2026-11-04 validation: `just check` passed; `just test` ran **577 cases, all
passed, zero skipped**; `cargo build -p kiln-editor -p kiln-player` passed;
`cargo clippy -p kiln-runtime --no-default-features --all-targets -- -D warnings`
passed; `cargo build --target wasm32-unknown-unknown -p kiln-web` passed. Built
editor startup was attempted and failed at window creation because DISPLAY and
Wayland variables are absent. No native visual or browser physics gameplay
acceptance is claimed.

### 6. Sprites, animation, tilemaps, and VFX — IMPLEMENTATION COMPLETE; NATIVE WORKFLOW ACCEPTANCE PENDING

- [x] Complete sprite drag/drop, pivots, sorting, import filtering, and pixel scale.
- [x] Replace animation preview placeholders with actual texture playback; support frame editing, events, transitions, and runtime parity.
- [x] Complete tile palette/brush/erase/fill, multiple layers, terrain painting, collision previews, and map persistence.
- [x] Verify particle authoring, preview, serialization, and runtime execution agree.

Acceptance: create an animated character and a multilayer playable tilemap through the editor, save/reopen, play, and export without hand-editing scene files.

2026-11-04: authoring-command acceptance generates
`samples/visual2d_editor`, saves/reopens it, packs a real KAR archive and executes
the native player loader/shared gameplay from source and archive at 30/144 FPS.
Assertions cover wall collision, clip transitions and frame events, particles,
sorting and authoring isolation. Painted solids also generate merged static
Rapier geometry; grounding, contacts, rays, edits and removal are regression
tested. Headless egui acceptance checks the actual
decoded preview texture and frame UVs. `just check` passes; `just test` runs
**592 tests, all passed, zero skipped**; editor/player builds, no-physics Clippy
and WASM build pass. Chromium WebGL smoke tests pass for both this sample and
the original vertical slice, including input and zero runtime/asset errors;
linear filtering plus generated mipmaps is also exercised.

Native editor/player startup was attempted; this runner has no X11/Wayland
display. Native drag/drop/panel interaction and graphical packed-player
acceptance remain unverified. Real archive export is exercised; automating the
editor's Build/Export panel remains Task 8. See `docs/VISUAL_AUTHORING_2D.md` for
the exact supported contract and graphical acceptance procedure.

### 7. Audio, UI, saving, and settings — IMPLEMENTATION COMPLETE

- [x] Implement native audio import/playback/preview, simultaneous effects, looping music, named buses, and persisted volume settings. Shared decoded PCM passes project save/reopen and packed-player loading; browser Web Audio loop/overlap/gain also passes real Chromium checks. Audible-device acceptance remains separate.
- [x] Add versioned player preferences and typed `setting_get`/`setting_set`, shared by scene/entity scripts and independent of sample score/coins. Failed/corrupt preference files report errors and are preserved.
- [x] Implement native saved UI layout/anchors, bitmap and TTF/OTF text, buttons, focus/keyboard/controller navigation and root/entity script events. Native player menus and editor UI Designer add/drag/undo/redo/save/reopen/Play pass on Xvfb. Browser saved fonts/UI, controller API navigation and touch pass in Chromium; physical-controller acceptance remains separate.
- [x] Provide versioned general game data and completed-tick save/load APIs shared by editor/player/browser: active scene/world, stable IDs, scripts, UI, typed progress, independent preferences, atomic failure handling and resource rebinding.

Acceptance: title/pause/settings/game-over screens, music and overlapping effects, controller navigation, and save/resume work in editor and exported game.

Audio checkpoint: see [AUDIO_AUTHORING_2D.md](AUDIO_AUTHORING_2D.md), ADR-0055 and
`kiln-player/tests/audio_workflow.rs`. Native editor/player enable audio by default;
no-audio builds remain supported. The saved UI checkpoint is documented in [UI_AUTHORING_2D.md](UI_AUTHORING_2D.md).
The general save contract and compatibility limits are documented in
[GAME_SAVING_2D.md](GAME_SAVING_2D.md), ADR-0058 and
[evidence/task7-complete/README.md](evidence/task7-complete/README.md).
Fresh-process native directory/KAR resume, editor Play resume and browser reload
resume passed with health/inventory/script lifecycle preserved. Corrupt/future
saves and browser quota failures preserve the old slot/live world. Software
acceptance is complete; physical-controller, audible-device and remote CI checks
remain explicitly separate. Real editor export/distribution is Task 8.

### 8. Real build/export and distribution — IN PROGRESS (Linux and Windows candidate checkpoints accepted)

- [x] Replace simulated Build/Export with a real asynchronous native/browser CLI process, bounded streamed stdout/stderr, actual exit status, job-specific receipt/resource validation off the UI thread and cancellation/cleanup. Real editor Save & Export and UI Cancel pass on Linux/Xvfb. Unsupported targets/options fail explicitly; Browser gates authored physics/joints/solid tilemaps against the selected runtime capability and validates/gates authored sprite materials against the selected runtime.
- [x] Bundle resources/runtimes for the current supported native/browser targets. Native packs/verifies a KAR and includes a matching player; Browser includes the normalized project tree, hash-verified prepared JS/WASM and actual loading/start/error shell. Export/play need no Cargo. Default Browser builds now include shared 2D physics; capabilities remain explicit and other target/toolchain support is pending.
- [x] Deliver clean Linux userspace acceptance plus relocated Browser loading/input/audio-unlock/save behavior. A pinned Ubuntu 24.04 container runs the installed editor and exported game offline/unprivileged without Rust, repository or development sysroot mounts; real edit/save/export/cancel and game settings/save/restart/Continue pass. Shipped runtime requirements include the discovered dynamic X11 dependency. Browser artifact passes nested HTTP deployment, loading/error/retry, input, explicit audio unlock and reload/save/resume. Physical machines/other OS and driver/device coverage remain in the platform gate below.
- [x] Publish an accepted Linux release candidate and retain GitHub build-origin attestation as Actions artifacts. Refreshed licensed run 43215982528 at 3ff72d3 passed native quality/tests, two fresh locked/offline builds, actual six-payload comparison, browser/moved installation/export/gameplay and clean Ubuntu gates; candidate/evidence upload and separate attestation/bundle upload succeeded. Archives include matched tools, browser runtime, templates/examples, source/build records, SHA-256 inventory, project grants, original dependency/font/Rust notices and verified MPL sources. Public job conclusions establish hosted execution; independent downloaded-signature verification is separate.
- [x] Resolve the project license policy and include its grant files in new portable packages. Owner delegated the choice; MIT OR Apache-2.0 is retained with full root texts, consistent metadata for all 34 workspace packages and original third-party/font notices. Actual local packaging verified both exact project grant files; historical candidate contents remain unchanged.
- [x] Prepare a Windows x64 portable ZIP with static MSVC CRT and accept its relocated installation on a Windows runner. Run 43215982526 at 3ff72d3 passed package verification/corruption rejection, all-template Windows/browser CLI exports without build tools on child PATH, source-unavailable standalone game relocation, visible editor/player captures and normal close. Candidate upload and separate origin attestation/bundle upload passed. Full GUI authoring/gameplay and physical devices remain separate.
- [x] Propagate complete matched runtime project/dependency/Rust notices into native/browser game exports from verified portable installations. Receipt-bound SHA-256 inventories cover original notice bytes and selected runtime payloads. CLI/editor reject damaged or incomplete inventories; loose developer runtimes warn explicitly. See ADR-0072 and [runtime notice evidence](evidence/task8-runtime-notices/README.md).
- [ ] Finish reviewed public editor releases and validate other platforms on appropriate machines. Remaining SDK/dependency provenance, OS installer/signing, physical-machine, full Windows GUI authoring/gameplay, macOS and other GPU checks stay open. Current Linux candidate requires GLIBC 2.39 and documented system libraries. Package-size goals are removed by owner request (ADR-0062). See [PORTABLE_EDITOR.md](PORTABLE_EDITOR.md).

Acceptance: author and export from the editor, move the artifact outside the repository to a machine without Rust, and play the full game. A failing exporter produces failure, never a success message.

Native checkpoint: [NATIVE_EXPORT_2D.md](NATIVE_EXPORT_2D.md), ADR-0059 and
[evidence/task8-native/README.md](evidence/task8-native/README.md). Task 8 remains
open for physical-machine/other-platform checks, remaining provenance/licensing review and
other-platform editor distribution (development relocation checkpoint passes). See [BROWSER_EXPORT_2D.md](BROWSER_EXPORT_2D.md) for
the tested browser contract. Task 9 is not started.

`just portable-release` now captures the actual source/lockfile, builds native and
WASM hosts offline/locked into a fresh target directory and packages a checked
unsigned local build record. See [BUILD_PROVENANCE.md](BUILD_PROVENANCE.md).
This adds source-to-artifact execution evidence; it does not close signing,
byte-reproducibility, licensing or physical/other-platform gates.

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
- Task 1's specified local acceptance commands now pass. Next is task 2: dependable scene authoring/project lifecycle. Hardware, WebGPU/other browsers, export packaging, performance/resource budgets, and full audio/UI/save functionality remain separate gates in tasks 2–9.

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

### 2026-11-04 — Task 3 prefab drag-and-drop placement

- Prefab list entries are now egui drag sources. Dropping a prefab into the scene canvas computes the world-space position through a tested pointer mapping helper and queues the same validated `InstantiatePrefab` scene command as toolbar placement.
- Canvas drops work with the gizmo mode set to None and are blocked while editing is locked. Existing command behavior supplies stable IDs, undo history, authored prefab links, and persisted placement overrides.
- Validation: pointer mapping unit test, focused prefab/entity suite (7 passed), `just check` passed, `just test`: 542 passed/zero failed, and `cargo build -p kiln-editor` passed. Native GUI drag acceptance remains unverified in this headless environment.
- Task 3 remains IN PROGRESS: instance-to-definition Apply, texture-clear override representation, nested scenes, and asset import/rename/move workflow remain unfinished. Task 4 has not started.

### 2026-11-04 — Task 3 apply-to-definition and nested-scene safeguards

- Inspector now offers Apply Overrides to Prefab. It replaces the definition with the selected instance's supported authored components, clears that instance's overrides, and propagates the new base to other saved instances while retaining their overrides. Dedicated Undo Prefab Apply / Redo Prefab Apply controls restore project prefab/scene snapshots; histories reset on project load and Save As.
- Nested-scene composition now rejects direct/indirect cycles, supports safe offset/additive edits, and can remove a link without deleting either scene. Scene Editor lists actual nested links and exposes these controls.
- Regression tests cover applying one instance into a definition with a second instance's independent override, indirect cycle rejection, finite offset validation, link configuration, and removal. `just check` passed; `just test`: 544 passed, zero failed/skipped; `cargo build -p kiln-editor` passed.
- Task 3 remains IN PROGRESS: asset rename/move/reference tracking, import settings/stable asset IDs/pickers, and broader lifecycle acceptance remain outstanding. Task 4 has not started.

### 2026-11-04 — Task 3 asset move and reference tracking

- Asset Browser can move/rename a selected image to an existing project-relative folder. Path traversal, symlink escapes, symlink source assets, missing destinations, and overwriting are rejected. The active scene reloads after the operation.
- Scene and prefab Sprite texture references are rewritten together. Scene Editor picks the next sorted, not-yet-nested scene, and lists actual nested links with offset/additive controls and safe removal.
- Regression tests cover file moves and invalid paths, reference rewrites in scenes/prefabs, project-wide Apply to Prefab, and nested-scene graph safeguards. `just check` passed; `just test`: 546 passed; `cargo build -p kiln-editor` passed.
- Task 3 remains IN PROGRESS: stable asset IDs/import settings/resource pickers/cache reimport tracking, texture-clear overrides, and full two-level GUI/save/reopen/export acceptance remain outstanding. Task 4 has not started.

### 2026-11-04 — Task 3 implementation complete

- Added serde-defaulted stable path-to-ID and ID-to-import-settings project metadata. Asset import assigns IDs and persists filtering/mipmap/sprite settings into both the project and hash cache. Cache settings survive reload and migrate with a rename; asset scans exclude caches/build/VCS trees and do not follow symlinks.
- Added explicit texture-clear prefab overrides while retaining deserialization of legacy string overrides. Inspector already provides asset/material/animation pickers and missing-reference diagnostics; Asset Browser now moves images and updates project references and import cache records.
- Added full two-scene prefab/rename/save/reopen/export acceptance. `just check` passed; `just test`: 552 passed, zero failed/skipped; editor build passed. Native visual drag/drop remains unverified without a display server or Xvfb.
- Task 3 implementation is complete. Keep Task 4 pending until graphical acceptance is recorded in a GUI-capable environment.

### 2026-11-04 — Task 4 Play controls and source diagnostics

- User explicitly requested Task 4; the prior Task 3 graphical check remains separately pending. Added ADR-0052 to specify Play isolation and the remaining entity/language execution contract.
- Current-scene Play uses the active scene; project-start Play is separate. Pause/resume, one-frame step, restart, focus-loss pause, text-input isolation, forwarded action keys, and guarded runtime-transform application are implemented. Editor/player now share the fixed-step clock with bounded catch-up.
- Compiler diagnostics in editor/player include source file, line, and column. Tests cover Unicode/clamped spans, non-default scene launch, frame-step timing, and retaining authored metadata while applying runtime transforms.
- `just check`: passed; `just test`: 556 passed, zero failed/skipped; editor/player builds passed. Native interaction remains unverified without a GUI runner. Entity attachments/lifecycle, functions/modules/collections, signals, live VM debugging, runtime source maps, and full gameplay acceptance remain pending. Task 4 remains IN PROGRESS.

### 2026-11-04 — Task 4 entity scripting implementation completed

- Added authored Script components, validated undoable attach/edit/remove commands, inspector resource selection/numeric properties, and projection/disk preservation. Native editor/player now call the same scene/module loader and entity scheduler, including attachment-only scenes.
- Added isolated persistent state, self IDs, start/update/signal/collision/stop phases, stable dispatch order, deferred bounded signals, bounded array operations, scene-transition requests, and normalized keyboard actions. World systems advance once per tick rather than once per callback.
- Added non-recursive named functions with isolated locals, project-relative modules with import-cycle/path/size limits, IR source offsets, and source-mapped runtime errors. Existing scene scripts keep their top-level per-frame behavior; reserved-name migration and limits are documented in KILNSCRIPT.md.
- Replaced the detached instruction-debugger UI with actual Play pause/frame-step/restart controls and live entity-state inspection. Player restarts/transitions rebuild worlds and reset timing.
- Shared acceptance covers independent enemy state/properties/arrays, projectile/collision/trigger/signal/transition, source-vs-player-asset parity, authoring isolation, and stop callbacks. Negative tests cover module cycles, recursive functions, and imported runtime errors. `just check`, full `just test` (560 passed), native editor/player builds, and focused physics-enabled acceptance pass. GUI interaction remains pending on a graphical runner.

### 2026-11-05 — Task 7 native audio and player preferences

- Replaced empty-player registration with real resource decoding. SFX triggers
  overlap, music loops or completes per asset, and playback supports live master/
  bus gain, mute/solo, pause/resume/stop/replay and bounded voices.
- Audio Editor authors the Project library and mixer, imports project-relative
  files without overwriting collisions, and previews outside Play. Native editor
  and player configure the same assets once, including archive bytes.
- Scene/entity scripts share audio and player settings through success and VM
  error paths. Versioned preferences preserve typed game-specific options and
  volume overrides across restart, separately from sample save slots.
- Tests cover decoded/mixed PCM, real author/save/reopen/KAR/player parity,
  invalid imports, future/corrupt/unwritable settings, integer precision and
  callback context restoration. Native listening/GUI and browser parity remain
  separate pending acceptance. See STATE.md for this session's executed checks.

### 2026-11-05 — Task 7 saved native UI screens

- Added project-backed screens/widgets/fonts with stable IDs, responsive anchors/
  pivots/stretch, hierarchical clipping, focus/navigation, back/pause state and
  edge-triggered input. Both UI editor tabs now edit one saved document, with UI
  document undo/redo. Native editor/player use the same session/layout/quads.
- Added real font atlas sampling/metrics, script events and dynamic labels, input
  capture, paused gameplay scheduling, context restoration and draw/data bounds.
  Native apps enable existing fonts/controller backends by default; CI installs
  ALSA/udev headers before checks/builds. No new external runtime dependency.
- Real native player verification on Xvfb caught and fixed launcher Escape
  interception. Mouse/keyboard title/settings/pause/game-over now pass, including
  script events, persisted volume, one retained scene session and captured PNGs.
  Automated editor toolbar/menu interaction remained inconclusive; full editor
  interaction and physical controller acceptance remain pending.
- Added `samples/ui2d_editor`, real save/reopen/KAR/runtime parity tests and a
  repeatable `tests/native/ui_smoke.py`. General world saves, browser integration
  and Tasks 8–9 are not completed by this checkpoint. See STATE.md for checks.

### 2026-11-05 — Requested editor design and 3D visual preview

- Paused Task 7 for the user's supplied native editor design: charcoal chrome,
  global toolbar, docked hierarchy/inspector/workspace and thumbnail asset shelf.
  Existing authoring/history/project commands remain in use. See ADR-0057,
  [EDITOR_DESIGN.md](EDITOR_DESIGN.md) and `samples/editor_design`.
- Added isolated orbitable 3D demonstration geometry with transient inspector
  properties. It is clearly labeled preview-only and never modifies 2D project
  data; this does not complete or resume full 3D engine implementation.
- Fixed hierarchy click/drag conflicts, nested dock layout and an actual native
  Play deadlock caused by re-entering egui Context from an input-lock closure.
- Native inspector edit/undo/redo/save/restart, 3D orbit/lighting/project isolation
  and actual Play/movement/pause/frame-step/Stop now pass on Xvfb/software
  graphics. Added repeatable `tests/native/editor_smoke.py`, CI integration and
  inspected screenshots in `docs/evidence/editor-design/`. The full two-scene,
  specialist-panel, saved-UI authoring and device gates remain separate.
- `just check` and all **631 tests** passed. Task 7's generic game restoration,
  browser parity and Tasks 8–9 remain unfinished; resume Task 7 next.


### 2026-11-06 — Task 7 general gameplay saves and browser integration

- Replaced sample-only gameplay save/load with bounded versioned active-scene
  snapshots, typed global progress, script lifecycle/locals/state, live entities
  and resources, UI stack/overrides and atomic staged restore. Native atomic writes
  and browser localStorage preserve prior slots on corrupt/future/quota failures.
  Settings remain independent. ADR-0058 and GAME_SAVING_2D.md define compatibility.
- Browser now uses shared scene/module/entity lifecycle and saved fonts/UI,
  controller polling, correctly scaled pointer/touch input and Web Audio voices,
  music/buses/preferences. Ordered key queues preserve rapid menu transitions.
- Fixed native UI Designer press-origin selection and cumulative drag movement.
  Added real GUI author/save/reopen/Play acceptance and a restartable save demo.
- `just check`, **641 tests (zero skipped)**, default native binaries, minimal
  native checks, strict WASM Clippy and WASM build passed. Real editor, native
  directory/KAR restart/save/resume, existing editor/menu regressions and Chromium
  rendering/controller/touch/audio/storage checks passed. CI now runs these paths.
- Retained inspected evidence in `docs/evidence/task7-complete/`. Physical-controller,
  audible-device and remote CI results remain unverified. Task 8's real editor
  export/distribution workflow was not started; all of 2D is not release-complete.


### 2026-11-06 — Task 8 native export transaction

- Replaced simulated UI export and source-tree-dependent CLI scaffolds with a
  real saved-project/native-runtime transaction. Added runtime handshake, bounded
  diagnostics, real status, job-specific receipts, staged verified archives,
  cancellation/fallback/reaping and previous-output preservation. No Cargo runs
  during export/play. Unsupported platforms/installer flags report failure.
- Real editor Save & Export saves an unsaved X=88 scene edit, honors the output
  field, verifies output and preserves it after a failing rebuild. Actual UI Cancel
  cleans an active handshake. The editor exports with an empty tools PATH.
- Relocated game runs directly from /tmp without --archive or Rust on PATH;
  menus/fonts/audio commands/settings and fresh-process save/resume pass. This is
  software/cloud-runner evidence, not clean-machine or physical-device proof.
- `just check`, **649 tests (zero skipped)**, native binary builds, minimal native
  checks and strict WASM Clippy pass. CI runs editor export plus relocated-game
  acceptance. Debug runtime size is recorded separately from performance/resource budgets.
- Browser exporter, portable editor packages, installers/signing, other-platform
  and actual clean-machine dependency acceptance remain Task 8 follow-up.


### 2026-11-10 — Task 8 prepared browser export and relocated page acceptance

- Added Browser to the editor's supervised Save & Export workflow. Prepared
  JS/WASM bundles carry embedded engine/font metadata and SHA-256 identities;
  missing/mixed/changed bundles fail before publishing output. No Cargo/Node/
  wasm-bindgen runs during export. Runtime preparation is a separate build step.
- Published relative project resources and a real loading/start/error shell. Asset
  loading never advances gameplay; trusted Play resumes audio before the first
  tick. Held Enter repeats cannot activate the underlying menu. Asset/loader
  failures show reload feedback. Authored physics/joints/material overrides fail
  explicitly instead of silently changing game behavior on the browser host.
- Actual native editor Browser export saved an unsaved X=88 edit, validated output,
  preserved previous output on failure, moved the artifact outside the repo and
  made the source project unavailable. Chromium serves only that folder at a
  nested URL and passes loading, consent, controller/touch, audio, preferences,
  reload/save/resume, corrupt/quota failures and failure/retry assertions.
- `just check`, **652 tests (zero skipped)**, native builds/minimal checks, WASM
  build and strict target Clippy passed. Native export/cancel/relocated gameplay
  and direct browser runtime/menu/movement regressions also passed. CI now runs
  the actual editor/browser export path. Selected evidence is in task8-browser/.
- The interrupted native launch was retried through normal approval after the
  usage limit cleared. Managed Chrome blocks file:// by administrator policy;
  that guidance branch is unverified, and file launching is not supported.
  Clean-machine desktop/release/other-platform/device and portable editor gates
  remain open. Task 9/full 3D were not started.

### 2026-11-10 — Task 8 portable development editor checkpoint

- Added prebuilt portable editor packaging and device-free tool identities. The
  installation contains editor/CLI/player, prepared JS/WASM, three working
  templates and the saved-UI example. Authoring/export requires no development
  toolchain; the producer uses Python standard library, Cargo metadata inputs and
  the preparing compiler's notices. ADR-0061 records scope and remaining gates.
- Added offline CLI installation verification of host/version/features,
  sizes/SHA-256/modes and browser metadata; reject missing/extra files,
  incompatible identities, traversal/duplicates and symlinks/special files.
  Private staging, normalized tar/gzip metadata and exclusive publication protect
  existing outputs. Checksums are unsigned integrity checks, not authentication.
- Preserve dependency expressions/attributions, original and pinned reference
  notices, Cargo.lock, font licenses, compiler standard-library copyright report
  and checksum-verified MPL sources. Inventory covers 433 host/WASM workspace
  packages conservatively, including build/test packages, with 17 source archives.
  Internal manifests with no package license remain null; exact binary provenance
  and release licensing review remain open.
- CLI empty template now has a real main scene/script and initialization refuses
  nonempty output. Moved installation in a path containing spaces initializes and
  exports all three templates to Native/Browser with empty PATH. Installed editor
  runs from an unrelated cwd; actual unsaved edits/save/export, prior-output
  preservation/cancel and both relocated gameplay/save/resume suites pass.
- Record Linux ELF/glibc observations (this build needs GLIBC 2.39), document
  runtime requirements and keep clean-machine, performance/resource budgets/provenance,
  Windows/macOS and physical devices separate. CI is configured to prepare and
  exercise the bundle; remote execution is unobserved. Task 9 remains unstarted.

### 2026-11-10 — Owner removes package-size goals

- Removed editor/player/WASM/mobile package-size ceilings from active goals and
  commands (ADR-0062). Size observations remain informational; performance,
  startup, memory/resource limits, correct exports and compatibility still matter.
- Historical ADRs/evidence retain their original measurements and are superseded
  on size policy by ADR-0062. Task 8 continues with clean Linux userspace testing.

### 2026-11-10 — Task 8 clean Ubuntu runtime acceptance

- Added a pinned Ubuntu 24.04 runtime-only image and a repeatable Docker runner.
  Runtime container is offline/unprivileged with read-only root; only archive and
  evidence are mounted. No Rust/Cargo/compiler/Git, repository or development
  sysroot is available. The build context contains only four test scripts and
  runtime requirements. CI's producing job is pinned to Ubuntu 24.04 for GLIBC
  compatibility and runs this acceptance before uploading evidence.
- Clean launch exposed a runtime-loaded libxkbcommon-x11 requirement that ldd
  cannot detect. Added a shared Ubuntu package list to both the image and shipped
  bundle instructions. The test verifies shipped/tested requirements match.
- Final isolated run passes installation hashes/identities/library resolution,
  real editor Player X=88 save/native export, prior-output preservation, UI
  cancellation, relocated game menus/settings/save/restart/Continue and corrupt
  slot preservation, followed by unchanged-installation verification.
- 657 Rust tests, four producer regressions and just check pass. Selected actual
  editor/resumed-game captures and machine-readable evidence are retained in
  evidence/task8-clean-linux. Same-kernel/software graphics/ALSA null evidence
  does not replace physical GPU/audio/controller or other OS testing. Browser
  gameplay was not rerun inside this native-only image. Size is not a gate.

### 2026-11-10 — Task 8 capability-checked browser physics

- Default kiln-web builds enable existing kiln-runtime/Rapier physics, sharing
  the native fixed-step/collider/joint/script/save paths. The physics feature can
  be disabled explicitly; no new third-party engine/runtime dependency was added.
- RuntimeInfo/prepared WASM metadata now declare a strict physics capability.
  Legacy JSON/three-line metadata defaults false. Body/joint/prefab/solid-tilemap
  or declared-physics projects fail export with an incapable selected runtime;
  direct browser API startup also returns an actionable missing-feature error.
  Sprite material overrides remain rejected. ADR-0063 records the contract.
- Added a playable physics fixture and relocated HTTP/WASM acceptance: gravity,
  floor/mask filtering, sensor enter/exit/raycast, solid tiles, kinematics,
  distance/revolute/prismatic joints (including slider limits/motor), upward
  velocity across save/page reload/Start once, rebuilt contacts, fresh scene
  colliders and cross-scene restore. Pixel assertions verify the blue body at
  its actual simulated floor heights. Both exporter and browser reject an
  intentionally physics-disabled runtime. CI runs the fixture and opt-out build.
- Added the playground to portable bundles; packaging still needs no development
  tools during authoring/export. Package size remains informational, not a goal.
  Actual editor Browser Save & Export and prior movement/UI/audio/storage
  acceptance pass against the upgraded runtime. Task 8 remains open for material
  integration, release provenance and physical/other-platform checks.

### 2026-11-10 — Task 8 shared authored sprite materials

- Replaced unused/broken material pipeline plumbing with one fixed-vertex/custom
  WGSL fragment contract across editor/native/browser. Real scene submission now
  resolves authored material IDs. Pipelines use the actual target format, correct
  quad/instance layouts and compatible camera/sprite/material texture bindings.
  Adjacent batches retain transparent painter order and reset default pipelines.
- Added typed Float/Vec2/Vec3/Vec4/Color specialization and named Texture defaults/
  overrides. PNG paths use the normal project resource/import pipeline. Export
  validates definitions, references, parameter types/finiteness, WGSL stages/
  bindings and resources before publication; backend errors are captured.
- Prepared runtime identity now advertises materials; old JSON/metadata defaults
  false, so incapable runtimes fail explicitly instead of dropping materials.
  Editor templates/entry names are corrected, project shader data stays
  authoritative, and Shader Editor can create parameters/materials and edit
  overrides. Actual scene preview replaces the former fake shader preview box.
- Native/editor and relocated HTTP browser pixel tests cover defaults/overrides,
  named textures, transparent default/custom/default order, flip/rotation and
  cross-scene save/load. Invalid shader loading feedback and old-runtime rejection
  are exercised. CI runs both paths; material playground joins portable examples.
  ADR-0064 and SPRITE_MATERIALS_2D.md record scope. Package size is not a gate.
- Remaining Task 8 work is physical/other-platform validation, startup/frame-time/
  memory evidence on reference hardware and release provenance/licensing; shader
  host APIs alone are no longer the evidence for material support.

### 2026-11-10 — Task 8 captured release-profile build

- Added `just portable-release`: snapshot actual source/Cargo.lock, build native
  and default WASM offline/locked into a fresh directory, retain command logs and
  package a checked unsigned local build record. Prebuilt preparation remains
  available with explicit preparation-only provenance. ADR-0065 documents scope.
- Built/verified a real release candidate; native debug assertions are disabled.
  Clean Ubuntu acceptance now also checks material pixels in installed editor/
  player. Native export/cancel/failure, moved gameplay/save/Continue and unchanged
  installation verification pass. Release browser material/physics/UI regressions
  and actual installed-editor Browser export/relocated loading/consent/save pass.
- `just check` passed; 665 Rust and ten producer/build tests passed. Selected
  record/results/inspected captures: evidence/task8-release-build. See
  BUILD_PROVENANCE.md for precise source/archive identity and limitations.
- Task 8 remains open for physical/other-platform/reference-hardware measurements,
  internal licensing, trusted provenance/repeated binary equality and signing.
  Package size remains informational. Task 9/full 3D have not started.

### 2026-11-11 — Task 8 repeated release payload verification

- Added validated source replay into fresh source/target directories and a
  comparison command that rehashes actual source/lock, artifacts, receipts and
  logs. Fixed commands are executed by the driver; records cannot supply code.
  Incompatible inputs, changed/missing/linked files or unequal native/WASM files
  fail, including a nonzero CLI exit and failed report. Existing evidence is kept.
- Rebuilt the exact earlier release snapshot on the same host/toolchain. All six
  editor/CLI/player/JS/WASM/descriptor payloads match byte for byte. Execution
  records make whole archive hashes different; this distinction is recorded.
  Both completed installations pass their CLI verifier. See ADR-0066,
  BUILD_PROVENANCE.md and evidence/task8-repeated-build/README.md.
- `just check` passed; 665 Rust and 19 producer/replay/comparison tests pass.
  Physical/other-platform/reference-hardware, licensing and trusted CI/toolchain/
  SDK/dependency provenance/signing remain Task 8 gates. Size has no ceiling;
  Task 9/full 3D have not started.

### 2026-11-11 — Task 8 hosted candidate verification prepared

- Tracked the root Cargo.lock used by actual local release evidence. Added a
  pinned feature/default-branch and manually dispatched release-candidate workflow:
  native quality/tests, two fresh locked/offline builds, actual payload comparison,
  browser material/physics, moved installation/gameplay and clean Ubuntu gates.
- Read-only build/test precedes a separate official Actions provenance job. It
  downloads accepted files without checking out/executing engine code, creates
  GitHub build-origin attestations and retains the bundle as an Actions artifact.
  Origin attestation is separate from OS signing and physical compatibility.
- Local actionlint, 665 Rust / 19 Python tests and actual source-capture/permission
  preflight pass. Hosted completion must be observed after pushing; prepared
  workflow files alone are not a claim that remote gates passed. ADR-0067 and
  BUILD_PROVENANCE.md document the workflow and verification command.

### 2026-11-11 — Task 8 hosted acceptance observed; publication blocked

- Actual GitHub run 43088213774 for 462d17f passed native quality/tests, both fresh
  release builds, six-payload comparison, browser material/physics, moved editor /
  export/gameplay and clean Ubuntu gates. Source/lock/workflow changes were pushed
  and the remote SHA verified.
- Candidate and diagnostic artifact uploads failed; attestation was not reached.
  Exact upload diagnostics are unavailable through the anonymous view, and this
  environment explicitly blocks api.github.com. Saved a restricted-network draft
  adding that host, requiring review/save/publish; it is not applied. This is a
  blocked publication result, not a fully successful CI/attestation claim.
- Recorded public step conclusions/preflight in evidence/task8-hosted-ci. Added
  PHYSICAL_DEVICE_TEST.md with real-desktop/device/manual measurement checks for
  Intel HD 520 and other machines; no physical result is implied. Licensing policy
  remains pending the owner's answer to README:TBD versus workspace MIT/Apache.

### 2026-11-11 — Task 8 artifact diagnostics and early service gate

- The owner's screenshot identifies both upload failures as CreateArtifact HTTP
  404, "workflow run not found", and warns of forced Node 20-to-24 execution.
  Storage/quota, token expiry and runtime causation are not established.
- Updated immutable official action pins to native Node 24 releases. Added a
  five-minute artifact upload/download/byte-comparison gate before engine builds;
  errors remain strict. Cached Cargo tool installation metadata with binaries.
- Local actionlint, just check and 19 Python tests pass; actual preflight shell
  scripts accept unchanged downloads and reject altered/missing bytes. Hosted
  candidate publication/attestation still require a successful observed run.
- Pushed 0b9a67b and verified the exact remote SHA. Hosted run 43107026471's actual
  probe upload/download/byte comparison passed; candidate build is in progress.
  Local Rust suite rerun: 665 passed, zero failures/skips. This is early artifact
  service evidence, not yet complete candidate publication or attestation.

### 2026-11-11 — Task 8 hosted publication and origin attestation passed

- Observed run 43107026471 at 0b9a67b through all three jobs: overall success in
  47m 9s. Native quality/tests, both captured builds, actual six-payload equality,
  browser material/physics, moved installation/export/gameplay and clean Ubuntu
  acceptance all passed. Candidate/diagnostic uploads and separate attestation /
  bundle upload passed too. Original 404 causation remains unproven.
- Saved public step conclusions/overall success in
  evidence/task8-hosted-ci/hosted-run2-complete.json. Candidate contents/signatures
  were not independently downloaded/verified here; no such claim is inferred.
- Added a successful-run download recipe verifying all five files against that
  run's source/signer SHA before extraction/installation checking. Linked it from
  the main download, getting-started, portable-editor and physical-test docs.
- just check and documentation Bash/official CLI flag/archive-root checks pass.
  No engine/workflow changes this session. Project licensing, physical/reference
  hardware, other-platform distribution and OS signing keep Task 8 open. Task 9
  remains unstarted; package size remains informational.

### 2026-11-12 — Task 8 Windows portable candidate and project license

- Windows run 43215982526 at 3ff72d3 completed successfully in 7m 29s. MSVC static
  CRT/DLL inspection, ZIP/notices, moved installation/corruption rejection,
  all-template native/browser CLI exports, source-unavailable game relocation,
  visible editor/player client captures and normal close all passed. Candidate
  and separate origin-attestation/bundle uploads passed too.
- Fixed Windows ZIP original-name normalization, UTF-8 metadata reads and uppercase
  SYSTEMROOT capture lookup. Local 29 Python tests, 665 Rust tests, just check and
  actionlint pass. Compiler caches preserve strict downstream acceptance; Windows
  uses prebuilt preparation, not the captured Linux replay contract.
- Owner delegated license choice: MIT OR Apache-2.0 retained with full root grants,
  consistent metadata across all 34 workspace packages and original third-party /
  font notices preserved. Actual local packaging verifies both project grant bytes.
- Recorded public success/failure history and download instructions in
  WINDOWS_PORTABLE.md and evidence/task8-windows. Candidate signatures/captures
  were not independently downloaded/reviewed here. Refreshed Linux run 43215982528
  remains in progress; full new Linux success is not yet claimed.
- Remaining Task 8: automated game runtime-notice propagation (manual copy documented),
  full Windows GUI/physical tests, macOS, SDK/dependency review and OS signing.
  Task 9/full 3D stay unstarted; package size remains informational.

### 2026-11-12 — Task 8 verified runtime notice propagation

- Completed native/browser notice copying from the selected runtime's verified
  installation. Exact original bytes and runtime payload hashes are inventoried
  in a receipt-bound manifest; CLI/editor verify integrity off the UI thread.
  Modified/incomplete installations fail without publishing; loose developer
  runtimes warn explicitly. No dependency or execution protocol change (ADR-0072).
- Local moved development installation passes all-template native/browser exports,
  GUI Save & Export with unsaved edits, overwrite preservation, native UI Cancel,
  source-unavailable relocation and actual native/browser gameplay/save/resume.
  Every export preserves 1514 original notice files; final installation verification
  passes. Seven new integrity tests, all 672 Rust tests (zero skipped), 29 Python
  tests, just check, WASM check and workflow lint pass. See
  [notice evidence](evidence/task8-runtime-notices/README.md).
- Refreshed licensed Linux run 43215982528 at 3ff72d3 completed successfully in
  45m 2s, including captured/repeated builds, moved/clean-Ubuntu acceptance, upload
  and separate origin attestation. This prior run predates automatic copying and
  does not validate the new feature; new hosted results remain separate.
- Task 8 stays open for full Windows GUI/physical tests, macOS, remaining
  SDK/dependency/exact-linked-code review and OS signing. No release-size goal or
  public GitHub Release was introduced; Task 9/full 3D remain unstarted.
