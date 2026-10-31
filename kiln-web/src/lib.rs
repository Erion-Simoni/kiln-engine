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
use kiln_ui::TextRenderer;
use log::{error, info, warn};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlImageElement, KeyboardEvent, TouchEvent, AudioContext, GainNode, MouseEvent, WheelEvent};

use image::GenericImageView;

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
    mouse_x: f64,
    mouse_y: f64,
    mouse_pressed: bool,
    mouse_just_pressed: bool,
    mouse_just_released: bool,
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
    /// Web Audio API context for audio playback
    audio_context: Option<web_sys::AudioContext>,
    /// Master gain node
    master_gain: Option<web_sys::GainNode>,
    /// Registered audio buffers (name -> AudioBuffer)
    audio_buffers: std::collections::HashMap<String, web_sys::AudioBuffer>,
    /// Audio file paths registered for playback (name -> URL)
    audio_files: std::collections::HashMap<String, String>,
    /// Canvas width
    canvas_width: u32,
    /// Canvas height
    canvas_height: u32,
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

        // Load project first to get audio assets and scene script
        let project = load_project(project_path).await?;

        let textures =
            load_textures(renderer.device(), renderer.queue(), &mut sprite_renderer, &project, project_path).await?;

        let mut world = GameWorld::new();
        let scene_id = project
            .scenes
            .keys()
            .next()
            .cloned()
            .ok_or("Project has no scenes")?;
        let scene = project.scenes.get(&scene_id).ok_or("Scene not found")?;

        // Initialize Web Audio API
        let audio_context = web_sys::AudioContext::new().ok();
        let master_gain = audio_context.as_ref().map(|ctx| ctx.create_gain()).ok();
        if let Some(gain) = &master_gain {
            gain.connect_with_audio_node(&audio_context.as_ref().unwrap().destination()).ok();
            gain.set_value(1.0);
        }

        // Register all project audio assets and decode them
        let mut audio_buffers = std::collections::HashMap::new();
        let mut audio_files = std::collections::HashMap::new();
        if let Some(ctx) = &audio_context {
            for asset in &project.audio_assets {
                audio_files.insert(asset.name.clone(), format!("{}/{}", project_path_dir(project_path), asset.path));
                if let Ok(array_buffer) = fetch_audio_buffer(&asset.path).await {
                    if let Ok(buffer) = ctx.decode_audio_data_sync(&array_buffer) {
                        audio_buffers.insert(asset.name.clone(), buffer);
                    }
                }
            }
        }

        let camera = Camera::new(width as f32, height as f32);

        let mut slice = None;
        if let Some(script_path) = &scene.script {
            let script_source = load_script(script_path, project_path).await?;
            let mut session =
                SliceSession::new(&script_source)
                    .map_err(|e| format!("Script compile error: {:?}", e))?;
            
            // Set up audio player callback for Web Audio API
            if let Some(audio_ctx) = &audio_context {
                let audio_buffers_clone = audio_buffers.clone();
                let master_gain_clone = master_gain.clone();
                let audio_files_clone = audio_files.clone();
                session.vm.host_mut().audio_player = Some(Box::new(move |name: &str| {
                    // Try to play from pre-decoded buffers first
                    if let Some(buffer) = audio_buffers_clone.get(name) {
                        if let Some(ctx) = audio_ctx.clone().dyn_into::<web_sys::AudioContext>().ok() {
                            if let Ok(source) = ctx.create_buffer_source() {
                                source.set_buffer(buffer);
                                if let Some(gain) = &master_gain_clone {
                                    let _ = source.connect_with_audio_node(gain);
                                } else {
                                    let _ = source.connect_with_audio_node(&ctx.destination());
                                }
                                let _ = source.start();
                            }
                        }
                    }
                    // If not in buffers, try to fetch and play from URL
                    else if let Some(url) = audio_files_clone.get(name) {
                        if let Some(ctx) = audio_ctx.clone().dyn_into::<web_sys::AudioContext>().ok() {
                            let url_clone = url.clone();
                            let ctx_clone = ctx.clone();
                            let master_gain_clone = master_gain_clone.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Ok(array_buffer) = fetch_audio_buffer(&url_clone).await {
                                    if let Ok(buffer) = ctx_clone.decode_audio_data_sync(&array_buffer) {
                                        if let Ok(source) = ctx_clone.create_buffer_source() {
                                            source.set_buffer(&buffer);
                                            if let Some(gain) = &master_gain_clone {
                                                let _ = source.connect_with_audio_node(gain);
                                            } else {
                                                let _ = source.connect_with_audio_node(&ctx_clone.destination());
                                            }
                                            let _ = source.start();
                                        }
                                    }
                                }
                            });
                        }
                    }
                }));
            }
            
            slice = Some(session);
        }

        spawn_scene_entities(&mut world, &project, scene, &textures, &sprite_renderer)?;

        Ok(Self {
            surface,
            renderer,
            sprite_renderer,
            camera,
            world,
            slice,
            game_loop: GameLoop::new(),
            input: WebInput::default(),
            audio_context,
            master_gain,
            audio_buffers,
            audio_files,
            canvas_width: width,
            canvas_height: height,
        })
    }

    fn run(mut self, window: &web_sys::Window) {
        let app = std::rc::Rc::new(std::cell::RefCell::new(Some(self)));
        let app_clone = std::rc::Rc::clone(&app);

        // --- Keyboard input: update WebInput so gameplay receives real input ---
        {
            let app_for_keydown = app_clone.clone();
            let on_keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
                let mut app = app_for_keydown.borrow_mut();
                if let Some(app) = app.as_mut() {
                    match event.key().as_str() {
                        "ArrowRight" | "d" | "D" => app.input.input_x = 1.0,
                        "ArrowLeft" | "a" | "A" => app.input.input_x = -1.0,
                        "ArrowDown" | "s" | "S" => app.input.input_y = 1.0,
                        "ArrowUp" | "w" | "W" => app.input.input_y = -1.0,
                        _ => {}
                    }
                }
            }) as Box<dyn FnMut(KeyboardEvent)>);
            let _ = window
                .add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
            on_keydown.forget();

            let app_for_keyup = app_clone.clone();
            let on_keyup = Closure::wrap(Box::new(move |event: KeyboardEvent| {
                let mut app = app_for_keyup.borrow_mut();
                if let Some(app) = app.as_mut() {
                    match event.key().as_str() {
                        "ArrowRight" | "d" | "D" => app.input.input_x = 0.0,
                        "ArrowLeft" | "a" | "A" => app.input.input_x = 0.0,
                        "ArrowDown" | "s" | "S" => app.input.input_y = 0.0,
                        "ArrowUp" | "w" | "W" => app.input.input_y = 0.0,
                        _ => {}
                    }
                }
            }) as Box<dyn FnMut(KeyboardEvent)>);
            let _ = window
                .add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref());
            on_keyup.forget();
        }
        // ----------------------------------------------------------------------

        // --- Touch input: update WebInput so gameplay receives touch input ---
        {
            let app_for_touchstart = app_clone.clone();
            let on_touchstart = Closure::wrap(Box::new(move |event: TouchEvent| {
                let mut app = app_for_touchstart.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if let Some(touch) = event.touches().item(0) {
                        app.input.input_x = 1.0;
                        app.input.input_y = 1.0;
                        // Store touch position in input_x/input_y as normalized coords
                        // The actual position handling would need canvas rect
                        let _ = touch.client_x();
                        let _ = touch.client_y();
                    }
                }
            }) as Box<dyn FnMut(TouchEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "touchstart",
                    on_touchstart.as_ref().unchecked_ref(),
                );
            on_touchstart.forget();

            let app_for_touchend = app_clone.clone();
            let on_touchend = Closure::wrap(Box::new(move |event: TouchEvent| {
                let mut app = app_for_touchend.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if event.touches().length() == 0 {
                        app.input.input_x = 0.0;
                        app.input.input_y = 0.0;
                    }
                }
            }) as Box<dyn FnMut(TouchEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "touchend",
                    on_touchend.as_ref().unchecked_ref(),
                );
            on_touchend.forget();

            let app_for_touchmove = app_clone.clone();
            let on_touchmove = Closure::wrap(Box::new(move |event: TouchEvent| {
                let mut app = app_for_touchmove.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if let Some(touch) = event.touches().item(0) {
                        app.input.input_x = 1.0;
                        app.input.input_y = 1.0;
                        let _ = touch.client_x();
                        let _ = touch.client_y();
                    }
                }
            }) as Box<dyn FnMut(TouchEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "touchmove",
                    on_touchmove.as_ref().unchecked_ref(),
                );
            on_touchmove.forget();
        }
        // ----------------------------------------------------------------------

        // --- Mouse input: update WebInput so gameplay receives mouse input ---
        {
            let app_for_mousedown = app_clone.clone();
            let on_mousedown = Closure::wrap(Box::new(move |event: MouseEvent| {
                let mut app = app_for_mousedown.borrow_mut();
                if let Some(app) = app.as_mut() {
                    app.input.mouse_pressed = true;
                    app.input.mouse_just_pressed = true;
                    app.input.mouse_x = event.client_x() as f64;
                    app.input.mouse_y = event.client_y() as f64;
                    // Also update input axes for UI interaction
                    app.input.input_x = 1.0;
                    app.input.input_y = 1.0;
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "mousedown",
                    on_mousedown.as_ref().unchecked_ref(),
                );
            on_mousedown.forget();

            let app_for_mouseup = app_clone.clone();
            let on_mouseup = Closure::wrap(Box::new(move |event: MouseEvent| {
                let mut app = app_for_mouseup.borrow_mut();
                if let Some(app) = app.as_mut() {
                    app.input.mouse_pressed = false;
                    app.input.mouse_just_released = true;
                    app.input.mouse_x = event.client_x() as f64;
                    app.input.mouse_y = event.client_y() as f64;
                    app.input.input_x = 0.0;
                    app.input.input_y = 0.0;
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "mouseup",
                    on_mouseup.as_ref().unchecked_ref(),
                );
            on_mouseup.forget();

            let app_for_mousemove = app_clone.clone();
            let on_mousemove = Closure::wrap(Box::new(move |event: MouseEvent| {
                let mut app = app_for_mousemove.borrow_mut();
                if let Some(app) = app.as_mut() {
                    app.input.mouse_x = event.client_x() as f64;
                    app.input.mouse_y = event.client_y() as f64;
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "mousemove",
                    on_mousemove.as_ref().unchecked_ref(),
                );
            on_mousemove.forget();

            let app_for_wheel = app_clone.clone();
            let on_wheel = Closure::wrap(Box::new(move |event: WheelEvent| {
                let mut app = app_for_wheel.borrow_mut();
                if let Some(app) = app.as_mut() {
                    // Handle zoom with mouse wheel (delta_y negative = zoom in)
                    app.camera.zoom *= (1.0 - event.delta_y() as f32 * 0.001).clamp(0.1, 10.0);
                    event.prevent_default();
                }
            }) as Box<dyn FnMut(WheelEvent)>);
            let _ = window
                .add_event_listener_with_callback(
                    "wheel",
                    on_wheel.as_ref().unchecked_ref(),
                );
            on_wheel.forget();
        }
        // ----------------------------------------------------------------------

        // --- Window resize handler ---
        {
            let app_for_resize = app_clone.clone();
            let on_resize = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let mut app = app_for_resize.borrow_mut();
                if let Some(app) = app.as_mut() {
                    let window = web_sys::window().expect("no window");
                    let document = window.document().expect("no document");
                    if let Some(canvas) = document.get_element_by_id("game-canvas") {
                        if let Ok(canvas) = canvas.dyn_into::<HtmlCanvasElement>() {
                            let width = canvas.client_width().max(1) as u32;
                            let height = canvas.client_height().max(1) as u32;
                            if width != app.canvas_width || height != app.canvas_height {
                                app.canvas_width = width;
                                app.canvas_height = height;
                                app.surface.configure(
                                    &app.renderer.device(),
                                    &wgpu::SurfaceConfiguration {
                                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                        format: app.renderer.surface_config().format,
                                        width,
                                        height,
                                        present_mode: app.renderer.surface_config().present_mode,
                                        alpha_mode: app.renderer.surface_config().alpha_mode,
                                        view_formats: vec![],
                                        desired_maximum_frame_latency: 2,
                                    },
                                );
                                app.camera = Camera::new(width as f32, height as f32);
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            let _ = window
                .add_event_listener_with_callback(
                    "resize",
                    on_resize.as_ref().unchecked_ref(),
                );
            on_resize.forget();
        }
        // ----------------------------------------------------------------------

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
                // Update mouse state on the slice host for UI click detection
                let host = slice.host_mut();
                host.mouse_x = input.mouse_x;
                host.mouse_y = input.mouse_y;
                host.mouse_pressed = input.mouse_just_pressed; // Only trigger on just pressed
                
                let _ = slice.step(&mut self.world, input_axis, dt.as_secs_f64());
            }
            // Update particle systems
            crate::sprite::update_particle_systems(&mut self.world, dt.as_secs_f64());
        });
        
        // Reset just_pressed/just_released flags
        self.input.mouse_just_pressed = false;
        self.input.mouse_just_released = false;
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

        // Collect sprite instances from the ECS world (CPU gather pass).
        self.sprite_renderer.begin_frame();

        // Tilemaps are drawn first so entities appear on top of them.
        // Only the chunks inside the camera view are submitted.
        let view_bounds = self.camera.view_bounds();
        let world = self.world.world_ref();
        for (_, tilemap) in world.query::<(&Transform, &kiln_runtime::Tilemap)>().iter() {
            for (texture, instance) in tilemap.collect_visible(view_bounds) {
                self.sprite_renderer.push_textured(texture, instance);
            }
        }

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

        {
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

            self.sprite_renderer
                .flush(&mut pass, &self.camera, self.renderer.queue());
        }

        if self.renderer.has_post() {
            self.renderer.apply_post(&mut encoder, &view);
        }

        // Render HUD if slice has HUD lines
        if let Some(slice) = &mut self.slice {
            // Render UI commands from script
            let ui_commands = slice.take_ui_commands();
            if !ui_commands.is_empty() {
                let mut ui_renderer = UiRenderer::new();
                let size = self.renderer.surface_config();
                let hud_camera = Camera::new_top_left(size.width as f32, size.height as f32);
                
                for command in ui_commands {
                    match command {
                        SliceCommand::UiButton { id, x, y, w, h } => {
                            self.render_ui_button(&mut ui_renderer, &hud_camera, &view, id, x, y, w, h, &mut encoder);
                        }
                        SliceCommand::UiLabel { id, x, y, text } => {
                            self.render_ui_label(&mut ui_renderer, &hud_camera, &view, id, x, y, text, &mut encoder);
                        }
                        SliceCommand::UiPanel { id, x, y, w, h } => {
                            self.render_ui_panel(&mut ui_renderer, &hud_camera, &view, id, x, y, w, h, &mut encoder);
                        }
                        _ => {}
                    }
                }
            }
            
            // Render HUD text
            let lines = slice.hud_lines();
            if !lines.is_empty() {
                let text = TextRenderer::new();
                let mut hud_instances = Vec::new();
                for (index, line) in lines.iter().enumerate() {
                    let y = 16.0 + index as f32 * 20.0;
                    let quads = text.render(line, 16.0, y, 16.0, [1.0, 1.0, 1.0, 1.0]);
                    hud_instances.extend(quads);
                }
                if !hud_instances.is_empty() {
                    self.sprite_renderer.begin_frame();
                    for instance in hud_instances {
                        self.sprite_renderer.push(instance);
                    }
                    let size = self.renderer.surface_config();
                    let hud_camera = Camera::new_top_left(size.width as f32, size.height as f32);
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("web_hud_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    self.sprite_renderer
                        .flush(&mut pass, &hud_camera, self.renderer.queue());
                }
            }
        }

        self.renderer.finish_profiler_frame(&mut encoder);
        self.renderer.queue().submit([encoder.finish()]);
        frame.present();

        self.renderer.poll_profiler();
    }
}



async fn load_script(path: &str, project_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let full_path = if path.starts_with("http") {
        path.to_string()
    } else {
        format!("{}/{}", project_path_dir(project_path), path)
    };
    let request = web_sys::Request::new_with_str_and_init(&full_path, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
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

/// Load all PNG textures referenced by the project and upload them to the GPU.
async fn load_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sprite_renderer: &mut SpriteRenderer,
    project: &kiln_core::Project,
    project_path: &str,
) -> Result<HashMap<String, TextureId>, Box<dyn std::error::Error>> {
    let mut texture_paths = Vec::new();
    
    // Collect all unique texture paths from the project
    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let kiln_core::project::Component::Sprite { texture, .. } = component {
                    if let Some(path) = texture {
                        if !texture_paths.contains(path) {
                            texture_paths.push(path.clone());
                        }
                    }
                }
            }
        }
    }
    
    let mut textures = HashMap::new();
    
    // Load each texture
    for path in &texture_paths {
        // Assume assets are served from the same base path as the project
        // The path in the project is like "art/hero.png", so we need to fetch it
        let full_path = if path.starts_with("http") {
            path.clone()
        } else {
            format!("{}/{}", project_path_dir(project_path), path)
        };
        
        match load_png_texture(device, queue, sprite_renderer, &full_path).await {
            Ok(tex_id) => {
                info!("Loaded texture: {} -> {:?}", path, tex_id);
                textures.insert(path.clone(), tex_id);
            }
            Err(e) => {
                warn!("Failed to load texture {}: {}", path, e);
                // Fall back to white texture
                textures.insert(path.clone(), TextureId::WHITE);
            }
        }
    }
    
    Ok(textures)
}

/// Extract the directory portion of the project path for loading assets.
fn project_path_dir(project_path: &str) -> String {
    if let Some(idx) = project_path.rfind('/') {
        project_path[..idx].to_string()
    } else {
        ".".to_string()
    }
}

/// Load a project file from the given path.
async fn load_project(project_path: &str) -> Result<kiln_core::Project, Box<dyn std::error::Error>> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(project_path, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
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
    let text = text.as_string().ok_or("Project text is not valid UTF-8")?;
    let project = kiln_core::Project::from_toml(&text)
        .map_err(|e| format!("TOML parse failed: {}", e))?;
    Ok(project)
}

/// Fetch an audio file and return the raw bytes.
async fn fetch_audio_buffer(path: &str) -> Result<js_sys::Uint8Array, Box<dyn std::error::Error>> {
    let window = web_sys::window().ok_or("No window available")?;
    let resp = window.fetch_with_str(path).await.map_err(|e| format!("{:?}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()).into());
    }
    let buffer = resp.array_buffer().map_err(|e| format!("{:?}", e))?;
    let buffer = wasm_bindgen_futures::JsFuture::from(buffer)
        .await
        .map_err(|e| format!("{:?}", e))?;
    let bytes = js_sys::Uint8Array::new(&buffer);
    Ok(bytes)
}

/// Load a single PNG texture from a URL and upload it to the GPU.
async fn load_png_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sprite_renderer: &mut SpriteRenderer,
    url: &str,
) -> Result<TextureId, Box<dyn std::error::Error>> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::Cors);
    
    let request = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
    let promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Fetch await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;
    
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()).into());
    }
    
    let array_buffer_promise = resp.array_buffer()
        .map_err(|e| format!("Array buffer failed: {:?}", e))?;
    let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise)
        .await
        .map_err(|e| format!("Array buffer await failed: {}", e.as_string().unwrap_or_else(|| "unknown".to_string())))?;
    let array_buffer: js_sys::ArrayBuffer = array_buffer
        .dyn_into()
        .map_err(|e| format!("ArrayBuffer cast failed: {:?}", e))?;
    let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
    
    // Decode PNG
    let image = image::load_from_memory(&bytes)
        .map_err(|e| format!("PNG decode failed: {}", e))?;
    let rgba = image.to_rgba8();
    let (width, height) = image.dimensions();
    
    // Upload to sprite renderer
    let tex_id = sprite_renderer.create_texture_rgba(device, queue, &url, width, height, &rgba);
    
    Ok(tex_id)
}

/// Spawn all entities from a scene, including tilemaps.
fn spawn_scene_entities(
    world: &mut GameWorld,
    project: &kiln_core::Project,
    scene: &kiln_core::project::Scene,
    textures: &HashMap<String, TextureId>,
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

    // Spawn tilemaps using the runtime's function with a texture resolver.
    // Create a resolver that maps tileset IDs to TextureIds.
    let mut resolve = |tileset_id: &str| {
        // Look up the tileset in the project and get its texture path
        project.tilesets.get(tileset_id).and_then(|ts| {
            textures.get(&ts.texture).copied()
        })
    };
    
    if let Err(e) = kiln_runtime::spawn_tilemaps(world, project, scene, kiln_core::SceneOffset::default(), &mut resolve) {
        warn!("Failed to spawn tilemaps: {}", e);
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

impl App {
    fn render_ui_button(
        &mut self,
        ui_renderer: &mut kiln_ui::UiRenderer,
        hud_camera: &Camera,
        view: &wgpu::TextureView,
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let button = kiln_ui::widget::Button::new(
            kiln_ui::widget::WidgetId::from_u64(id.hash::<std::collections::hash_map::DefaultHasher>() as u64),
            id.clone(),
        ).with_layout(kiln_ui::layout::Layout {
            anchor: kiln_ui::layout::Anchor::TopLeft,
            constraints: kiln_ui::layout::LayoutConstraints::tight(w as f32, h as f32),
            ..Default::default()
        });
        let button = button.with_bounds(kiln_ui::layout::Rect::new(x as f32, y as f32, w as f32, h as f32));
        let quads = ui_renderer.render_button(&button, false, false);
        self.sprite_renderer.begin_frame();
        for quad in quads {
            self.sprite_renderer.push_textured(quad.texture, quad.instance);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("web_ui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.sprite_renderer.flush(&mut pass, hud_camera, self.renderer.queue());
    }

    fn render_ui_label(
        &mut self,
        ui_renderer: &mut kiln_ui::UiRenderer,
        hud_camera: &Camera,
        view: &wgpu::TextureView,
        id: String,
        x: f64,
        y: f64,
        text: String,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let label = kiln_ui::widget::Label::new(
            kiln_ui::widget::WidgetId::from_u64(id.hash::<std::collections::hash_map::DefaultHasher>() as u64),
            text,
        ).with_layout(kiln_ui::layout::Layout {
            anchor: kiln_ui::layout::Anchor::TopLeft,
            ..Default::default()
        });
        let label = label.with_bounds(kiln_ui::layout::Rect::new(x as f32, y as f32, 200.0, 24.0));
        let quads = ui_renderer.render_label(&label);
        self.sprite_renderer.begin_frame();
        for quad in quads {
            self.sprite_renderer.push_textured(quad.texture, quad.instance);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("web_ui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.sprite_renderer.flush(&mut pass, hud_camera, self.renderer.queue());
    }

    fn render_ui_panel(
        &mut self,
        ui_renderer: &mut kiln_ui::UiRenderer,
        hud_camera: &Camera,
        view: &wgpu::TextureView,
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let panel = kiln_ui::widget::Panel::new(
            kiln_ui::widget::WidgetId::from_u64(id.hash::<std::collections::hash_map::DefaultHasher>() as u64),
        ).with_layout(kiln_ui::layout::Layout {
            anchor: kiln_ui::layout::Anchor::TopLeft,
            constraints: kiln_ui::layout::LayoutConstraints::tight(w as f32, h as f32),
            ..Default::default()
        });
        let panel = panel.with_bounds(kiln_ui::layout::Rect::new(x as f32, y as f32, w as f32, h as f32));
        let quads = ui_renderer.render_panel(&panel);
        self.sprite_renderer.begin_frame();
        for quad in quads {
            self.sprite_renderer.push_textured(quad.texture, quad.instance);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("web_ui_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.sprite_renderer.flush(&mut pass, hud_camera, self.renderer.queue());
    }
}
