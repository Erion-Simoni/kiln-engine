# Kiln Session Handoff

**Active focus:** User-requested native editor redesign and isolated 3D visual preview completed on 2026-11-05. Task 7 is paused and remains unfinished overall.
**Branch:** `feat/2d-production-workflow`. Base pushed checkpoint: `cb1d688` (saved native UI). This session changes editor presentation/input; it does not resume full 3D engine implementation or Tasks 8–9.

## Done

- Applied the supplied Kiln Editor HTML as a visual reference to the native Rust/egui editor: compact charcoal palette, 1440×900 default window, global authoring/transport toolbar, hierarchy/inspector sidebars, Scene/Game/Animation tabs and Assets/Console/Profiler shelf. No HTML runtime, web wrapper or new dependency. ADR-0057 and EDITOR_DESIGN.md document the design.
- Default docking remains resizable/detachable and resettable. Replaced nested CentralPanels with DockArea::show_inside; reserved chrome before the central workspace, corrected split proportions, clipped the viewport to visible bounds and disabled scene scrollbar expansion. Layout regression covers 1000/1440/1920 widths.
- Existing real commands remain behind gizmos, snapping, framing, scene tabs, hierarchy, inspector, asset operations and Play transport. Replaced hierarchy's competing drag wrapper with one click/drag response; actual widget tests verify selection and drop-to-reparent. Compact inspector controls preserve component/history command paths.
- Added actual project asset thumbnails, category rail and scene cards with double-click opening. Image-card dragging retains existing sprite creation. Thumbnail cache caps at 128 64×64 textures (~2 MiB) and never frees textures referenced by the current frame when full; budget/lifetime regression added.
- Added clearly labeled, editor-only 3D perspective geometry: lit cube/faceted meshes, platform/contact silhouettes, floor grid, selection bounds, gizmo visuals and orientation compass. Orbit/zoom, mesh selection and temporary transform/color/roughness/light controls work. Mode buttons focus Scene; Game and Play use the actual 2D project. 2D authoring tools remain disabled in 3D preview.
- Preview state is independent of Project/ECS/history. 2D mode retains real scene/selection; Save never serializes preview objects or lighting. No 3D import, gameplay, physics, gizmo manipulation, saving or export is claimed.
- Added samples/editor_design: real saved 2D sprites and a movement script matching the reference's blue player, green platform, gold coins and gray obstacle. Tiny original PNGs have a reproducible stdlib generator.
- Real native acceptance found/fixed the pre-existing editor Play freeze: update_play re-entered egui Context::pixels_per_point while inside Context::input. Compute scale first and extract edges without Context calls. Regression covers quick pointer edges, DPI conversion and keyboard capture. Native editor logging now honors RUST_LOG with a warn default.
- Added tests/native/editor_smoke.py and included it in the native UI CI job. The script uses a disposable project and isolated XDG/display profile, edits/saves/undoes/redoes/reopens, verifies 3D orbit/lighting and unchanged project bytes, and measures actual rendered Play/pause/frame-step/Stop behavior. Retained inspected screenshots/results in docs/evidence/editor-design/.
- Prior Task 7 audio and saved game-UI work remains intact: decoded/polyphonic native audio, looping music/buses/persisted preferences; Project.ui screens/layout/fonts/navigation/events, modal capture and paused gameplay scheduling. See AUDIO_AUTHORING_2D.md, UI_AUTHORING_2D.md and samples/ui2d_editor.

## Executed validation

- `just check`: passed formatting, strict native workspace/all-target Clippy, workspace check and standalone audio sample check.
- `just test`: **631 passed, zero failed/skipped**. Includes prior scene/prefab/physics/visual/audio/saved-UI suites and this checkpoint's editor regressions.
- Native editor binary build: passed with the default audio/fonts/gamepad features.
- **Real native editor smoke on Xvfb/software graphics: passed.** Changed Player X 165 → 215, saved, undid/redid and reopened in a fresh editor process. 3D mode/orbit/lighting left the saved 2D project byte-identical. Play moved the rendered player from ~250 to 315 px, pause held 315 px, one frame step advanced to 318 px, and Stop restored ~250 px. No panic/GPU validation/Play errors. This closes the earlier basic toolbar/Play automation blocker; it does not certify every specialist panel or the complete two-scene authoring gate.
- Screenshots were visually inspected against the extracted HTML reference. Its bundled scripts were not used as instructions or shipped in the editor. CI now builds editor/player and runs both native smoke scripts; remote CI execution remains pending.
- Previous checkpoint's native player menus/settings, WASM build and browser movement checks remain recorded in their evidence; these were not rerun for an editor-only change. No shared runtime/renderer source changed in this session.
- Builds used CARGO_INCREMENTAL=0 and CARGO_PROFILE_DEV_DEBUG=0/CARGO_PROFILE_TEST_DEBUG=0 for disk usage. No check or source was excluded.

## Next

- Resume Task 7: general versioned game data plus game/scene/entity/attached-script save/restore, including the intended UI state. The existing save_game/load_game path still uses sample-specific SliceState score/coins.
- Complete saved UI authoring panel/drag/save/reopen/Play GUI acceptance and the full two-scene/specialist-panel graphical workflows. The new smoke covers the ordinary inspector/history/project/transport path, not every panel.
- Finish browser integration of saved UI, native script/module lifecycle, audio/mixer/preferences and localStorage. The older with_action_map/WebAudio browser path remains behind native.
- Retain physical-controller and real audible-device acceptance as open. Normalized token tests and ALSA null output do not certify hardware.
- Only after those gates continue to Task 8's real asynchronous export workflow and Task 9's reference games/release gate. Full 3D implementation remains paused; this session authorized visuals only.

## Known boundaries

- 3D preview properties are temporary and reset with the session; the implementation is an editor triangle-painter demonstration, without a runtime depth-buffer/mesh-import pipeline. The 2D showcase has movement but no collision or game progression.
- Saved game UI currently covers Panel/Label/Button with atlas text. Rich controls, shaping and SDF sampling remain outside that checkpoint. Use explicit extra font characters for runtime Unicode. UI document Undo/Redo remains separate from scene history.
- Native/browser/platform/device readiness gates remain distinct. Build/Export still simulates success (Task 8). Do not describe these editor screenshots or archive tests as complete distribution or Unity/Godot/GameMaker parity.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Native editor/player default to audio/fonts/gamepad (Linux development headers: libasound2-dev/libudev-dev). Rootless Xvfb/xdotool are under the workspace tool sysroot for testing. Local display/browser sockets and GitHub access use the runner's supported require_escalated permission. The workspace restart left a stale test :96 socket; final acceptance used the clean :95 display and shut it down normally.
