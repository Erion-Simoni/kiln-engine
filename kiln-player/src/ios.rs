//! iOS staticlib for kiln-player
//! Uses winit's iOS backend (UIKit integration)
//!
//! To build for iOS device:
//!   cargo build --release --target aarch64-apple-ios -p kiln-player
//!
//! To build for iOS simulator (Apple Silicon):
//!   cargo build --release --target aarch64-apple-ios-sim -p kiln-player
//!
//! The staticlib exports `start_winit_app()` for Objective-C to call

use kiln_core::Transform;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{GameLoop, GameWorld, InputState, Sprite};
use log::info;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{Touch, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;

/// GPU state that is only available after the window is created.
struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
}

struct PlayerApp {
    gpu: Option<GpuState>,
    camera: Camera,
    game_loop: GameLoop,
    world: GameWorld,
    input: InputState,
}

impl PlayerApp {
    fn new() -> Self {
        let mut world = GameWorld::new();

        // Create a simple demo scene (like desktop demo)
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
        }
    }

    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface: wgpu::Surface<'static> = instance
            .create_surface(window.clone())
            .expect("create surface");

        let renderer = Renderer::new(&instance, &surface, size.width, size.height).await;
        let sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.format());
        sprite_renderer.upload_quad(renderer.queue());

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
        });

        info!("GPU initialized for iOS");
    }

    fn handle_touch(&mut self, touch: Touch) {
        match touch.phase {
            TouchPhase::Started => {
                self.input
                    .touches
                    .insert(touch.id, (touch.location.x, touch.location.y));
            }
            TouchPhase::Moved => {
                self.input
                    .touches
                    .insert(touch.id, (touch.location.x, touch.location.y));
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.input.touches.remove(&touch.id);
            }
        }
    }

    fn update(&mut self) {
        self.game_loop.tick(|_dt| {
            // Demo scene — static for now
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

        gpu.sprite_renderer.begin_frame();
        for (transform, sprite) in self.world.world_mut().query_mut::<(&Transform, &Sprite)>() {
            gpu.sprite_renderer.push_textured(
                sprite.texture,
                SpriteInstance {
                    position: [transform.position.x, transform.position.y],
                    size: sprite.size,
                    color: sprite.color,
                    uv_rect: sprite.uv_rect,
                },
            );
        }

        let gpu = self.gpu.as_mut().unwrap();
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
    }
}

impl ApplicationHandler for PlayerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("iOS app resumed");

        // Create window on resume (required for iOS)
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kiln Game")
                        .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32)),
                )
                .expect("window"),
        );

        pollster::block_on(self.init_gpu(window));
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        info!("iOS app suspended");

        // Drop GPU state on suspend (required for iOS)
        self.gpu = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.gpu.as_ref().map(|g| g.window.id()) != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Touch(touch) => {
                self.handle_touch(touch);
            }
            WindowEvent::Resized(size) => {
                self.camera = Camera::new(size.width as f32, size.height as f32);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.update();
        self.render();
    }
}

/// Entry point for iOS staticlib
/// Called from Objective-C main in Xcode project
#[no_mangle]
pub extern "C" fn start_winit_app() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting Kiln iOS player");

    let event_loop = EventLoop::new().unwrap();
    let mut player = PlayerApp::new();
    event_loop.run_app(&mut player).unwrap();
}
