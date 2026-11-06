# Saved UI demo

Open `kiln.toml` in the editor to inspect saved title, HUD, pause, settings and game-over screens. In the player:

```bash
cargo run -p kiln-player -- --archive samples/ui2d_editor --game-name ui2d-demo
```

- Title: Start, Settings or Continue saved game; mouse, arrows/Tab and Enter/Space, or a gamepad.
- Settings: Sound 25% persists player volume and changes the button text.
- Start: shows health/inventory and moves the blue actor. Take 7 damage plays two overlapping effects. Escape opens Pause and freezes gameplay.
- Pause: Resume, Settings, Game over or Save game. Restart the player and use
  Continue to restore the actor, health, inventory and script lifecycle. F5/F9 also
  save/load slot 0. A failed save/load displays the error and retains the old slot.

An attached UI controller starts looping music while the title is paused and
handles button events separately from ordinary gameplay. The scene script guards
movement with `script_phase() == "update"`. Audio assets are the synthesized
Task 7 tones. The renamed DejaVu font subset is covered by `assets/fonts/LICENSE.txt`;
normal builds/tests require no system font or FontTools installation.

Both UI Designer and UI Layout Editor edit the same saved project document.
See `docs/UI_AUTHORING_2D.md` for layout, parent clipping, custom fonts, UI undo,
script APIs and authoring. `docs/GAME_SAVING_2D.md` describes versioned saves,
limits and compatibility. The UI workflow test generates its own four-screen
baseline fixture in a temporary directory; this checked-in demo extends it with
general game saving and a HUD.

The acceptance test also packs and reloads a real KAR and compares source/packed
runtime geometry/events/settings without requiring a display or audio device.
`tests/native/task7_smoke.py` and `task7_editor_smoke.py` check actual saving,
reopening and Play. `tests/web/task7.cjs` checks the browser workflow.
The optional `tests/native/ui_smoke.py` checks real native rendering/menu clicks
on a virtual display. Player preferences live outside this project in the chosen
`--game-name` profile; editor Play uses `.kiln-save/` next to the project.
