# Kiln Session Handoff

**Active focus:** Task 7 native audio and player preferences checkpoint completed on 2026-11-05. Task 7 as a whole and the complete 2D engine remain unfinished.
**Branch:** `feat/2d-production-workflow`. 3D remains paused. Tasks 8–9 were not started.

## Done

- Replaced empty audio registrations with real decoded PCM, shared among independent SFX voices and one replaceable music voice. Loop/completion, pause/resume, stop/replay, named buses, gain, mute/solo, bounded polyphony and headless advancement are implemented. Malformed gains are normalized before persistence.
- Audio Editor edits `Project.audio_assets` and saved mixer defaults directly. Imports decode/validate, copy external files to project-relative paths, preserve collisions, and register stable asset IDs. Asset moves update audio references. Preview opens its own output outside Play and reports errors; project replacement/Play stops it.
- Native editor/player enable the existing audio feature by default, configure resources once from project files/archive bytes, and open output explicitly. Session creation and all automated mixer tests require no device. Audio opt-out builds remain available.
- Scene/entity callbacks share audio, platform sound callbacks and player preferences, restoring their context even after VM failure.
- Added versioned `preferences.json`, typed `setting_get`/`setting_set`, named-bus volume APIs and persisted audio overrides. Writes flush a temporary file before replacement. Corrupt/newer data blocks writes; failed writes retain previous settings. Preferences are independent of sample score/coins and save slots.
- Added reproducible `samples/audio2d_editor`, real direct/packed-player PCM acceptance, import/move/error/settings tests, ADR-0055 and `docs/AUDIO_AUTHORING_2D.md`. Reconciled the completion plan/readiness audit; marked the old audio design as historical. CI installs ALSA headers before native builds/tests and checks audio opt-out.

## Executed validation

- `just check`: passed (formatting, strict native workspace/all-target Clippy, workspace check and standalone audio sample check).
- `just test`: **603 passed, zero failed/skipped**. Includes decoded PCM overlap/live gain/mute/solo, looping/replacement/completion, pause/resume/stop/replay, voice limits and headless completion; actual import/save/reopen/KAR/player-VM parity; preferences restart/integer precision/corrupt/future/unwritable files; callback context restoration on errors.
- Native editor and player binary builds: passed. Both individual `--no-default-features` package checks: passed.
- `cargo build -p kiln-web --target wasm32-unknown-unknown`: passed with two existing kiln-assets warnings. Regenerated WASM bindings and ran Chromium/WebGL smoke: passed (392 drawn elements; movement changed x 119.5 → 131.5 and stopped after release).
- Standalone audio sample through ALSA's **null** output: passed. A real native output stream opened, decoded/played the generated WAV and completed without an error. This is not a listening test.
- Generated the audio sample through its acceptance test. Both synthesized WAV files total 62,488 bytes; decoded mono PCM totals 124,800 bytes and is shared among voices. Full-file music decoding remains a memory consideration.
- The first full builds exhausted the runner's 63 GB disk with old/new debug artifacts. Removed only regenerable target/debug artifacts and reran successfully with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`. No source/check was excluded. The local ALSA development symlink also needed its target linked to the installed system runtime library.

## Next

- Continue Task 7: persist and run editor-authored UI screens/widgets, anchors/fonts/buttons, keyboard/controller focus/navigation and script events in editor/player.
- Add general versioned game data and game/scene/entity/attached-script save/restore; validate title/pause/settings/game-over and save/resume acceptance. The older `save_game`/`load_game` path still uses sample-specific SliceState and is not general restoration.
- Complete browser audio/mixer/settings parity and a localStorage adapter. Browser currently has its older WebAudio sound callback.
- Perform real audible device and native GUI preview/authoring acceptance on a graphical machine. Then proceed to Task 8's actual export process and Task 9's reference games/release gate.

## Known issues and boundaries

- Full 2D release readiness is not established. Tasks 1–6 retain documented native workflow acceptance and earlier scripting/spawn/debugger/browser follow-ups; see `2D_READINESS_AUDIT.md`.
- No display server or real audio device is available here. Null-output/offline tests do not certify speakers/headphones, native clicks, saved custom UI/controller screens or clean-machine distribution. Remote CI/platform execution is pending.
- Music decodes fully into memory; spatial audio, device selection/hot-plug and streaming are deferred. A new scene session reinitializes playback, retaining saved preferences.
- The native player's existing default `--game-name KilnGame` shares a save/settings profile; supply a distinct name per game. Preview uses authored mixer defaults; gameplay uses persisted player overrides.
- Build/Export still simulates success (Task 8); do not represent real KAR acceptance as a finished editor export workflow. Do not start 3D or claim Unity/Godot/GameMaker parity.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. In this runner, use the three build-profile variables above to limit regenerable disk usage. GitHub and browser loopback operations use the supported `require_escalated` permission. The prior pushed baseline was `01fd40f`; this checkpoint belongs on the same feature branch.
