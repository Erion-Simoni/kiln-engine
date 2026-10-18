//! Tilemap sample — renders a 5×5 checkerboard of coloured tiles.
//!
//! Demonstrates that the sprite batch is generic enough to stand in for a
//! tilemap renderer: 25 tile sprites drawn in a single GPU draw call.
//!
//! Arrow keys: pan.  Space/Shift: zoom in/out.

use kiln_core::Transform;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{GameWorld, GameLoop, Sprite};
use std::sync::Arc;
use winit::{
    event::KeyEvent,
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::Window,
};

const TILE_W: f32 = 64.0;
const TILE_H: f32 = 64.0;
const COLS: u32 = 5;
const ROWS: u32 = 5;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = TilemapApp::default();
    let _ = event_loop.run_app(&mut app);
}

// ---------------------------------------------------------------------------

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
}

struct TilemapApp {
    gpu: Option<GpuState>,
    camera: Camera,
    game_loop: GameLoop,
    world: GameWorld,
}

impl Default for TilemapApp {
    fn default() -> Self {
        let mut world = GameWorld::new();

        // Build a COLS × ROWS checkerboard tilemap
        for row in 0..ROWS {
            for col in 0..COLS {
                let x = col as f32 * TILE_W + TILE_W / 2.0;
                let y = row as f32 * TILE_H + TILE_H / 2.0;
                // Checkerboard pattern: light vs dark
                let (r, g, b) = if (row + col) % 2 == 0 {
                    (0.55, 0.72, 0.55) // light green
                } else {
                    (0.22, 0.38, 0.22) // dark green
                };
                world.spawn((
                    Transform::new(x, y),
                    Sprite::new(r, g, b, 1.0).with_size(TILE_W - 2.0, TILE_H - 2.0),
                ));
            }
        }

        Self {
            gpu: None,
            camera: Camera::new(800.0, 600.0),
            game_loop: GameLoop::new(),
            world,
        }
    }
}

impl TilemapApp {
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface: wgpu::Surface<'static> = instance
            .create_surface(window.clone())
            .expect("surface");

        let renderer = Renderer::new(&instance, &surface, size.width, size.height).await;
        let sprite_renderer = SpriteRenderer::new(renderer.device(), renderer.format());
        sprite_renderer.upload_quad(renderer.queue());

        self.gpu = Some(GpuState { window, surface, renderer, sprite_renderer });
    }

    fn handle_key(&mut self, event: KeyEvent) {
        use winit::keyboard::Key::Named;
        match event.logical_key {
            Named(NamedKey::ArrowUp)    => self.camera.pan(0.0, 10.0),
            Named(NamedKey::ArrowDown)  => self.camera.pan(0.0, -10.0),
            Named(NamedKey::ArrowLeft)  => self.camera.pan(-10.0, 0.0),
            Named(NamedKey::ArrowRight) => self.camera.pan(10.0, 0.0),
            Named(NamedKey::Space)      => self.camera.zoom(1.1),
            Named(NamedKey::Shift)      => self.camera.zoom(0.9),
            _ => {}
        }
    }

    fn render(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else { return };
        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 { return; }

        gpu.renderer.resize(&gpu.surface, size.width, size.height);

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => { log::warn!("{e}"); return; }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        gpu.sprite_renderer.begin_frame();
        for (transform, sprite) in self.world.world_mut().query_mut::<(&Transform, &Sprite)>() {
            gpu.sprite_renderer.push(SpriteInstance {
                position: [transform.position.x, transform.position.y],
                size: sprite.size,
                color: sprite.color,
            });
        }

        let gpu = self.gpu.as_mut().unwrap();
        let mut encoder = gpu.renderer.device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("tilemap_frame") }
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tilemap_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            gpu.sprite_renderer.flush(&mut pass, &self.camera, gpu.renderer.queue());
        }

        gpu.renderer.queue().submit([encoder.finish()]);
        frame.present();
        gpu.window.request_redraw();
    }
}

impl winit::application::ApplicationHandler for TilemapApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(
                winit::window::WindowAttributes::default()
                    .with_title("Kiln — Tilemap Sample")
                    .with_inner_size(winit::dpi::LogicalSize::new(400u32, 400u32)),
            ).expect("window"),
        );
        pollster::block_on(self.init_gpu(window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            winit::event::WindowEvent::Resized(s) => {
                self.camera = Camera::new(s.width as f32, s.height as f32);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.game_loop.tick(|_dt| {});
        self.render();
    }
}
