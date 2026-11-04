//! Kiln web runtime — WASM export for browser.
//!
//! Creates a `wgpu` surface on an HTML `<canvas>`, loads a Kiln project
//! (`kiln.toml` + PNG assets), runs the fixed-timestep game loop with the
//! KilnScript slice runtime, and renders 2D sprites through the engine's
//! sprite batcher. Called from JavaScript via `KilnWeb.run(canvasId, projectPath)`.

// This crate hosts browser APIs and is only executable on wasm32.
#![cfg(target_arch = "wasm32")]

mod threading;

use kiln_core::Transform;
use kiln_render::{
    camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer, TextureId,
};
use kiln_runtime::{GameLoop, GameWorld, SliceCommand, SliceSession};
use kiln_ui::{TextRenderer, UiRenderer, Widget};
use log::{info, warn};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, HtmlCanvasElement, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};

use image::GenericImageView;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn run(canvas_id: &str, project_path: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .and_then(|element| element.dyn_into().ok())
        .ok_or_else(|| JsValue::from_str("Canvas element not found"))?;
    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;
    canvas.set_width(width);
    canvas.set_height(height);
    // wgpu selects one browser backend per instance; a present navigator.gpu
    // does not guarantee that WebGPU can supply an adapter. Probe before
    // claiming the canvas context, then fall back to a separate WebGL instance.
    let gpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });
    let instance = if gpu_instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .is_some()
    {
        gpu_instance
    } else {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        })
    };
    let surface = create_wgpu_surface(&instance, canvas.clone())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let app = App::new(&instance, surface, canvas, width, height, project_path)
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    app.run(&window);
    Ok(())
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
    /// Track which movement keys are currently pressed
    key_right: bool,
    key_left: bool,
    key_down: bool,
    key_up: bool,
}

struct App {
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    overlay_renderer: SpriteRenderer,
    camera: Camera,
    world: GameWorld,
    slice: Option<SliceSession>,
    game_loop: GameLoop,
    input: WebInput,
    /// Web Audio API context for audio playback
    audio_context: Option<web_sys::AudioContext>,
    /// Canvas element for touch position calculations
    canvas_element: Option<web_sys::HtmlCanvasElement>,
    /// Canvas width
    canvas_width: u32,
    /// Canvas height
    canvas_height: u32,
}

impl App {
    /// Get the canvas element for touch position calculations
    fn get_canvas_element(&self) -> Option<web_sys::HtmlCanvasElement> {
        self.canvas_element.clone()
    }
}

impl App {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        canvas: HtmlCanvasElement,
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
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
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

        let renderer =
            Renderer::new_from_parts(device, queue, surface_format, tier, width, height).await?;

        surface.configure(renderer.device(), &renderer.surface_config());

