//! Kiln web runtime — WASM export for browser.
//!
//! Creates a `wgpu` surface on an HTML `<canvas>`, loads a Kiln project
//! (`kiln.toml` + PNG assets), runs the fixed-timestep game loop with the
//! KilnScript slice runtime, and renders 2D sprites through the engine's
//! sprite batcher. Called from JavaScript via `KilnWeb.run(canvasId, projectPath)`.

mod threading;

use kiln_core::Transform;
use kiln_render::{
    camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer, TextureId,
};
use kiln_runtime::{GameLoop, GameWorld, SliceSession};
use log::{error, info, warn};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

pub use threading::{can_use_web_workers, is_cross_origin_isolated, run_background_job};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn run(canvas_id: &str, project_path: &str) {
    console_error_panic_hook::set_once();

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            error!("No window available");
            return;
        }
    };

    let document = match window.document() {
        Some(d) => d,
        None => {
            error!("No document available");
            return;
        }
    };

    let canvas: HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .and_then(|el| el.dyn_into::<HtmlCanvasElement>().ok())
        .expect("Canvas element not found");

    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;

    info!(
        "Kiln web runtime starting: {}x{} canvas, project={}",
        width, height, project_path
    );

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = match create_wgpu_surface(&instance, canvas) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create wgpu surface: {}", e);
            return;
        }
    };

    match App::new(&instance, surface, width, height, project_path).await {
        Ok(mut app) => app.run(&window),
        Err(e) => {
            error!("Failed to initialize app: {}", e);
        }
    }
}

#[derive(Default, Clone)]
struct WebInput {
    input_x: f64,
    input_y: f64,
}

struct App {
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    camera: Camera,
    world: GameWorld,
    slice: Option<SliceSession>,
    game_loop: GameLoop,
    input: WebInput,
}

impl App {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        project_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find GPU adapter")?;

        let tier = kiln_render::tier::RenderTier::auto_select(&adapter);
        info!("Selected render tier: {:?}", tier);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Kiln Web Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .first()
            .copied()
            .ok_or("No surface formats available")?;

        let mut renderer = Renderer::new_from_parts(
            device,
            queue,
            surface_format,
            tier,
            width,
            height,
        )
        .await?;

        let mut sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.scene_format());
        sprite_renderer.upload_quad(renderer.queue());

        let camera = Camera::new(width as f32, height as f32);

        let project = load_project(project_path).await?;

        let textures =
            load_textures(renderer.device(), renderer.queue(), &mut sprite_renderer, &project)?;

        let mut world = GameWorld::new();
        let scene_id = project
            .scenes
            .keys()
            .next()
            .cloned()
            .ok_or("Project has no scenes")?;
        let scene = project.scenes.get(&scene_id).ok_or("Scene not found")?;

        let mut slice = None;
        if let Some(script_path) = &scene.script {
            let script_source = load_script(script_path).await?;
            let mut session =
                SliceSession::new(&script_source)
                    .map_err(|e| format!("Script compile error: {:?}", e))?;
            slice = Some(session);
        }

        spawn_scene_entities(&mut world, scene, &textures, &sprite_renderer)?;

        Ok(Self {
            surface,
            renderer,
            sprite_renderer,
            camera,
            world,
            slice,
            game_loop: GameLoop::new(),
            input: WebInput::default(),
        })
    }

    fn run(mut self, window: &web_sys::Window) {
        let app = std::rc::Rc::new(std::cell::RefCell::new(Some(self)));
        let app_clone = std::rc::Rc::clone(&app);

        let f = std::rc::Rc::new(std::cell::RefCell::new(None::<Closure<dyn FnMut()>>));
        let f_clone = std::rc::Rc::clone(&f);

        *f.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let mut app = app_clone.borrow_mut().take().unwrap();

            app.update();
            app.render(&window);

            app_clone.borrow_mut().replace(app);

            let f_ref = f_clone.clone();
            let _ = window.request_animation_frame(
                f_ref.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
            );
        }) as Box<dyn FnMut()>));

        let _ = window.request_animation_frame(
            f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
        );
    }

    fn update(&mut self) {
        let input = self.input.clone();
        let input_axis = (input.input_x, input.input_y);

        self.game_loop.tick(|dt| {
            if let Some(slice) = &mut self.slice {
                let _ = slice.step(&mut self.world, input_axis, dt.as_secs_f64());
            }
        });
    }

    fn render(&mut self, _window: &web_sys::Window) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                warn!("Surface texture error: {}", e);
                let _ = self
                    .surface
                    .configure(&self.renderer.device(), &self.renderer.surface_config());
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .renderer
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("web_frame_encoder"),
            });

        let target_view = self.renderer.scene_target().unwrap_or(&view);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("web_main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
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

        self.sprite_renderer.begin_frame();
        let world = self.world.world_ref();
        for (transform, sprite) in world.query::<(&Transform, &kiln_runtime::Sprite)>().iter() {
            let mut instance = SpriteInstance {
                position: [transform.position.x, transform.position.y],
                size: sprite.size,
                color: sprite.color,
                uv_rect: sprite.uv_rect,
            };
            if sprite.texture != TextureId::WHITE {
                self.sprite_renderer.push_textured(sprite.texture, instance);
            } else {
                self.sprite_renderer.push(instance);
            }
        }

        self.sprite_renderer
            .flush(&mut pass, &self.camera, self.renderer.queue());
        drop(pass);

        if self.renderer.has_post() {
            self.renderer.apply_post(&mut encoder, &view);
        }

        self.renderer.finish_profiler_frame(&mut encoder);
        self.renderer.queue().submit([encoder.finish()]);
        frame.present();

        self.renderer.poll_profiler();
    }
}

