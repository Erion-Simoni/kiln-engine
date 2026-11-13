# Switchyard

Restore power by lighting all five lamps. Each reusable switch toggles three
lamps; lit lamps turn off when toggled again. Three circuits have different
starting patterns. The blue marker indicates the selected switch.

- Left/Right or A/D selects A, B or C. Hold briefly for a gameplay tick.
- Click the switch button to activate it. Its label lists the affected lamps.
- A toggles lamps 1, 3, 5; B toggles 1, 3, 4; C toggles 1, 2, 3.
- Every activation autosaves; Continue restores the board, selected switch and move
  count. The win screen advances to the next circuit, then cycles back to one.
- Escape opens the shared pause/settings menu.

Open `games/switchyard/kiln.toml` in Kiln and use Play Project Start. The ordinary
controller in `scripts/ui.ks` uses reusable lamp/switch functions, sprite roles,
typed game data, saved UI events, music/effects and game saves. Its authored
project identity keeps progress independent from Emberwatch and other examples.

Minimal solutions are A-B-C, B-C, then A-C. Automated acceptance solves all three
through keyboard selection and mouse activation, checks exact move counts and
continues across a fresh native process and browser reload. Source and KAR
regressions additionally compare restored world and game data after a real save.
