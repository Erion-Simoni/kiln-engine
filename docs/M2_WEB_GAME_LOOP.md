# M2 Web Game Loop — Design Doc

**Problem:** `kiln-web/src/lib.rs` is a stub. It initializes a console panic hook, reports `crossOriginIsolated` / worker availability, and runs a background ping job — but it draws nothing. The 2D-first gate (M2) promises a working 2D web export (WASM + WebGPU/WebGL2). This doc specifies how to wire `kiln-web` to the real game loop so a Kiln 2D game runs in the browser.

**Scope:** Add a `run` entry point that creates a `wgpu` surface on a `<canvas>`, initializes `Renderer` + `SpriteRenderer`, loads a project (`kiln.toml` + PNG assets), runs the `GameLoop` + `SliceSession`, and presents each frame. No `cargo` is available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-web/src/lib.rs` | `#[wasm_bindgen(start)]` sets panic hook; `run()` logs diagnostics only |
| `kiln-web/src/threading.rs` | Workers ping / single-thread fallback; cross-origin isolation checks |
| `kiln-render::Renderer` | `async fn new(&Instance, &Surface, width, height)` — adapter, device, queue, surface config, tier, post pipeline |
| `kiln-render::SpriteRenderer` | `new(&Device, &Queue, format)`, `begin_frame/push/push_textured/flush`, `create_texture_rgba` |
| `kiln-render::Camera` | Orthographic camera, `new(width, height)`, `pan`, `zoom` |
| `kiln-render::TextureId` | `WHITE` = 1×1 white texel; `create_rgba8_texture(device, queue, label, w, h, rgba)` |
| `kiln-runtime::GameLoop` | Fixed 1/60s timestep `tick(update)` |
| `kiln-runtime::SliceSession` | Runs KilnScript once per frame; `step(world, input, dt)` |
| `kiln-runtime::GameWorld` | hecs wrapper; `spawn(components)` |
| `kiln-core::Project` | `from_toml`, scenes, entities, components (Transform/Sprite/Role/Tilemap) |
| `kiln-assets::image` | `decode_png(bytes) -> Image { width, height, pixels }` |

---

## Design

### Entry point

`kiln-web/src/lib.rs` keeps `#[wasm_bindgen(start)]` for the panic hook. `run()` becomes the async main:

```rust
#[wasm_bindgen]
pub async fn run(canvas_id: &str, project_path: &str) {
    console_error_panic_hook::set_once();
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let canvas = document
        .get_element_by_id(canvas_id)
        .expect("canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("canvas element");

    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let surface = instance.create_surface(/* web-sys canvas surface */).await.expect("surface");

    let mut app = WebApp::new(&instance, &surface, width, height, project_path).await;
    app.run();  // requestAnimationFrame loop
}
```

> **wgpu-on-web surface:** wgpu 22 with `web-sys` creates a surface from an `HtmlCanvasElement`. The exact `create_surface` call signature depends on the `web-sys`/`winit` versions pinned in `Cargo.toml` (wgpu 22.1, winit 0.30.13). Verify against `kiln-render/src/renderer.rs` (desktop surface creation) when implementing.

### WebApp

```rust
struct WebApp {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    camera: Camera,
    world: GameWorld,
    slice: Option<SliceSession>,
    input: InputState,  // (input_x, input_y) accumulated from JS events
}
```

- `renderer` owns device/queue/surface config and tier (auto-selected from the GPU adapter).
- `sprite_renderer` targets `renderer.scene_format()` (the intermediate post target when post is enabled, else the swap-chain format).
- `camera` is an orthographic `Camera::new(width, height)` matching the canvas size.
- `slice` is the KilnScript `SliceSession` for the scene's script (from `Project`).

### Input

Wire DOM events into `InputState`:
- `keydown`/`keyup` → arrow keys / WASD → `get_input_x()` / `get_input_y()` axis (-1 to 1)
- `mousemove` → pointer position
- `mousedown`/`mouseup` → mouse buttons
- `wheel` → scroll

The slice runtime reads `input_x`/`input_y` each frame via `get_input_x()` / `get_input_y()`.

### Frame loop (requestAnimationFrame)

