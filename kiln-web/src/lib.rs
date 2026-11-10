//! Kiln web runtime — WASM export for browser.
//!
//! Creates a `wgpu` surface on an HTML `<canvas>`, loads a Kiln project
//! (`kiln.toml` + PNG assets), runs the fixed-timestep game loop with the
//! KilnScript slice runtime, and renders 2D sprites through the engine's
//! sprite batcher. Called from JavaScript via `KilnWeb.run(canvasId, projectPath)`.

// This crate hosts browser APIs and is only executable on wasm32.
#![cfg(target_arch = "wasm32")]

mod audio;
mod export_info;
mod input;
mod resources;
mod storage;
mod threading;

use kiln_render::{camera::Camera, Renderer, SpriteRenderer, TextureId};
use kiln_runtime::{GameLoop, GameWorld, SliceCommand, SliceSession};
use kiln_ui::{TextRenderer, UiRenderer, Widget};
use log::{info, warn};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, HtmlCanvasElement};

use image::GenericImageView;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

thread_local! {
    static PREPARED_GAME: std::cell::RefCell<Option<App>> = const { std::cell::RefCell::new(None) };
}

#[wasm_bindgen]
pub async fn run(canvas_id: &str, project_path: &str) -> Result<(), JsValue> {
    let (app, window) = load_app(canvas_id, project_path).await?;
    app.run(&window);
    Ok(())
}

/// Load all runtime resources without starting gameplay or installing input handlers.
#[wasm_bindgen]
pub async fn prepare_game(canvas_id: &str, project_path: &str) -> Result<(), JsValue> {
    if PREPARED_GAME.with(|app| app.borrow().is_some()) {
        return Err(JsValue::from_str("A game is already prepared"));
    }
    let (app, _) = load_app(canvas_id, project_path).await?;
    PREPARED_GAME.with(|pending| *pending.borrow_mut() = Some(app));
    Ok(())
}

/// Call from a trusted start gesture; resume audio before advancing the first tick.
#[wasm_bindgen]
pub async fn start_game() -> Result<(), JsValue> {
    let audio = PREPARED_GAME
        .with(|pending| {
            pending
                .borrow()
                .as_ref()
                .map(|app| app.audio_context.clone())
        })
        .ok_or_else(|| JsValue::from_str("Prepare the game before starting"))?;
    if let Some(context) = audio {
        wasm_bindgen_futures::JsFuture::from(context.resume()?).await?;
    }
    let mut app = PREPARED_GAME
        .with(|pending| pending.borrow_mut().take())
        .ok_or_else(|| JsValue::from_str("Game was already started"))?;
    app.game_loop = GameLoop::new();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    app.run(&window);
    Ok(())
}

async fn load_app(canvas_id: &str, project_path: &str) -> Result<(App, web_sys::Window), JsValue> {
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
    Ok((app, window))
}

#[derive(Default, Clone)]
struct WebInput {
    keys: std::collections::HashSet<String>,
    key_edges: Vec<String>,
    pointer_edges: Vec<([f32; 2], bool)>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_pressed: bool,
    mouse_just_pressed: bool,
    touch_origin: Option<[f64; 2]>,
    touch_axis: [f64; 2],
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
    project: kiln_core::Project,
    scene_id: String,
    scripts: HashMap<String, String>,
    textures: resources::Textures,
    ui_fonts: kiln_ui::authored::UiFonts,
    camera_controller: kiln_runtime::camera::CameraController,
    audio: Option<std::rc::Rc<std::cell::RefCell<audio::WebAudio>>>,
    focused: bool,
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
        let mut overlay_renderer =
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

