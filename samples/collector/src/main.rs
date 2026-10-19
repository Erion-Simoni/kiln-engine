//! Coin Collector sample — a simple playable game.
//!
//! Collect coins by moving the player square with arrow keys.
//! Collect 10 coins to win.
//!
//! Arrow keys: move player.  Collect 10 coins to win.

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

const PLAYER_SIZE: f32 = 32.0;
const COIN_SIZE: f32 = 16.0;
const WIN_SCORE: u32 = 10;

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct GameState {
    player_pos: (f32, f32),
    coins: Vec<(f32, f32)>,
    score: u32,
    input_held: (bool, bool, bool, bool), // up, down, left, right
    game_over: bool,
}

// ---------------------------------------------------------------------------
// GPU state
// ---------------------------------------------------------------------------

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
}

// ---------------------------------------------------------------------------
// Main app
// ---------------------------------------------------------------------------

struct CollectorApp {
    gpu: Option<GpuState>,
    camera: Camera,
    game_loop: GameLoop,
    world: GameWorld,
    state: GameState,
}

impl Default for CollectorApp {
    fn default() -> Self {
        let state = GameState {
            player_pos: (400.0, 300.0),
            coins: vec![],
            score: 0,
            input_held: (false, false, false, false),
            game_over: false,
        };

        let mut world = GameWorld::new();
        world.spawn((
            Transform::new(state.player_pos.0, state.player_pos.1),
            Sprite::new(0.2, 0.6, 1.0, 1.0).with_size(PLAYER_SIZE, PLAYER_SIZE),
        ));

        Self {
            gpu: None,
            camera: Camera::new(800.0, 600.0),
            game_loop: GameLoop::new(),
            world,
            state,
        }
    }
}

impl CollectorApp {
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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
            Named(NamedKey::ArrowUp) => self.state.input_held.0 = event.state.is_pressed(),
            Named(NamedKey::ArrowDown) => self.state.input_held.1 = event.state.is_pressed(),
            Named(NamedKey::ArrowLeft) => self.state.input_held.2 = event.state.is_pressed(),
            Named(NamedKey::ArrowRight) => self.state.input_held.3 = event.state.is_pressed(),
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

        // Render player
        for (transform, sprite) in self.world.world_mut().query_mut::<(&Transform, &Sprite)>() {
            gpu.sprite_renderer.push(SpriteInstance {
                position: [transform.position.x, transform.position.y],
                size: sprite.size,
                color: sprite.color,
            });
        }

        // Render coins
        for &(x, y) in &self.state.coins {
            gpu.sprite_renderer.push(SpriteInstance {
                position: [x, y],
                size: [COIN_SIZE, COIN_SIZE],
                color: [1.0, 0.8, 0.0, 1.0], // Gold
            });
        }

        let gpu = self.gpu.as_mut().unwrap();
        let mut encoder = gpu.renderer.device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("collector_frame") }
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("collector_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.1, a: 1.0 }),
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

fn rect_intersect(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = CollectorApp::default();
    let _ = event_loop.run_app(&mut app);
}

impl winit::application::ApplicationHandler for CollectorApp {
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
        // Fixed timestep update
        self.game_loop.tick(|dt| {
            if self.state.game_over {
                return;
            }

            let dt_secs = dt.as_secs_f32();

            // Simple manual movement
            const SPEED: f32 = 200.0;
            if self.state.input_held.0 { self.state.player_pos.1 -= SPEED * dt_secs; }
            if self.state.input_held.1 { self.state.player_pos.1 += SPEED * dt_secs; }
            if self.state.input_held.2 { self.state.player_pos.0 -= SPEED * dt_secs; }
            if self.state.input_held.3 { self.state.player_pos.0 += SPEED * dt_secs; }

            // Clamp to screen
            self.state.player_pos.0 = self.state.player_pos.0.clamp(0.0, 800.0);
            self.state.player_pos.1 = self.state.player_pos.1.clamp(0.0, 600.0);

            // Spawn coins periodically
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

            // Update ECS
            for (transform, _) in self.world.world_mut().query_mut::<(&mut Transform, &Sprite)>() {
                transform.position.x = self.state.player_pos.0;
                transform.position.y = self.state.player_pos.1;
            }
        });
        self.render();
    }
}