```rust
fn frame(&mut self) {
    // 1. Fixed-timestep update
    self.game_loop.tick(|dt| {
        if let Some(slice) = &mut self.slice {
            let input = (self.input.axis_x(), self.input.axis_y());
            let _ = slice.step(&mut self.world, input, dt.as_secs_f64());
        }
    });

    // 2. Render
    let frame = self.surface.get_current_texture().expect("surface");
    let view = frame.texture.create_view(&Default::default());
    let mut encoder = self.device.create_command_encoder(&Default::default());

    let target = self.renderer.scene_target().unwrap_or(&view);
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("main"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.08, b: 0.10, a: 1.0 }),
                store: wgpu::StoreOp::Store,
            },
            ..Default::default()
        })],
        ..Default::default()
    });

    self.sprite_renderer.begin_frame();
    for (transform, sprite) in self.world.world_mut().query::<(&Transform, &Sprite)>() {
        self.sprite_renderer.push(SpriteInstance {
            position: [transform.position.x, transform.position.y],
            size: sprite.size,
            color: sprite.color,
            uv_rect: sprite.uv_rect,
        });
    }
    self.sprite_renderer.flush(&mut pass, &self.camera, &self.queue);
    drop(pass);

    if self.renderer.has_post() {
        self.renderer.apply_post(&mut encoder, &view);
    }

    self.queue.submit([encoder.finish()]);
    frame.present();

    // Schedule next frame
    request_animation_frame(self);
}
```

### Texture loading

PNG assets are referenced by project-relative path in `Project` (e.g. `art/hero.png`). On the web, fetch them via `web_sys` `Request`/`Response`, decode with `kiln_assets::decode_png`, upload with `sprite_renderer.create_texture_rgba`.

```rust
async fn load_texture(device, queue, sprite_renderer, path: &str) -> Result<TextureId, ...> {
    let resp = wasm_bindgen_futures::JsFuture::from(
        web_sys::window().unwrap().fetch_with_str(path)
    ).await?;
    let buffer = wasm_bindgen_futures::JsFuture::from(
        resp.dyn_into::<web_sys::Response>()?.array_buffer()?
    ).await?;
    let bytes: Vec<u8> = js_sys::Uint8Array::new(&buffer).to_vec();
    let image = kiln_assets::decode_png(&bytes)?;
    let id = sprite_renderer.create_texture_rgba(
        device, &queue, path, image.width, image.height, &image.pixels
    );
    Ok(id)
}
```

### Asset path resolution

The `kiln.toml` manifest lists scenes and entity textures as project-relative paths (`art/hero.png`). On the web, these are fetched relative to the page origin (or the WASM package's public asset folder). The editor and player use the same `texture_paths(project)` collector (`kiln-player/src/textures.rs`), so the web host can preload every referenced texture before spawning.

### Post-processing / tiers

`Renderer::new` auto-selects `RenderTier` from the adapter:
- Low → no post pipeline
- Medium → bloom + fog
- High → bloom + fog + outline + LUT

`renderer.has_post()` gates `apply_post`. `scene_target()` returns the intermediate target when post is enabled, else `None` (render to swap chain). The sprite pipeline targets `scene_format()` so it always renders into the right target.

### Output size budget

M2 budget: Web WASM < 1.5 MB brotli-compressed (`docs/BUDGETS.md`). Keep `kiln-web` dependency footprint minimal:
- No editor crates (`kiln-editor`, `egui`, `egui_dock`)
- Feature-gate physics/audio off for web
- Measure with `wasm-opt -Oz` + brotli (`just size`)

### Acceptance criteria (testable)

1. **Command:** `cargo build --target wasm32-unknown-unknown --release -p kiln-web` produces `kiln_web.wasm`.
2. **Command:** `wasm-opt -Oz -o out.wasm kiln_web.wasm && brotli out.wasm` → `out.wasm.br` < 1.5 MB.
3. **Runtime:** A minimal HTML page with a `<canvas id="game">` calls `KilnWeb.run("game", "kiln.toml")` and renders a sprite at the world origin.
4. **Runtime:** Arrow keys / WASD move the sprite (input axis reaches `SliceSession`).
5. **Runtime:** `get_coin_count()` / `collect_coin()` work; HUD score updates.
6. **Headless:** `cargo test -p kiln-web` passes (the existing `threading` tests run on any platform).

### What stays out of scope (M2)

- glTF / 3D (M29+)
- Audio host calls (Part 2b)
- Physics host calls (Part 2c)
- Animation / VFX host calls (Part 2d, 2e)
- Full UI widget system in-game (Part 2f)

## See Also

- `docs/BUDGETS.md` — Web WASM <1.5 MB brotli budget
- `kiln-web/src/lib.rs` — current stub to replace
- `kiln-render/src/renderer.rs` — desktop surface/device setup (mirror for web)
- `demo/src/main.rs` — desktop sprite batch reference (same `SpriteInstance` layout)
- `kiln-player/src/textures.rs` — `texture_paths(project)` collector

