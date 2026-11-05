# Saved UI demo

Open `kiln.toml` in the editor to inspect four saved UI screens. In the player:

```bash
cargo run -p kiln-player -- --archive samples/ui2d_editor --game-name ui2d-demo
```

- Title: Start or Settings, using mouse, arrows/Tab and Enter/Space, or a gamepad.
- Settings: Sound 25% persists player volume and changes the button text.
- Start: closes UI and moves the blue actor. Escape opens Pause and freezes it.
- Pause: Resume, Settings or Game over. Game over's Resume demo continues the
  existing world; generic save/load/restart is still unfinished.

An attached UI controller starts looping music while the title is paused and
handles button events separately from ordinary gameplay. The scene script guards
movement with `script_phase() == "update"`. Audio assets are the synthesized
Task 7 tones. The renamed DejaVu font subset is covered by `assets/fonts/LICENSE.txt`;
normal builds/tests require no system font or FontTools installation.

Both UI Designer and UI Layout Editor edit the same saved project document.
See `docs/UI_AUTHORING_2D.md` for layout, parent clipping, custom fonts, UI undo,
script APIs, native verification and remaining browser/editor GUI acceptance.

Regenerate manifest/scripts/assets with:

```bash
KILN_TASK7_UI_SAMPLE_OUT=samples/ui2d_editor cargo test -p kiln-player --test ui_workflow authored_screens_survive_restart_archive_and_editor_player_gameplay
```

The acceptance test also packs and reloads a real KAR and compares source/packed
runtime geometry/events/settings without requiring a display or audio device.
The optional `tests/native/ui_smoke.py` checks real native rendering/menu clicks
on a virtual display. Player preferences live outside this project in the chosen
`--game-name` profile; editor Play uses `.kiln-save/` next to the project.
