//! Textured sprite sample — atlas UVs + batch-by-texture.
//!
//! Builds a 2×2 procedural atlas (four coloured tiles) and draws four sprites
//! from it in **one** draw call, plus a solid white-tinted quad (second call).
//! Logs `draw_calls` each frame (expect 2).

use kiln_render::{
    build_grid_atlas_rgba, camera::Camera, Renderer, SpriteInstance, SpriteRenderer, TextureId,
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = SpritesApp::default();
    let _ = event_loop.run_app(&mut app);
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    atlas: TextureId,
    cell_uvs: Vec<[f32; 4]>,
}

#[derive(Default)]
struct SpritesApp {
    gpu: Option<GpuState>,
    camera: Option<Camera>,
    logged: bool,
}

impl SpritesApp {
    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("surface");

        let renderer = Renderer::new(&instance, &surface, size.width, size.height).await;
        let mut sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.format());
        sprite_renderer.upload_quad(renderer.queue());

        let colors: [[u8; 4]; 4] = [
            [220, 60, 60, 255],
            [60, 180, 80, 255],
            [60, 100, 220, 255],
            [240, 200, 40, 255],
        ];
        let (rgba, w, h, cell_uvs) = build_grid_atlas_rgba(32, 32, 2, 2, |col, row, cell| {
            let idx = (row * 2 + col) as usize;
            let c = colors[idx];
            for px in cell.chunks_exact_mut(4) {
                px.copy_from_slice(&c);
            }
        });

        let atlas = sprite_renderer.create_texture_rgba(
            renderer.device(),
            renderer.queue(),
            "demo_atlas",
            w,
            h,
            &rgba,
        );

        self.camera = Some(Camera::new(size.width as f32, size.height as f32));
        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
            atlas,
            cell_uvs,
        });
    }

    fn render(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let Some(camera) = self.camera.as_ref() else {
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
                log::warn!("{e}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        gpu.sprite_renderer.begin_frame();

        let positions = [
            [200.0, 200.0],
            [280.0, 200.0],
            [200.0, 280.0],
            [280.0, 280.0],
        ];
        for (i, pos) in positions.iter().enumerate() {
            let mut inst = SpriteInstance::colored(*pos, [64.0, 64.0], [1.0, 1.0, 1.0, 1.0]);
            inst.uv_rect = gpu.cell_uvs[i];
            gpu.sprite_renderer.push_textured(gpu.atlas, inst);
        }

        // Second texture (white) → second draw call
        gpu.sprite_renderer.push(SpriteInstance::colored(
            [400.0, 240.0],
            [48.0, 48.0],
            [0.9, 0.9, 1.0, 1.0],
        ));

        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sprites_frame"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprites_pass"),
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
                .flush(&mut pass, camera, gpu.renderer.queue());
        }

        gpu.renderer.queue().submit([encoder.finish()]);
        frame.present();

        if !self.logged {
            let calls = self.gpu.as_ref().unwrap().sprite_renderer.draw_call_count();
            let queued = self.gpu.as_ref().unwrap().sprite_renderer.queued_count();
            log::info!("sprites queued={queued} draw_calls={calls} (expect 2)");
            assert_eq!(calls, 2, "atlas batch + white = 2 draw calls");
            self.logged = true;
        }

        self.gpu.as_ref().unwrap().window.request_redraw();
    }
}

impl ApplicationHandler for SpritesApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kiln — Textured Sprites")
                        .with_inner_size(winit::dpi::LogicalSize::new(640u32, 480u32)),
                )
                .expect("window"),
        );
        pollster::block_on(self.init_gpu(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => {
                self.camera = Some(Camera::new(s.width as f32, s.height as f32));
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.render();
    }
}
