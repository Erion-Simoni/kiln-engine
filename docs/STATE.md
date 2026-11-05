# Kiln Session Handoff

**Active focus:** Task 7 saved native UI checkpoint completed on 2026-11-05. Task 7 overall and full 2D release readiness remain unfinished.
**Branch:** `feat/2d-production-workflow`. Previous pushed checkpoint: `4c2fbc1` (audio/player preferences). 3D remains paused; Tasks 8–9 were not started.

## Done

- Added `Project.ui`: saved screens, stable widget IDs, hierarchy, anchors/pivots/stretch, clipping, colors/text/fonts, button actions, focus links and per-scene initial screens. Documents validate references, cycles/depth, geometry and draw/data limits before Play/loading.
- Both UI Designer and UI Layout Editor now edit one project-backed document; removed their unsaved parallel models. Added canvas selection/drag/snapping, properties, ordering, font imports and UI document undo/redo. Opening the panel without input does not dirty the project. UI font moves preserve stable font IDs and update resource references.
- Native editor Play and player use kiln-ui's shared layout/interaction/quads. Fonts use real alpha atlases/metrics and textured UV sampling, with bitmap fallback. UI-only renderers release old atlas textures before loading another library. Preview colors convert linear values correctly for egui.
- Added root/entity `ui_event` callbacks and runtime screen/text/visibility/enabled APIs. Numeric values can populate labels directly. Paused screens stop ordinary updates, physics/animation/VFX and elapsed gameplay time while UI runs; modal input capture suppresses gameplay axes/actions. Callback context is restored on VM errors.
- Added topmost/clipped hit tests, release capture, focus repair/restoration, spatial/Tab navigation, held-key suppression, bounded rapid-click/key edges and focus-loss cancellation. Native apps enable existing font/controller backends by default. Fixed old gamepad polling that appended stale tokens; CI installs ALSA/udev headers before builds.
- Added `samples/ui2d_editor`, real author/save/reopen/KAR/runtime tests, a small licensed font subset, native screenshots and repeatable `tests/native/ui_smoke.py`. Added a Linux native-player CI smoke job with retained evidence. ADR-0056 and UI_AUTHORING_2D.md describe the contract.
- Native player testing caught/fixed Escape routing into the launcher's built-in menu. Added an application-level regression for Escape/Space routing. Authored menus now retain the existing scene session through pause/settings/game-over.
- The prior audio checkpoint remains intact: real decoded/polyphonic audio, looping music, named buses, independent preview, versioned player settings and error-preserving writes. The UI demo exercises music/effects and persisted volume.

## Executed validation

- `just check`: passed (formatting, strict native workspace/all-target Clippy, workspace check and standalone audio sample check).
- `just test`: **622 passed, zero failed/skipped**. Includes old scene/prefab/physics/visual/audio suites plus UI geometry/colors/UVs/events parity, real font alpha/metrics, clipped input, quick taps, navigation, pause/resume, modal capture, error recovery, atomic UI history, resource moves and clean panel opening.
- Native editor/player binary builds: passed. Both individual `--no-default-features` checks: passed. Linux builds use installed runtime libraries through the local development-header sysroot; libudev's local development symlink needed its installed runtime target linked.
- WASM build and regenerated bindings: passed (two existing kiln-assets warnings). Chromium/WebGL rendering/input regression passed: 392 drawn elements, x 119.5 → 141.5 during input, then stopped after release. This does not establish browser saved-UI parity.
- **Real native player UI on Xvfb/software graphics: passed.** The committed smoke script clicks Title → Settings, invokes the volume/text callback, returns/starts gameplay, opens Pause with Escape and navigates to Game over. It verifies actual event traces, persisted values, no validation/panic/script errors and exactly one loaded scene session. Inspected screenshots are retained in docs/evidence/task7-ui/.
- Native editor launched a temporary copy of the sample on Xvfb. Its toolbar/menu click-through remained inconclusive after three automation attempts; logs did not report a Play startup failure. Full panel/drag/save/reopen/Play GUI acceptance is **pending**, not claimed passed. Headless authoring/persistence/runtime parity passes. Investigate the input harness or use a normal graphical desktop before closing that gate.
- CPU-only UI update/layout/draw observation: 1000 title ticks, 26 quads, mean 0.016422 ms / p95 0.016425 ms on this cloud runner. Font subset: 22,352 bytes; atlas: 4 MiB. This excludes VM/audio/physics/GPU time and is not reference-hardware/release evidence.
- Build profiles used `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` to keep generated artifacts within cloud disk space. No source/check was excluded. Remote CI/platform execution remains pending.

## Next

- Finish Task 7's general versioned game data and game/scene/entity/attached-script save/restore, including the intended UI state. The older save_game/load_game path still uses sample-specific SliceState and is not generic restoration.
- Complete full native editor interaction acceptance. Physical-controller navigation and real audible device acceptance also remain open; normalized token tests and ALSA null output do not certify hardware.
- Integrate saved UI, native script/module lifecycle, audio/mixer/preferences and localStorage in the browser host. Its older with_action_map/WebAudio path is still behind native.
- Only then proceed to Task 8's real asynchronous export workflow and Task 9's reference games/release gate. Do not start 3D.

## Known boundaries

- Saved widgets currently cover Panel/Label/Button; lists/scrolling, richer controls, shaping and high-quality SDF text are outside this checkpoint. Fonts bake ASCII/authored/extra characters at 48 px; missing dynamic characters use a fallback. Use explicit extra characters for runtime Unicode text.
- Use Undo UI/Redo UI for UI document history; the global scene Undo shortcut remains separate. Imported resource files remain available when their UI references are undone/removed.
- UI callbacks run with zero gameplay delta; guard normal gameplay with script_phase()=="update". Explicit UI callbacks may intentionally change the world or request scenes. New scene sessions reinitialize UI/audio while retaining player preferences.
- Browser saved UI is not integrated yet. Native screenshot success does not certify clean-machine exports, other platforms or physical hardware. Readiness audit retains prior scripting/spawn/debugger/browser gaps.
- The native player's existing default --game-name KilnGame shares a profile; use a distinct name per game. The demo's Resume demo continues the same world and does not claim save/load or restart.
- Build/Export still simulates success (Task 8). Do not describe archive tests or screenshots as a complete editor export workflow or Unity/Godot/GameMaker parity.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Native editor/player now default to audio/fonts/gamepad (Linux development headers: libasound2-dev and libudev-dev). Xvfb/xdotool were downloaded into the workspace tool sysroot solely for optional native testing. Network and local display/browser sockets use the runner's supported require_escalated permission.
