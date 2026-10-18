//! Kiln M2 demo — batched sprite rendering with camera pan/zoom.
//!
//! Arrow keys: pan camera.  Space: zoom in.  Shift: zoom out.
//! Renders a 10×10 grid of coloured sprites (100 sprites, 1 draw call).

use kiln_core::{Transform, Vec2};
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{GameLoop, GameWorld, InputState, Sprite};
use std::sync::Arc;
use winit::{
    event::KeyEvent,
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::Window,
};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}

// ---------------------------------------------------------------------------

/// GPU state that is only available after the window is created.
struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
}

struct App {
    gpu: Option<GpuState>,
    camera: Camera,
    game_loop: GameLoop,
    world: GameWorld,
    #[allow(dead_code)]
    input: InputState,
    #[allow(dead_code)]
    marker_pos: Vec2,
}

impl Default for App {
    fn default() -> Self {
        let mut world = GameWorld::new();
        let marker_pos = Vec2::new(400.0, 300.0);

        // 10×10 grid of sprites — 100 entities, drawn in a single GPU call
        for row in 0..10_u32 {
            for col in 0..10_u32 {
                let x = col as f32 * 72.0 + 36.0;
                let y = row as f32 * 56.0 + 28.0;
                let r = col as f32 / 9.0;
                let g = row as f32 / 9.0;
                world.spawn((
                    Transform::new(x, y),
                    Sprite::new(r, g, 0.6, 1.0).with_size(64.0, 48.0),
                ));
            }
        }

        Self {
            gpu: None,
            camera: Camera::new(800.0, 600.0),
            game_loop: GameLoop::new(),
            world,
            input: InputState::new(),
            marker_pos,
        }
    }
}

// ---------------------------------------------------------------------------

impl App {
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        // The surface is 'static because the window is kept alive in `GpuState`.
        let surface: wgpu::Surface<'static> = instance
            .create_surface(window.clone())
            .expect("create surface");

        let renderer = Renderer::new(&surface, size.width, size.height).await;
        let sprite_renderer = SpriteRenderer::new(renderer.device(), renderer.format());
        sprite_renderer.upload_quad(renderer.queue());

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
        });
    }

    fn handle_key(&mut self, event: KeyEvent) {
        use winit::keyboard::Key::Named;
        match event.logical_key {
            Named(NamedKey::ArrowUp) => self.camera.pan(0.0, 20.0),
            Named(NamedKey::ArrowDown) => self.camera.pan(0.0, -20.0),
            Named(NamedKey::ArrowLeft) => self.camera.pan(-20.0, 0.0),
            Named(NamedKey::ArrowRight) => self.camera.pan(20.0, 0.0),
            Named(NamedKey::Space) => self.camera.zoom(1.1),
            Named(NamedKey::Shift) => self.camera.zoom(0.9),
            _ => {}
        }
    }

    fn update(&mut self) {
        self.game_loop.tick(|_dt| {
            // Static grid demo — no movement this milestone
        });
    }

    fn render(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        gpu.renderer.resize(&gpu.surface, size.width, size.height);

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("frame error: {e}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Collect sprite instances from ECS (CPU gather pass)
        gpu.sprite_renderer.begin_frame();
        for (transform, sprite) in self.world.world_mut().query_mut::<(&Transform, &Sprite)>() {
            gpu.sprite_renderer.push(SpriteInstance {
                position: [transform.position.x, transform.position.y],
                size: sprite.size,
                color: sprite.color,
            });
        }

        let gpu = self.gpu.as_mut().unwrap(); // re-borrow after world query
        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame_encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.08,
                            b: 0.10,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            gpu.sprite_renderer
                .flush(&mut pass, &self.camera, gpu.renderer.queue());
        }

        gpu.renderer.queue().submit([encoder.finish()]);
        frame.present();
        gpu.window.request_redraw();

        log::trace!("sprites: {}", gpu.sprite_renderer.queued_count());
    }
}

// ---------------------------------------------------------------------------

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kiln M2 — Sprite Batch Demo")
                        .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32)),
                )
                .expect("window"),
        );
        pollster::block_on(self.init_gpu(window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            winit::event::WindowEvent::Resized(size) => {
                self.camera = Camera::new(size.width as f32, size.height as f32);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.update();
        self.render();
    }
}
