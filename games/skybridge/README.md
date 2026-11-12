# Skybridge

A two-level physics platformer with a two-frame run animation, grounded jumping,
a moving platform, a slope, collision layers, sensor gates and a bounded follow
camera. Jump at least once in each level, then cross its gold gate. The first
level transitions to the second real scene; the far gate completes the game.

- Left/Right or A/D moves; Up/W jumps when grounded.
- S saves the completed gameplay tick; L loads on key release.
- R replays the current level. The editor's Play Project Start restarts level one.
- A standalone native player first shows its launcher: press Enter to start.
  Browser export starts after its Play consent button.

Open `games/skybridge/kiln.toml` in Kiln. `main` is Lower Gantry; `scene_1` is Upper
Gantry. The second hero's exposed `stage` property is 2 and its physical gate is
at X=1100. The scene was duplicated through the editor, preserving its own camera
and component data. The `run` clip was built in Animation Editor from the supplied
sheet, assigned in the inspector and saved with both scenes. Controls were bound
in Input Rebinding. Ordinary scripts are `scripts/hero.ks`, `scripts/platform.ks`
and the scene HUD's `scripts/main.ks`.

Regression tests check actual Rapier grounding, jump height, changing animation
UVs and both sensor gates from source and KAR. Native/browser playthroughs cross
the scene transition and save, move, load and compare the restored hero position.
This is a short reference course, not a complete commercial platformer campaign.
