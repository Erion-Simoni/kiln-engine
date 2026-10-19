//! Particle system demo — 10k particles using CPU simulation.
//!
//! Demonstrates the kiln-vfx particle system with a fountain effect.
//!
//! Particles spawn from the center and fall with gravity.

use kiln_render::{camera::Camera, ParticleRenderer, Renderer, SpriteRenderer};
use kiln_vfx::{emitter::{Emitter, EmitterConfig, EmissionShape}, particle::ParticleData, system::{ParticleSystem, ParticleSystemConfig}};
use std::sync::Arc;
use winit::{event_loop::EventLoop, window::Window};

const SCREEN_WIDTH: f32 = 800.0;
const SCREEN_HEIGHT: f32 = 600.0;

// ---------------------------------------------------------------------------
// GPU state
// ---------------------------------------------------------------------------

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    particle_renderer: ParticleRenderer,
}

// ---------------------------------------------------------------------------
// Main app
// ---------------------------------------------------------------------------

struct ParticleApp {
    gpu: Option<GpuState>,
    camera: Camera,
    particle_system: ParticleSystem,
    time: f32,
}

impl Default for ParticleApp {
    fn default() -> Self {
        let config = ParticleSystemConfig {
            max_particles: 10_000,
        };
        let mut particle_system = ParticleSystem::new(config);

        // Create fountain emitter
        let emitter_config = EmitterConfig {
            shape: EmissionShape::Point,
            rate: 5000.0, // Spawn 5000 particles per second to reach 10k steady state
            speed_range: [100.0, 200.0],
            angle_range: [std::f32::consts::PI * 0.25, std::f32::consts::PI * 0.75], // Upward cone
            lifetime_range: [2.0, 3.0],
            size_range: [2.0, 4.0],
            color: [0.5, 0.7, 1.0, 1.0], // Light blue
            gravity: [0.0, -200.0], // Gravity pulls down
            fade_out: true,
        };
        let mut emitter = Emitter::new(emitter_config);
        emitter.set_position([SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0]);
        particle_system.add_emitter(emitter);

        Self {
            gpu: None,
            camera: Camera::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            particle_system,
            time: 0.0,
        }
    }
}

impl ParticleApp {
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface: wgpu::Surface<'static> =
            instance.create_surface(window.clone()).expect("surface");

        let renderer = Renderer::new(&instance, &surface, size.width, size.height).await;
        let sprite_renderer = SpriteRenderer::new(renderer.device(), renderer.format());
        sprite_renderer.upload_quad(renderer.queue());

        let particle_renderer = ParticleRenderer::new();

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
            particle_renderer,
        });
    }

    fn render(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else { return };
        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        gpu.renderer.resize(&gpu.surface, size.width, size.height);

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("{e}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        gpu.sprite_renderer.begin_frame();
        gpu.particle_renderer.begin_frame();

        // Convert CPU particles to GPU data and queue for rendering
        let particles: Vec<ParticleData> = self
            .particle_system
            .particles()
            .iter()
            .map(|p| ParticleData::from_particle(p))
            .collect();
        gpu.particle_renderer.push_particles(&particles);

        // Render particles as sprites
        for instance in gpu.particle_renderer.instances() {
            gpu.sprite_renderer.push(*instance);
        }

        let gpu = self.gpu.as_mut().unwrap();
        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("particle_frame"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("particle_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.05,
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

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    println!("Particle System Demo");
    println!("===================");
    println!("Rendering 10,000 particles with CPU simulation");
    println!("Close the window to exit");
    println!();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = ParticleApp::default();
    let _ = event_loop.run_app(&mut app);
}

impl winit::application::ApplicationHandler for ParticleApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kiln — Particle System Demo (10k particles)")
                        .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32)),
                )
                .expect("window"),
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
            winit::event::WindowEvent::Resized(s) => {
                self.camera = Camera::new(s.width as f32, s.height as f32);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Fixed timestep update (60 FPS target)
        const TARGET_FPS: f32 = 60.0;
        const DT: f32 = 1.0 / TARGET_FPS;

        self.time += DT;
        self.particle_system.update(DT);

        // Print stats every second
        if (self.time % 1.0) < DT {
            println!(
                "Particles: {} (target: 10,000)",
                self.particle_system.particle_count()
            );
        }

        self.render();
    }
}