        let scene_id = project.default_scene_id().ok_or("Project has no scenes")?;
        let scripts = resources::scripts(&project, project_path).await?;
        let (mut session, world) = resources::session(&project, &scene_id, &scripts, &textures)?;
        session
            .host_mut()
            .set_storage(std::rc::Rc::new(storage::BrowserStorage {
                project: project.id.clone(),
            }));
        let audio_context = web_sys::AudioContext::new().ok();
        let mut buffers = HashMap::new();
        if let Some(context) = &audio_context {
            for asset in &project.audio_assets {
                let bytes = fetch_audio_buffer(&format!(
                    "{}/{}",
                    project_path_dir(project_path),
                    asset.path
                ))
                .await
                .map_err(|e| format!("Audio fetch: {e:?}"))?;
                let buffer = decode_audio(context, &bytes)
                    .await
                    .map_err(|e| format!("Audio decode: {e:?}"))?;
                if buffers
                    .insert(asset.name.clone(), (asset.clone(), buffer))
                    .is_some()
                {
                    return Err("Duplicate audio asset name".into());
                }
            }
        }
        let audio = audio_context.as_ref().map(|context| {
            std::rc::Rc::new(std::cell::RefCell::new(audio::WebAudio::new(
                context.clone(),
                buffers,
                project.audio_mixer.clone(),
            )))
        });
        if let Some(audio) = &audio {
            session
                .host_mut()
                .set_platform_audio(Box::new(audio::AudioAdapter(audio.clone())));
        }
        let mut font_resources = HashMap::new();
        for font in project.ui.fonts.values() {
            let bytes =
                fetch_audio_buffer(&format!("{}/{}", project_path_dir(project_path), font.path))
                    .await
                    .map_err(|e| format!("Font fetch: {e:?}"))?;
            font_resources.insert(font.path.clone(), js_sys::Uint8Array::new(&bytes).to_vec());
        }
        let (ui_fonts, font_errors) = kiln_ui::authored::UiFonts::load(
            &project.ui,
            |path| {
                font_resources
                    .get(path)
                    .cloned()
                    .ok_or_else(|| format!("Missing font {path}"))
            },
            |id, size, rgba| {
                overlay_renderer.create_texture_rgba_with_options(
                    renderer.device(),
                    renderer.queue(),
                    id,
                    size,
                    rgba,
                    [false, false],
                )
            },
        );
        for error in font_errors {
            warn!("{error}");
        }
        let mut camera_controller = kiln_runtime::camera::CameraController::default();
        let settings = project.cameras.get(&scene_id).cloned().unwrap_or_default();
        let camera = camera_controller.update(
            &settings,
            &world,
            [project.window.width as f32, project.window.height as f32],
            0.0,
        )?;
        let slice = Some(session);

