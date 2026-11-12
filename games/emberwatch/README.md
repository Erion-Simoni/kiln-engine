# Emberwatch

A short top-down relay-defence game. The blue operator has five hull points.
Defeat the red scout (one hit), orange guard (two), and magenta brute (three),
then move into the green relay to win. Enemy contact damages the operator with a
short recovery interval. Losing offers a fresh round; winning saves the mission.

- Move with arrows or WASD. Keep moving to avoid approaching hostiles.
- Click **FIRE / auto-aim**. Only one projectile can be active at a time.
- Escape opens the shared pause menu; Save game records the completed tick.
- Continue on the title screen loads the saved world, enemies and progress.
- Settings persists the quieter volume choice independently from game saves.

Open `games/emberwatch/kiln.toml` in Kiln and use Play Project Start. Gameplay is
in `scripts/ui.ks`, attached to the scene's UI controller; `scripts/main.ks`
intentionally leaves scene-level gameplay to that attachment. These are ordinary
KilnScript functions, entities, sprites, UI events and save/audio APIs.

The native and browser acceptance defeats every hostile with real projectiles,
activates the relay by movement and verifies the resulting saved victory. Rust
regressions also let enemies defeat the player and verify restart restores a
playable round. Both source projects and real KAR archives are replayed.
