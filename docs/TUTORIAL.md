# Kiln Tutorial — Building Your First 2D Game

This tutorial walks you through building a simple coin collector game using Kiln. The recommended way to make a Kiln game is **data-driven**: you create a project with `kiln-cli init`, author your scene in `kiln.toml`, add your art as PNG files, and write your game rules in **KilnScript** (a small text language that compiles to the engine's IR bytecode and runs on the same VM as node graphs). No Rust code is required for a complete 2D game.

## Prerequisites

- Rust 1.87 or later
- Just (install via `cargo install just`)

## What You'll Build

**Coin Collector** — a simple 2D game where:
- You control a hero sprite with arrow keys or WASD
- Collect gold coins that appear on screen
- A HUD shows your score; press F5 to save, F9 to load
- Gameplay takes 1–2 minutes

The finished sample lives in `samples/vertical_slice/`.

## Step 1: Create a New Project

Use `kiln-cli init` to scaffold a new project from the `empty_2d` template:

```bash
cargo run --bin kiln-cli -- init --name mygame --template empty_2d
```

This creates a `mygame/` directory with:

```
mygame/
  kiln.toml          # Project manifest (window, scenes, entities)
  art/               # Your PNG assets go here
  scripts/           # Your KilnScript goes here
```

The `empty_2d.ron` template lives in `templates/` at the repo root.

## Step 2: Add Your Art

Drop PNG images into your project directory (e.g. `mygame/art/`). Kiln decodes any PNG colour type (grayscale, palette, RGB, RGBA, 16-bit, `tRNS`) into straight RGBA8 and uploads it to the GPU as a sprite texture.

Reference assets by their project-relative path with `/` separators (e.g. `art/hero.png`). This works identically on Windows, macOS and Linux.

## Step 3: Author Your Scene in `kiln.toml`

The project manifest declares your window, scenes, entities and their components. Here's a minimal scene with a player and three coins:

```toml
id = "mygame"
name = "My Game"
version = "0.1.0"

[window]
width = 640
height = 480
vsync = true

[scenes.main]
id = "main"
name = "Main"
script = "scripts/main.ks"

[[scenes.main.entities]]
id = "player"
name = "Player"
components = [
  { type = "Transform", x = 120.0, y = 200.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/hero.png" },
  { type = "Role", role = "player" },
]

[[scenes.main.entities]]
id = "coin1"
name = "Coin 1"
components = [
  { type = "Transform", x = 240.0, y = 260.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/coin.png" },
  { type = "Role", role = "coin" },
]

[[scenes.main.entities]]
id = "coin2"
name = "Coin 2"
components = [
  { type = "Transform", x = 400.0, y = 180.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/coin.png" },
  { type = "Role", role = "coin" },
]
```

Component reference:
- **`Transform`** — `x`, `y` (world position), `rotation` (radians), `scale` (uniform).
- **`Sprite`** — `color` ([r, g, b, a]), `texture` (optional asset path; omitted = solid colour quad).
- **`Role`** — `role` (a gameplay name like `"player"` or `"coin"`, used by scripts to find entities by purpose).

## Step 4: Write Your Game Rules in KilnScript

KilnScript runs once per frame. It reads *facts* about the frame (input axes, elapsed time, score, nearby pickups) and emits *commands* (move the player, collect a pickup, set the status message). The engine applies the commands after the script returns.

Create `mygame/scripts/main.ks`:

```ks
// KilnScript: collect the coins.
// Runs once per frame. The engine gives input, delta time and world state;
// the script decides what happens.

let speed = 120.0;
let dt = get_delta_time();
let dx = get_input_x() * speed * dt;
let dy = get_input_y() * speed * dt;
move_player(dx, dy);

if collect_coin() {
    add_score(10);
    set_status("PICKED UP A COIN");
}

if get_coin_count() == 0 {
    set_status("ALL COINS COLLECTED");
}
```

Available builtins:
- `get_input_x()` / `get_input_y()` — input axis, -1 to 1 (arrow keys / WASD)
- `get_delta_time()` — seconds since the last frame
- `get_score()` / `add_score(n)` — read and add to the score
- `get_coin_count()` / `collect_coin()` — remaining pickups / collect one in reach
- `move_player(dx, dy)` — move the player by delta
- `set_status(msg)` / `get_status()` — show a message on the HUD
- `print(...)` — print to the console

The pickup radius is 24 world units; a coin counts as collected when the player's box overlaps it.

## Step 5: Run Your Game

Run it in the standalone player:

```bash
cargo run -p kiln-player -- --archive mygame
```

Or open it in the editor: open `mygame/kiln.toml` (File → Open Project) and press **Play** in the scene view.

Controls:
| Key | Action |
|-----|--------|
| Arrow keys / WASD | Move the hero |
| F5 | Save the game |
| F9 | Load the game |

## Step 6: Save and Load

The slice state (score, elapsed time, player position, collected coins) is serialised to JSON and written to your platform's save folder (AppData on Windows, Application Support on macOS, XDG on Linux). Press F5 to save, F9 to load — both work in the player and the editor.

## What's Next?

- **Tilemaps:** See `docs/TILEMAPS.md` for chunked tilemaps with solid-tile collision.
- **KilnScript reference:** See `docs/KILNSCRIPT.md` for the full language reference.
- **Node graphs:** Visual node graphs compile to the same IR as KilnScript (`kiln-graph` crate).
- **Post-processing:** Bloom, fog, outline and LUT grading in `kiln-render` (tier-dependent).
- **UI:** `kiln-ui` provides buttons, labels, lists and nine-slice panels for in-game HUDs.
- **Save system:** `kiln-save` provides versioned save slots with migration.
- **Packaging:** `kiln-cli export --platform <windows|macos|linux>` produces a standalone game; see `docs/PACKAGING.md`.

See `docs/ARCHITECTURE.md` for the full crate map and `docs/ROADMAP.md` for planned features.ctiveEventLoop) {
    self.game_loop.tick(|dt| {
        // Update player position based on input
        const SPEED: f32 = 200.0;
        let dt_secs = dt.as_secs_f32();

        if self.state.input_held.0 { self.state.player_pos.1 -= SPEED * dt_secs; }
        if self.state.input_held.1 { self.state.player_pos.1 += SPEED * dt_secs; }
        if self.state.input_held.2 { self.state.player_pos.0 -= SPEED * dt_secs; }
        if self.state.input_held.3 { self.state.player_pos.0 += SPEED * dt_secs; }

        // Clamp to screen bounds
        self.state.player_pos.0 = self.state.player_pos.0.clamp(0.0, 800.0);
        self.state.player_pos.1 = self.state.player_pos.1.clamp(0.0, 600.0);

        // Spawn coins if fewer than 3 on screen
        if self.state.coins.len() < 3 {
            let x = (rand::random::<f32>() * 700.0) + 50.0;
            let y = (rand::random::<f32>() * 500.0) + 50.0;
            self.state.coins.push((x, y));
        }

        // Collision detection
        let player_rect = (
            self.state.player_pos.0 - PLAYER_SIZE / 2.0,
            self.state.player_pos.1 - PLAYER_SIZE / 2.0,
            PLAYER_SIZE,
            PLAYER_SIZE,
        );

        self.state.coins.retain(|&(cx, cy)| {
            let coin_rect = (cx - COIN_SIZE / 2.0, cy - COIN_SIZE / 2.0, COIN_SIZE, COIN_SIZE);
            let collision = rect_intersect(player_rect, coin_rect);
            if collision {
                self.state.score += 1;
                if self.state.score >= WIN_SCORE {
                    self.state.game_over = true;
                }
            }
            !collision
        });

        // Update ECS to sync transform with game state
        for (transform, _) in self.world.world_mut().query_mut::<(&mut Transform, &Sprite)>() {
            transform.position.x = self.state.player_pos.0;
            transform.position.y = self.state.player_pos.1;
        }
    });
    self.render();
}
```

## Step 7: Rendering

Rendering uses the sprite batch renderer:

```rust
fn render(&mut self) {
    let Some(gpu) = self.gpu.as_mut() else { return };

    gpu.sprite_renderer.begin_frame();

    // Render player from ECS
    for (transform, sprite) in self.world.world_mut().query_mut::<(&Transform, &Sprite)>() {
        gpu.sprite_renderer.push(SpriteInstance {
            position: [transform.position.x, transform.position.y],
            size: sprite.size,
            color: sprite.color,
        });
    }

    // Render coins from game state
    for &(x, y) in &self.state.coins {
        gpu.sprite_renderer.push(SpriteInstance {
            position: [x, y],
            size: [COIN_SIZE, COIN_SIZE],
            color: [1.0, 0.8, 0.0, 1.0], // Gold
        });
    }

    // Flush sprites in one draw call
    gpu.sprite_renderer.flush(&mut pass, &self.camera, gpu.renderer.queue());
}
```

The sprite batch renderer efficiently draws all sprites in a single GPU draw call.

## Step 8: Building and Running

To build and run the sample:

```bash
# Build the sample
cargo build --bin collector

# Run the sample
cargo run --bin collector
```

Or use the justfile:

```bash
just run
```

Then select the collector sample.

## What's Next?

This tutorial covered:
- Window setup with winit
- GPU initialization with wgpu
- Input handling
- Fixed timestep game loop
- Sprite batch rendering
- Simple collision detection

For more advanced features, explore:
- **KilnScript**: Text-based scripting language (`kiln-script` crate)
- **Node graphs**: Visual programming (`kiln-graph` crate)
- **Post-processing**: Bloom, LUT, fog effects (`kiln-render` post module)
- **Audio**: OGG/Opus playback with rodio (`kiln-runtime` audio feature)
- **Physics**: 2D physics with rapier2d (`kiln-runtime` physics feature)

See `docs/ARCHITECTURE.md` for the full crate map and `docs/ROADMAP.md` for planned features.