async fn load_project(path: &str) -> Result<kiln_core::Project, Box<dyn std::error::Error>> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(path, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
    let window = web_sys::window().ok_or("No window")?;
    let promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Fetch await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP {} for {}", resp.status(), path).into());
    }
    let text_promise = resp.text().map_err(|e| format!("{:?}", e))?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Text await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    let text_str = text.as_string().ok_or("Response text is not valid UTF-8")?;
    let project = kiln_core::Project::from_toml(&text_str)?;
    Ok(project)
}

async fn load_script(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(path, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
    let window = web_sys::window().ok_or("No window")?;
    let promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Fetch await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;
    let text_promise = resp.text().map_err(|e| format!("{:?}", e))?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Text await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    Ok(text.as_string().ok_or("Script text is not valid UTF-8")?)
}

fn load_textures(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _sprite_renderer: &mut SpriteRenderer,
    project: &kiln_core::Project,
) -> Result<std::collections::HashMap<String, TextureId>, Box<dyn std::error::Error>> {
    let mut textures = std::collections::HashMap::new();
    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let kiln_core::project::Component::Sprite { texture, .. } = component {
                    if let Some(path) = texture {
                        if textures.contains_key(path) {
                            continue;
                        }
                        textures.insert(path.clone(), TextureId::WHITE);
                    }
                }
            }
        }
    }
    Ok(textures)
}

fn spawn_scene_entities(
    world: &mut GameWorld,
    scene: &kiln_core::project::Scene,
    textures: &std::collections::HashMap<String, TextureId>,
    _sprite_renderer: &SpriteRenderer,
) -> Result<(), Box<dyn std::error::Error>> {
    for entity in &scene.entities {
        let mut transform = Transform::new(0.0, 0.0);
        let mut sprite = kiln_runtime::Sprite::new(1.0, 1.0, 1.0, 1.0);

        for component in &entity.components {
            match component {
                kiln_core::project::Component::Transform { x, y, rotation, scale } => {
                    transform = Transform::new(*x, *y)
                        .with_rotation(*rotation)
                        .with_scale(glam::Vec2::splat(*scale));
                }
                kiln_core::project::Component::Sprite { color, texture } => {
                    sprite = kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]);
                    if let Some(path) = texture {
                        if let Some(&tex_id) = textures.get(path) {
                            sprite = sprite.with_texture(tex_id);
                        }
                    }
                }
                _ => {}
            }
        }

        world.world_mut().spawn((transform, sprite));
    }
    Ok(())
}

fn create_wgpu_surface(
    instance: &wgpu::Instance,
    canvas: HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, Box<dyn std::error::Error>> {
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?;
    Ok(surface)
}