        Ok(Self {
            surface,
            renderer,
            sprite_renderer,
            overlay_renderer,
            camera,
            world,
            slice,
            game_loop: GameLoop::new(),
            project,
            scene_id,
            scripts,
            textures,
            ui_fonts,
            camera_controller,
            audio,
            focused: true,
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

        input::install(&app_clone, window);

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

    fn viewport(&self) -> [f32; 4] {
        self.camera
            .fit_viewport([0, 0, self.canvas_width, self.canvas_height])
            .map(|v| v as f32)
    }

    fn transition(&mut self, id: &str) -> Result<(), String> {
        let (mut next, world) =
            resources::session(&self.project, id, &self.scripts, &self.textures)?;
        if let Some(previous) = &mut self.slice {
            previous.stop(&mut self.world).map_err(|e| e.to_string())?;
            previous.transfer_game_data_to(&mut next);
        }
        next.host_mut()
            .set_storage(std::rc::Rc::new(storage::BrowserStorage {
                project: self.project.id.clone(),
            }));
        if let Some(audio) = &self.audio {
            next.host_mut()
                .set_platform_audio(Box::new(audio::AudioAdapter(audio.clone())));
        }
        self.world = world;
        self.slice = Some(next);
        self.scene_id = id.into();
        self.camera_controller = Default::default();
        self.camera = self.camera_controller.update(
            &self.project.cameras.get(id).cloned().unwrap_or_default(),
            &self.world,
            [
                self.project.window.width as f32,
                self.project.window.height as f32,
            ],
            0.0,
        )?;
        Ok(())
    }

    fn update(&mut self) {
        if !self.focused {
            self.game_loop = GameLoop::new();
            return;
        }
        let input = self.input.clone();
        let viewport = self.viewport();
        let mut keys: Vec<_> = input.keys.iter().cloned().collect();
        keys.extend(input::gamepad_keys());
        let mut key_edges = std::mem::take(&mut self.input.key_edges);
        let mut pointer_edges = std::mem::take(&mut self.input.pointer_edges);
        let mut transition = None;
        let mut restored = false;
        self.game_loop.tick(|dt| {
            if transition.is_some() || restored {
                return;
            }
            if let Some(slice) = &mut self.slice {
                let host = slice.host_mut();
                host.set_pressed_keys(&keys);
                for key in key_edges.drain(..) {
                    host.queue_ui_key(&key);
                }
                for (point, down) in pointer_edges.drain(..) {
                    host.queue_ui_pointer(viewport, point, down);
                }
                host.set_ui_pointer(
                    viewport,
                    Some([input.mouse_x as f32, input.mouse_y as f32]),
                    input.mouse_pressed,
                );
                host.set_mouse_state(input.mouse_x, input.mouse_y, input.mouse_just_pressed);
                if let Some(position) = self
                    .camera
                    .screen_to_world([input.mouse_x as f32, input.mouse_y as f32], viewport)
                {
                    host.set_mouse_world(position.map(|v| v as f64));
                }
                if let Err(error) = slice.step(&mut self.world, input.axes(), dt.as_secs_f64()) {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Script execution failed: {error}"
                    )));
                    return;
                }
                match slice.service_saves(
                    &mut self.world,
                    &self.project,
                    |path| {
                        self.scripts
                            .get(path)
                            .cloned()
                            .ok_or_else(|| format!("Missing script {path}"))
                    },
                    |path| self.textures.get(path).copied(),
                ) {
                    Ok(true) => {
                        self.scene_id = slice.scene_id().unwrap_or(&self.scene_id).to_owned();
                        self.camera_controller = Default::default();
                        restored = true;
                    }
                    Ok(false) => {}
                    Err(error) => web_sys::console::warn_1(&JsValue::from_str(&format!(
                        "Gameplay storage: {error}"
                    ))),
                }
                transition = slice.take_scene_request();
                if let Ok(camera) = self.camera_controller.update(
                    &self
                        .project
                        .cameras
                        .get(&self.scene_id)
                        .cloned()
                        .unwrap_or_default(),
                    &self.world,
                    [
                        self.project.window.width as f32,
                        self.project.window.height as f32,
                    ],
                    dt.as_secs_f32(),
                ) {
                    self.camera = camera;
                }
            }
        });
        // Do not drop quick edges when RAF ran before the next fixed tick.
        self.input.key_edges.splice(0..0, key_edges);
        self.input.pointer_edges.splice(0..0, pointer_edges);
        self.input.mouse_just_pressed = false;
        if let Some(id) = transition {
            if let Err(error) = self.transition(&id) {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Scene transition: {error}"
                )));
            }
        }
        if let Some(audio) = &self.audio {
            audio.borrow_mut().prune();
        }
    }

    fn render(&mut self, _window: &web_sys::Window) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                warn!("Surface texture error: {}", e);
                self.surface
                    .configure(self.renderer.device(), &self.renderer.surface_config());
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
        for (texture, instance) in
            kiln_runtime::visual::collect_scene_draws(&self.world, view_bounds)
        {
            self.sprite_renderer.push_textured(texture, instance);
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

            let viewport = self.viewport();
            if viewport[2] > 0.0 && viewport[3] > 0.0 {
                pass.set_viewport(viewport[0], viewport[1], viewport[2], viewport[3], 0.0, 1.0);
            }
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
            if let Some(slice) = self
                .slice
                .as_ref()
                .filter(|s| !s.host_ui().document().screens.is_empty())
            {
                for quad in slice.host_ui().draw(
                    &self.ui_fonts,
                    kiln_ui::authored::UiViewport::new(
                        self.viewport(),
                        slice.host_ui().document().reference_size,
                    ),
                ) {
                    self.overlay_renderer
                        .push_textured(quad.texture, quad.instance);
                }
            } else {
                let ui = UiRenderer::new();
                for command in commands {
                    for quad in command_quads(command, &ui) {
                        self.overlay_renderer
                            .push_textured(quad.texture, quad.instance);
                    }
                }
                let text = TextRenderer::new();
                for (index, line) in lines.iter().enumerate() {
                    for instance in
                        text.render(line, 16.0, 16.0 + index as f32 * 20.0, 16.0, [1.0; 4])
                    {
                        self.overlay_renderer.push(instance);
                    }
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
    if !resp.ok() {
        return Err(format!("HTTP resource error {}", resp.status()).into());
    }
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
) -> Result<resources::Textures, Box<dyn std::error::Error>> {
    let mut texture_paths = Vec::new();

    // Collect all unique texture paths from the project
    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let kiln_core::project::Component::Sprite {
                    texture: Some(path),
                    ..
                } = component
                {
                    if !texture_paths.contains(path) {
                        texture_paths.push(path.clone());
                    }
                }
            }
        }
    }

    for path in project
        .tilesets
        .values()
        .map(|set| &set.texture)
        .chain(project.animation_clips.values().map(|clip| &clip.texture))
    {
        if !path.is_empty() && !texture_paths.contains(path) {
            texture_paths.push(path.clone());
        }
    }
    texture_paths.sort();
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

        match load_png_texture(
            device,
            queue,
            sprite_renderer,
            &full_path,
            project.texture_options(path),
        )
        .await
        {
            Ok((tex_id, [width, height])) => {
                info!("Loaded texture: {} -> {:?}", path, tex_id);
                textures.insert(path.clone(), (tex_id, [width, height]));
            }
            Err(e) => {
                warn!("Failed to load texture {}: {}", path, e);
                // Fall back to white texture
                return Err(format!("Texture resource failed: {path}: {e}").into());
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
    if !resp.ok() {
        return Err(format!("HTTP resource error {}", resp.status()).into());
    }
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
    options: [bool; 2],
) -> Result<(TextureId, [u32; 2]), Box<dyn std::error::Error>> {
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
    let tex_id = sprite_renderer.create_texture_rgba_with_options(
        device,
        queue,
        url,
        [width, height],
        &rgba,
        options,
    );

    Ok((tex_id, [width, height]))
}

/// Spawn all entities from a scene, including tilemaps.
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
