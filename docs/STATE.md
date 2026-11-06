# Kiln Session Handoff

**Active focus:** Task 7 audio, saved UI, general gameplay saves and settings implementation completed on 2026-11-06. Physical-device and remote CI checks remain separate.
**Branch:** `feat/2d-production-workflow`. Base checkpoint: `badba72` (native editor design and isolated 3D preview). This session completes Task 7 only; Task 8 export/distribution and full 3D remain unstarted.

## Done

- Added general gameplay schema v1 independent of sample score/coins. Editor/player F5/F9 and scene/entity `save_game/load_game` now capture/restore active-scene world state, stable IDs, spawn/delete, parents/nonuniform transforms, sprite resources/layout/UVs, roles, physics/velocity/joints, live tile data, animation timing, root/attached VM locals/state/properties/lifecycle, shared typed progress, queued signals/scene requests and logical UI navigation/overrides. Texture handles are rebound rather than serialized. ADR-0058 precedes the architecture change; GAME_SAVING_2D.md documents the exact contract and limits.
- Added `game_get/game_set`, `has_save`, `save_status/save_error` and paused-UI completion/failure events. Script requests run after completed-tick commands; staged validation/compilation/resource resolution occurs before replacing the live world. Normal scene changes carry custom progress, including values written by entity Stop callbacks. Runtime-generated IDs use a saved monotonic counter, preventing deleted-ID reuse.
- Native saves use bounded versioned metadata, flushed temporary-file replacement and failed-write cleanup. Corrupt/future/mismatched-project slots are retained. Script source/locals signatures are conservative: changed scripts reject old gameplay slots; there is no migration from legacy sample-only saves. Existing explicit legacy sample APIs remain available separately.
- Added shared platform storage/audio interfaces without a new dependency. Player profiles default to project ID; `--game-name` selects an alternate profile. Editor Play uses `.kiln-save/`; browser storage uses project-scoped localStorage. Preferences remain independent of gameplay and failed writes roll back settings, including audio gains.
- Browser now uses the shared scene/module/entity-script loader, saved UI/fonts, camera viewport/pointer mapping, controller polling and native lifecycle semantics. Web Audio supports bounded overlapping effects, looping music, pause/resume, master/named bus gains and preferences. Browser storage reports denied/quota/corrupt failures while preserving prior slot/live state. Focus loss resets input/clock and suspends audio; user gestures unlock it. Existing dependencies were enabled for fonts and browser APIs.
- Fixed quick keyboard menu events losing order across fixed ticks; queued Cancel/Up/Accept now execute in order, without held keys overtaking pending edges. Fixed UI Designer drags selecting the backdrop after crossing a small label's boundary and resetting displacement each frame; authoring uses press origin and total movement.
- Extended samples/ui2d_editor with saved HUD health/inventory, overlapping damage sounds, Pause Save, title Continue, restart-safe initialization and readable failure feedback. Native directory/KAR and browser restart workflows preserve health 66, inventory brass_key and start-count 1.
- Added automated native editor/player and Chromium Task 7 acceptance, CI steps (including actual KAR packing), strict WASM Clippy and retained selected inspected captures/results in docs/evidence/task7-complete/. Updated scripting/audio/UI docs and 2D_COMPLETION_PLAN.md.

## Executed validation

- `just check`: passed formatting, strict native workspace/all-target Clippy, workspace check and standalone audio sample check.
- `just test`: **641 passed, zero failed/skipped**, including normal scene-transition progress/Stop-callback restoration.
- Native editor/player default-feature binary builds and `cargo check -p kiln-editor -p kiln-player --no-default-features`: passed.
- `cargo clippy -p kiln-web --target wasm32-unknown-unknown -- -D warnings`: passed. Removed existing WASM-only archive warnings so the target is checked without suppressions. WASM build/bindgen passed; browser bundle uses real saved fonts and shared scripts/storage/audio.
- Native Task 7 editor smoke passed: add/drag/undo/redo label, verify `[104,80]` saved offset, reopen process, Play save/load with health/inventory/lifecycle retained and authored UI unchanged. Actual screenshot shows health 66 and brass_key after a fresh Play load.
- Native Task 7 player smoke passed from both project directory and a real packed KAR, including fresh-process resume and corrupt-save preservation. Existing native saved-menu/settings/game-over smoke and editor inspector/history/3D-isolation/Play-control regressions also passed.
- Real Chromium Task 7 smoke passed: browser Gamepad API navigation, exactly one touch activation on offset/scaled canvas, active looping music, overlapping SFX, persisted gain/settings, page reload/Continue, independent preferences and quota/corrupt-save preservation. Existing browser movement/rendering smoke also passed.
- Native tests use Xvfb/software graphics and ALSA null output; browser controller input is simulated through the production Gamepad API path. No physical-controller or audible-device certification is claimed. Remote CI has not run in this workspace.

## Next

- Run the physical-controller and audible speakers/headphones checks on actual target devices, and inspect remote CI artifacts after push.
- Task 8, only when requested: replace the simulated editor Build/Export result with real asynchronous CLI export, truthful exit/output/cancel behavior, clean-machine native/browser artifacts and distribution. Current packed-player/browser runtime acceptance does not certify the editor exporter.
- Task 9 reference games and release gate follow export/distribution. Do not describe the whole engine as Unity/Godot/GameMaker parity.

## Known boundaries

- General saves restore the active logical world. Other scenes load authored definitions; carry persistent quest/inventory/world flags in `game_*` values. Transient audio voices, held input, solver/contact caches and particle instances are not serialized. Music restarts via startup in a fresh process; existing voices survive an in-process load. Script/schema migration requires an explicit future design.
- Browser WebGL/WebGPU rendering capabilities and physics/renderer feature parity remain bounded by earlier tasks; this session does not implement missing 3D or platform features. Browser storage belongs to its origin/profile and can be cleared by the browser/user.
- Saved UI supports Panel/Label/Button, alpha-atlas TTF/OTF and bitmap text. Rich controls, shaping/streaming/SDF text remain outside the contract. Add explicit font characters for dynamic Unicode. UI document Undo/Redo is separate from scene history.
- The editor-only 3D visual preview remains isolated from project/runtime/save/export. Build/Export still simulates success until Task 8. Full two-scene/specialist-panel release acceptance is broader than these Task 7 workflows.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Builds use CARGO_INCREMENTAL=0 and debug-info overrides for disk usage. Native development needs ALSA/udev headers; rootless Xvfb/xdotool live in the workspace tool sysroot. Display/browser sockets and GitHub access use the runner's supported require_escalated permission. Local Chromium tests set CHROMIUM_PATH=/usr/bin/chromium; CI installs the matching Playwright browser. No check/source was excluded or warning suppressed.
