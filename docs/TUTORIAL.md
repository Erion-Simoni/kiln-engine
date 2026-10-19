# Kiln Tutorial — Building Your First Game

This tutorial walks you through building a simple coin collector game using Kiln. The game demonstrates the core engine features: rendering, ECS, input handling, and the game loop.

## Prerequisites

- Rust 1.87 or later
- Just (install via `cargo install just`)
- Basic Rust knowledge

## Game Overview

**Coin Collector** is a simple 2D game where:
- You control a blue square with arrow keys
- Collect gold coins that appear on screen
- Win by collecting 10 coins
- Gameplay takes 1-2 minutes

The sample is located in `samples/collector/`.

## Step 1: Project Structure

First, let's understand the Kiln workspace structure:

```
kiln/
├── kiln-core/      # Math, time, shared types
├── kiln-render/    # wgpu renderer, sprite batching
├── kiln-runtime/   # Game loop, ECS world, input
├── samples/
│   └── collector/  # Our game
```

The sample's `Cargo.toml` depends on:
- `kiln-core` for `Transform` component
- `kiln-render` for rendering and camera
- `kiln-runtime` for game loop, ECS, and sprite component

## Step 2: Setting Up the Window

Every Kiln game starts with a window and event loop using `winit`:

```rust
use winit::{
    event_loop::EventLoop,
    window::Window,
};

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = CollectorApp::default();
    let _ = event_loop.run_app(&mut app);
}
```

The `CollectorApp` implements `winit::application::ApplicationHandler` to handle window events.

## Step 3: GPU Initialization

When the window is created, we initialize the GPU renderer:

```rust
fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    let window = Arc::new(
        event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title("Kiln — Coin Collector Sample")
                .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32)),
        ).expect("window"),
    );
    pollster::block_on(self.init_gpu(window));
}
```

The `init_gpu` method creates:
- A wgpu instance and surface
- A `Renderer` for wgpu integration
- A `SpriteRenderer` for 2D sprite batching

## Step 4: Game State

We track game state in a simple struct:

```rust
#[derive(Clone)]
struct GameState {
    player_pos: (f32, f32),
    coins: Vec<(f32, f32)>,
    score: u32,
    input_held: (bool, bool, bool, bool), // up, down, left, right
    game_over: bool,
}
```

For production games, you'd use the ECS world more extensively. This sample keeps it simple for learning.

## Step 5: Input Handling

Arrow keys update the input state:

```rust
fn handle_key(&mut self, event: KeyEvent) {
    use winit::keyboard::Key::Named;
    match event.logical_key {
        Named(NamedKey::ArrowUp)    => self.state.input_held.0 = event.state.is_pressed(),
        Named(NamedKey::ArrowDown)  => self.state.input_held.1 = event.state.is_pressed(),
        Named(NamedKey::ArrowLeft)  => self.state.input_held.2 = event.state.is_pressed(),
        Named(NamedKey::ArrowRight) => self.state.input_held.3 = event.state.is_pressed(),
        _ => {}
    }
}
```

## Step 6: Game Loop Update

The fixed timestep game loop ensures consistent physics:

```rust
fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
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
