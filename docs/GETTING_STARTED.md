# Getting Started with Kiln — Make a 2D Game

Kiln is a lightweight standalone desktop game engine: editor, runtime and CLI. The recommended way to make a 2D game is **data-driven**: create a project, add PNG art, author your scene in `kiln.toml`, write your rules in **KilnScript**, then run it in the player or the editor. No Rust code is required for a complete 2D game.

## What's Already Built (2D-first gate, complete)

The 2D-first gate (G-1 … G-6) is done:

| Gate | What it means |
|------|---------------|
| G-1 Asset → GPU | PNG → GPU textures, sprite batching, texture-bound sprites |
| G-2 Real tilemap | Chunked tilemaps, camera culling, solid-tile collision, editor painting |
| G-3 VM maturity | KilnScript with `I64`/`F64`/`Bool`/`Str`, strings, locals, tests |
| G-4 Vertical slice | `samples/vertical_slice/` — a complete, script-driven 2D game |
| G-5 Roadmap reconciled | Milestone verdicts match the honesty audit |
| G-6 Editor truth | Engine-drawn scene view, inspector save/load, undo/redo, PIE |

M29 (3D foundation) is in progress — see `ROADMAP_FULL_M14_PLUS.md`.

## Step 1: Prerequisites

- **Rust 1.87+** — `rustup` to install
- **just** — `cargo install just`

The engine is a Rust workspace. Build it once to verify:

```bash
cargo check --workspace
```

## Step 2: Create a New Project

Use `kiln-cli init` with the `empty_2d` template:

```bash
cargo run --bin kiln-cli -- init --name mygame --template empty_2d
```

This creates:

```
mygame/
  kiln.toml          # Project manifest
  art/               # Drop your PNGs here
  scripts/           # Your KilnScript goes here
```

Templates live in `templates/` at the repo root (`empty_2d.ron`, `platformer.ron`, `topdown.ron`).

## Step 3: Add Art

Place PNG files in `mygame/art/`. Kiln decodes every PNG colour type (grayscale, palette, RGB, RGBA, 16-bit, `tRNS`) into straight RGBA8 and uploads it to the GPU.

Reference assets by project-relative path with `/` separators: `art/hero.png`. This is identical on Windows, macOS and Linux.

## Step 4: Author Your Scene (`kiln.toml`)

The manifest declares your window, scenes and entities:

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
components = [
  { type = "Transform", x = 120.0, y = 200.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/hero.png" },
  { type = "Role", role = "player" },
]

[[scenes.main.entities]]
id = "coin1"
components = [
  { type = "Transform", x = 240.0, y = 260.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/coin.png" },
  { type = "Role", role = "coin" },
]
```

Components:
- **Transform** — `x`, `y` position, `rotation` (radians), `scale` (uniform).
- **Sprite** — `color` `[r,g,b,a]`, optional `texture` asset path (omitted = solid colour quad).
- **Role** — `role` string (e.g. `"player"`, `"coin"`); scripts find entities by role.

## Step 5: Write Game Rules (`scripts/main.ks`)

KilnScript runs once per frame:

```ks
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

Builtins: `get_input_x`, `get_input_y`, `get_delta_time`, `get_score`, `add_score`, `get_coin_count`, `collect_coin`, `move_player`, `set_status`, `get_status`, `print`.

See `docs/KILNSCRIPT.md` for the full language reference.

## Step 6: Run

Standalone player:

```bash
cargo run -p kiln-player -- --archive mygame
```

Editor: open `mygame/kiln.toml` (File → Open Project) and press **Play** in the scene view.

Controls: arrow keys/WASD move, F5 saves, F9 loads.

## Step 7: Tilemaps (Optional)

For grid-based levels, see `docs/TILEMAPS.md` — chunked tilemaps with camera culling and solid-tile collision, authored in the editor's tilemap panel.

## Step 8: Export and Package

Build a standalone game for Windows, macOS or Linux:

```bash
cargo run --bin kiln-cli -- export --platform windows --output export/mygame
cargo run --bin kiln-cli -- export --platform macos  --output export/mygame
cargo run --bin kiln-cli -- export --platform linux   --output export/mygame
```

Packaging (installers, code signing, storefront publishing) is documented in `docs/PACKAGING.md`.

## Where to Go Next

| Topic | Doc |
|-------|-----|
| KilnScript language | `docs/KILNSCRIPT.md` |
| Tilemaps with collision | `docs/TILEMAPS.md` |
| First game walkthrough | `docs/TUTORIAL.md` |
| Packaging and signing | `docs/PACKAGING.md` |
| Android export | `docs/ANDROID.md` |
| iOS export | `docs/IOS.md` |
| Architecture / crate map | `docs/ARCHITECTURE.md` |
| Roadmap (M29 = 3D) | `ROADMAP_FULL_M14_PLUS.md` |
| Budgets (size / perf) | `docs/BUDGETS.md` |
| Session handoff / status | `docs/STATE.md` |

## Reference Sample

`samples/vertical_slice/` is a complete, script-driven 2D game — art from disk, KilnScript rules, save/load, HUD. Run it:

```bash
cargo run -p kiln-player -- --archive samples/vertical_slice
```