        let mut sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.scene_format());
        sprite_renderer.upload_quad(renderer.queue());
        let overlay_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), surface_format);
        overlay_renderer.upload_quad(renderer.queue());

        // Load project first to get audio assets and scene script
        let project = load_project(project_path).await?;

        let textures = load_textures(
            renderer.device(),
            renderer.queue(),
            &mut sprite_renderer,
            &project,
            project_path,
        )
        .await?;

        let mut world = GameWorld::new();
        let scene_id = project.default_scene_id().ok_or("Project has no scenes")?;
        let scene = project.scenes.get(&scene_id).ok_or("Scene not found")?;

        // Initialize Web Audio API
        let audio_context = web_sys::AudioContext::new().ok();
        let master_gain = audio_context
            .as_ref()
            .and_then(|ctx| ctx.create_gain().ok());
        if let Some(gain) = &master_gain {
            gain.connect_with_audio_node(&audio_context.as_ref().unwrap().destination())
                .ok();
            gain.gain().set_value(1.0);
        }

        // Register all project audio assets and decode them
        let mut audio_buffers = std::collections::HashMap::new();
        if let Some(ctx) = &audio_context {
            for asset in &project.audio_assets {
                if let Ok(array_buffer) = fetch_audio_buffer(&format!(
                    "{}/{}",
                    project_path_dir(project_path),
                    asset.path
                ))
                .await
                {
                    if let Ok(buffer) = decode_audio(ctx, &array_buffer).await {
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
                SliceSession::with_action_map(&script_source, project.action_map.clone())
                    .map_err(|e| format!("Script compile error: {:?}", e))?;

            // JS audio handles stay on the browser thread; access the public host.
            if let Some(audio_ctx) = audio_context.clone() {
                let buffers = audio_buffers.clone();
                let gain = master_gain.clone();
                session.host_mut().set_audio_player(Box::new(move |name| {
                    let Some(buffer) = buffers.get(name) else {
                        return;
                    };
                    match audio_ctx.create_buffer_source() {
                        Ok(source) => {
                            source.set_buffer(Some(buffer));
                            let connected = match &gain {
                                Some(gain) => source.connect_with_audio_node(gain),
                                None => source.connect_with_audio_node(&audio_ctx.destination()),
                            };
                            if connected.is_ok() {
                                let _ = source.start();
                            }
                        }
                        Err(error) => web_sys::console::error_1(&error),
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
            overlay_renderer,
            camera,
            world,
            slice,
            game_loop: GameLoop::new(),
            input: WebInput::default(),
            audio_context,
            canvas_element: Some(canvas.clone()),
            canvas_width: width,
            canvas_height: height,
        })
    }

    fn run(self, window: &web_sys::Window) {
        let app = std::rc::Rc::new(std::cell::RefCell::new(Some(self)));
        let app_clone = std::rc::Rc::clone(&app);

        // --- Keyboard input: update WebInput so gameplay receives real input ---
        {
            let app_for_keydown = app_clone.clone();
            let on_keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
                let mut app = app_for_keydown.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if let Some(context) = &app.audio_context {
                        let _ = context.resume();
                    }
                    let mut changed = false;
                    match event.key().as_str() {
                        "ArrowRight" | "d" | "D" => {
                            if !app.input.key_right {
                                app.input.key_right = true;
                                changed = true;
                            }
                        }
                        "ArrowLeft" | "a" | "A" => {
                            if !app.input.key_left {
                                app.input.key_left = true;
                                changed = true;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if !app.input.key_down {
                                app.input.key_down = true;
                                changed = true;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if !app.input.key_up {
                                app.input.key_up = true;
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                    // Recompute input_x and input_y based on all pressed keys
                    if changed {
                        app.input.input_x =
                            (app.input.key_right as i8 - app.input.key_left as i8) as f64;
                        app.input.input_y =
                            (app.input.key_down as i8 - app.input.key_up as i8) as f64;
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
                    let mut changed = false;
                    match event.key().as_str() {
                        "ArrowRight" | "d" | "D" => {
                            if app.input.key_right {
                                app.input.key_right = false;
                                changed = true;
                            }
                        }
                        "ArrowLeft" | "a" | "A" => {
                            if app.input.key_left {
                                app.input.key_left = false;
                                changed = true;
                            }
                        }
                        "ArrowDown" | "s" | "S" => {
                            if app.input.key_down {
                                app.input.key_down = false;
                                changed = true;
                            }
                        }
                        "ArrowUp" | "w" | "W" => {
                            if app.input.key_up {
                                app.input.key_up = false;
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                    // Recompute input_x and input_y based on all pressed keys
                    if changed {
                        app.input.input_x =
                            (app.input.key_right as i8 - app.input.key_left as i8) as f64;
                        app.input.input_y =
                            (app.input.key_down as i8 - app.input.key_up as i8) as f64;
                    }
                }
            }) as Box<dyn FnMut(KeyboardEvent)>);
            let _ =
                window.add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref());
            on_keyup.forget();
        }
        // ----------------------------------------------------------------------

        // --- Touch input: update WebInput so gameplay receives touch input ---
        {
            let app_for_touchstart = app_clone.clone();
            let on_touchstart = Closure::wrap(Box::new(move |event: TouchEvent| {
                let mut app = app_for_touchstart.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if let Some(context) = &app.audio_context {
                        let _ = context.resume();
                    }
                    if let Some(touch) = event.touches().item(0) {
                        // Calculate input axes from touch position relative to canvas center
                        if let Some(canvas) = app.get_canvas_element() {
                            let rect = canvas.get_bounding_client_rect();
                            let canvas_width = rect.width() as f64;
                            let canvas_height = rect.height() as f64;
                            let touch_x = touch.client_x() as f64 - rect.left();
                            let touch_y = touch.client_y() as f64 - rect.top();

                            // Normalize to -1 to 1 range from canvas center
                            app.input.input_x =
                                (touch_x - canvas_width / 2.0) / (canvas_width / 2.0);
                            app.input.input_y =
                                (touch_y - canvas_height / 2.0) / (canvas_height / 2.0);

                            // Clamp to [-1, 1]
                            app.input.input_x = app.input.input_x.clamp(-1.0, 1.0);
                            app.input.input_y = app.input.input_y.clamp(-1.0, 1.0);
                        } else {
                            // Fallback if canvas not available
                            app.input.input_x = 1.0;
                            app.input.input_y = 1.0;
                        }
                    }
                }
            }) as Box<dyn FnMut(TouchEvent)>);
            let _ = window.add_event_listener_with_callback(
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
                .add_event_listener_with_callback("touchend", on_touchend.as_ref().unchecked_ref());
            on_touchend.forget();

            let app_for_touchmove = app_clone.clone();
            let on_touchmove = Closure::wrap(Box::new(move |event: TouchEvent| {
                let mut app = app_for_touchmove.borrow_mut();
                if let Some(app) = app.as_mut() {
                    if let Some(touch) = event.touches().item(0) {
                        // Calculate input axes from touch position relative to canvas center
                        if let Some(canvas) = app.get_canvas_element() {
                            let rect = canvas.get_bounding_client_rect();
                            let canvas_width = rect.width() as f64;
                            let canvas_height = rect.height() as f64;
                            let touch_x = touch.client_x() as f64 - rect.left();
                            let touch_y = touch.client_y() as f64 - rect.top();

                            // Normalize to -1 to 1 range from canvas center
                            app.input.input_x =
                                (touch_x - canvas_width / 2.0) / (canvas_width / 2.0);
                            app.input.input_y =
                                (touch_y - canvas_height / 2.0) / (canvas_height / 2.0);

                            // Clamp to [-1, 1]
                            app.input.input_x = app.input.input_x.clamp(-1.0, 1.0);
                            app.input.input_y = app.input.input_y.clamp(-1.0, 1.0);
                        } else {
                            app.input.input_x = 1.0;
                            app.input.input_y = 1.0;
                        }
                    }
                }
            }) as Box<dyn FnMut(TouchEvent)>);
            let _ = window.add_event_listener_with_callback(
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
                    if let Some(context) = &app.audio_context {
                        let _ = context.resume();
                    }
                    app.input.mouse_pressed = true;
                    app.input.mouse_just_pressed = true;
                    app.input.mouse_x = event.client_x() as f64;
                    app.input.mouse_y = event.client_y() as f64;
                    // Also update input axes for UI interaction
                    app.input.input_x = 1.0;
                    app.input.input_y = 1.0;
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = window.add_event_listener_with_callback(
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
                .add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref());
            on_mouseup.forget();

            let app_for_mousemove = app_clone.clone();
            let on_mousemove = Closure::wrap(Box::new(move |event: MouseEvent| {
                let mut app = app_for_mousemove.borrow_mut();
                if let Some(app) = app.as_mut() {
                    app.input.mouse_x = event.client_x() as f64;
                    app.input.mouse_y = event.client_y() as f64;
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            let _ = window.add_event_listener_with_callback(
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
            let _ =
                window.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
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
                .add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
            on_resize.forget();
        }
        // ----------------------------------------------------------------------

        let f = std::rc::Rc::new(std::cell::RefCell::new(None::<Closure<dyn FnMut()>>));
        let f_clone = std::rc::Rc::clone(&f);

        let animation_window = window.clone();
        *f.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            let mut app = app_clone.borrow_mut().take().unwrap();

            app.update();
            app.render(&animation_window);

            app_clone.borrow_mut().replace(app);

            let f_ref = f_clone.clone();
            if let Err(error) = animation_window
                .request_animation_frame(f_ref.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            {
                web_sys::console::error_1(&error);
            };
        }) as Box<dyn FnMut()>));

        let _ =
            window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    fn update(&mut self) {
        let input = self.input.clone();
        let input_axis = (input.input_x, input.input_y);

        self.game_loop.tick(|dt| {
            if let Some(slice) = &mut self.slice {
                // Update mouse state on the slice host for UI click detection
                let host = slice.host_mut();
                host.set_mouse_state(input.mouse_x, input.mouse_y, input.mouse_just_pressed);

                if let Err(error) = slice.step(&mut self.world, input_axis, dt.as_secs_f64()) {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Script execution failed: {error}"
                    )));
                }
            }
            // SliceSession updates particles itself; avoid advancing them twice.
            if self.slice.is_none() {
                kiln_runtime::sprite::update_particle_systems(
                    self.world.world_mut(),
                    dt.as_secs_f32(),
                );
            }
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

        let mut encoder =
            self.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("web_frame_encoder"),
                });

        let target_view = self.renderer.scene_target().unwrap_or(&view);

        // Collect sprite instances from the ECS world (CPU gather pass).
        let _ = kiln_runtime::update_global_transforms(&mut self.world);
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

        for (transform, sprite) in world
            .query::<(&kiln_runtime::GlobalTransform, &kiln_runtime::Sprite)>()
            .iter()
        {
            let instance = SpriteInstance {
                position: [transform.position.x, transform.position.y],
                size: sprite.size,
                color: sprite.color,
                uv_rect: sprite.uv_rect,
                material_id: 0,
                rotation: 0.0,
            }
            .with_transform(transform.rotation, [transform.scale.x, transform.scale.y]);
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

        // World and overlay need separate GPU buffers. Reusing the world
        // buffer here would overwrite its queued draw before submission.
        if let Some((commands, lines)) = self
            .slice
            .as_mut()
            .map(|slice| (slice.take_ui_commands(), slice.hud_lines()))
        {
            self.overlay_renderer.begin_frame();
            let ui = UiRenderer::new();
            for command in commands {
                for quad in command_quads(command, &ui) {
                    self.overlay_renderer
                        .push_textured(quad.texture, quad.instance);
                }
            }
            let text = TextRenderer::new();
            for (index, line) in lines.iter().enumerate() {
                for instance in text.render(line, 16.0, 16.0 + index as f32 * 20.0, 16.0, [1.0; 4])
                {
                    self.overlay_renderer.push(instance);
                }
            }
            let size = self.renderer.surface_config();
            let camera = Camera::new_top_left(size.width as f32, size.height as f32);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("web_overlay_pass"),
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
            self.overlay_renderer
                .flush(&mut pass, &camera, self.renderer.queue());
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
        .map_err(|e| {
            format!(
                "Fetch await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;
    let text_promise = resp.text().map_err(|e| format!("{:?}", e))?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| {
            format!(
                "Text await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
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
async fn load_project(
    project_path: &str,
) -> Result<kiln_core::Project, Box<dyn std::error::Error>> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(project_path, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
    let promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| {
            format!(
                "Fetch await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;
    let text_promise = resp.text().map_err(|e| format!("{:?}", e))?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| {
            format!(
                "Text await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
    let text = text.as_string().ok_or("Project text is not valid UTF-8")?;
    let project =
        kiln_core::Project::from_toml(&text).map_err(|e| format!("TOML parse failed: {}", e))?;
    Ok(project)
}

/// Fetch an audio file and return the raw bytes.
async fn fetch_audio_buffer(path: &str) -> Result<js_sys::ArrayBuffer, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(path))
        .await?
        .dyn_into::<web_sys::Response>()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "Audio HTTP error: {}",
            response.status()
        )));
    }
    wasm_bindgen_futures::JsFuture::from(response.array_buffer()?)
        .await?
        .dyn_into::<js_sys::ArrayBuffer>()
}

async fn decode_audio(
    context: &AudioContext,
    bytes: &js_sys::ArrayBuffer,
) -> Result<web_sys::AudioBuffer, JsValue> {
    wasm_bindgen_futures::JsFuture::from(context.decode_audio_data(bytes)?)
        .await?
        .dyn_into::<web_sys::AudioBuffer>()
}

fn widget_id(name: &str) -> kiln_ui::WidgetId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    kiln_ui::WidgetId::from_u64(hasher.finish())
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
        .map_err(|e| {
            format!(
                "Fetch await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|e| format!("Response cast failed: {:?}", e))?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()).into());
    }

    let array_buffer_promise = resp
        .array_buffer()
        .map_err(|e| format!("Array buffer failed: {:?}", e))?;
    let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise)
        .await
        .map_err(|e| {
            format!(
                "Array buffer await failed: {}",
                e.as_string().unwrap_or_else(|| "unknown".to_string())
            )
        })?;
    let array_buffer: js_sys::ArrayBuffer = array_buffer
        .dyn_into()
        .map_err(|e| format!("ArrayBuffer cast failed: {:?}", e))?;
    let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();

    // Decode PNG
    let image = image::load_from_memory(&bytes).map_err(|e| format!("PNG decode failed: {}", e))?;
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
    // Share entity identity, roles and animation setup with the native player.
    let mut make_sprite = |entity: &kiln_core::Entity, color: [f32; 4], texture: Option<&str>| {
        let mut sprite =
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]).with_size(32.0, 32.0);
        if let Some(id) = texture.and_then(|path| textures.get(path)) {
            sprite = sprite.with_texture(*id);
        }
        for component in &entity.components {
            if let kiln_core::Component::Sprite {
                layer, material, ..
            } = component
            {
                sprite.layer = *layer;
                sprite.material = material.clone();
            }
        }
        sprite
    };
    kiln_runtime::spawn_scene(world, project, &scene.id, &mut make_sprite);

    // Spawn tilemaps using the runtime's function with a texture resolver.
    // Create a resolver that maps tileset IDs to TextureIds.
    let mut resolve = |tileset_id: &str| {
        // Look up the tileset in the project and get its texture path
        project
            .tilesets
            .get(tileset_id)
            .and_then(|ts| textures.get(&ts.texture).copied())
    };

    kiln_runtime::spawn_tilemaps(
        world,
        project,
        scene,
        kiln_core::SceneOffset::default(),
        &mut resolve,
    );

    Ok(())
}

fn create_wgpu_surface(
    instance: &wgpu::Instance,
    canvas: HtmlCanvasElement,
) -> Result<wgpu::Surface<'static>, Box<dyn std::error::Error>> {
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?;
    Ok(surface)
}

/// Convert script UI commands into one overlay batch rather than overwriting
/// the instance buffer between several draws in the same command encoder.
fn command_quads(command: SliceCommand, renderer: &UiRenderer) -> Vec<kiln_ui::UiQuad> {
    use kiln_ui::{Button, Label, Panel, Rect};
    match command {
        SliceCommand::UiButton { id, x, y } => {
            let mut button = Button::new(widget_id(&id), id);
            button.set_bounds(Rect::new(x as f32, y as f32, 64.0, 32.0));
            renderer.render_button(&button, false, false)
        }
        SliceCommand::UiLabel { id, x, y, text } => {
            let mut label = Label::new(widget_id(&id), text);
            label.set_bounds(Rect::new(x as f32, y as f32, 200.0, 24.0));
            renderer.render_label(&label)
        }
        SliceCommand::UiPanel { id, x, y, w, h } => {
            let mut panel = Panel::new(widget_id(&id));
            panel.set_bounds(Rect::new(x as f32, y as f32, w as f32, h as f32));
            renderer.render_panel(&panel)
        }
        _ => Vec::new(),
    }
}
