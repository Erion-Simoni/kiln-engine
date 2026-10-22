//! Kiln Player - thin launcher for standalone games
//! Loads a game from a packed archive (`.kar`) or a project directory and runs it.

use anyhow::Result;
use clap::Parser;
use kiln_core::{cache_dir, save_dir, settings_dir, Project, Transform};
use kiln_player::loader;
use kiln_player::spawn;
use kiln_player::textures::TextureLibrary;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{GameLoop, GameWorld, InputState, SliceSession, Sprite, Tilemap};
use kiln_ui::TextRenderer;
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use winit::event::KeyEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

/// Kiln Player - run standalone games
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the game archive (.kar file) or project directory
    #[arg(short, long)]
    archive: PathBuf,

    /// Scene to start (defaults to the first scene in the project)
    #[arg(long)]
    scene: Option<String>,

    /// Game name (used for save/settings folders)
    #[arg(short, long, default_value = "KilnGame")]
    game_name: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    info!("Loading game from: {}", args.archive.display());

    // Initialize platform-specific paths
    let save_path = save_dir(&args.game_name);
    let settings_path = settings_dir(&args.game_name);
    let cache_path = cache_dir(&args.game_name);

    info!("Save directory: {}", save_path.display());
    info!("Settings directory: {}", settings_path.display());
    info!("Cache directory: {}", cache_path.display());

    // Create directories if they don't exist
    std::fs::create_dir_all(&save_path)?;
    std::fs::create_dir_all(&settings_path)?;
    std::fs::create_dir_all(&cache_path)?;

    // Load the project and its assets
    let game = loader::load(&args.archive, args.scene.as_deref())?;
    info!(
        "Loaded '{}' from {}",
        game.project.name,
        game.source.describe()
    );
    info!("Scene '{}' selected", game.scene_id);

    // Spawn the scene entities into the runtime world.
    // The scene is spawned once the GPU is ready, because sprite textures
    // have to be uploaded before entities can reference them.
    let world = GameWorld::new();

    let window_settings = &game.project.window;
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = PlayerApp::new(
        world,
        window_settings.width,
        window_settings.height,
        game.project,
        game.scene_id,
        game.assets,
        save_path,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Player application
// ---------------------------------------------------------------------------

/// GPU state that is only available after the window is created.
struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    /// A second renderer for the screen-space HUD, which uses its own camera.
    hud_sprite_renderer: SpriteRenderer,
}

/// The player application: owns the world, the camera and the GPU state.
struct PlayerApp {
    gpu: Option<GpuState>,
    camera: Camera,
    game_loop: GameLoop,
    world: GameWorld,
    #[allow(dead_code)]
    input: InputState,
    title: String,
    project: Project,
    scene_id: String,
    assets: HashMap<String, Vec<u8>>,
    textures: TextureLibrary,
    /// The gameplay script of the loaded scene, if it has one.
    slice: Option<SliceSession>,
    /// Keys currently held down, used to drive the player.
    held: Vec<Key>,
    /// Width of the game window, in logical pixels.
    window_width: u32,
    /// Height of the game window, in logical pixels.
    window_height: u32,
    /// The directory save slots are written to.
    save_dir: PathBuf,
    /// The save slot the game state is stored in.
    save_slot: u32,
}

impl PlayerApp {
    /// Create the application around an already populated world.
    fn new(
        world: GameWorld,
        width: u32,
        height: u32,
        project: Project,
        scene_id: String,
        assets: HashMap<String, Vec<u8>>,
        save_dir: PathBuf,
    ) -> Self {
        Self {
            gpu: None,
            camera: Camera::new(width as f32, height as f32),
            game_loop: GameLoop::new(),
            world,
            input: InputState::new(),
            title: "Kiln Game".to_string(),
            project,
            scene_id,
            assets,
            textures: TextureLibrary::default(),
            slice: None,
            held: Vec::new(),
            window_width: width,
            window_height: height,
            save_dir,
            save_slot: 0,
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
        // The scene may be rendered into an intermediate HDR target when
        // post-processing is enabled, so target that format.
        let mut sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.scene_format());
        sprite_renderer.upload_quad(renderer.queue());

        // The HUD is drawn directly to the swap chain, so it uses the surface
        // format and a renderer of its own.
        let hud_sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.format());
        hud_sprite_renderer.upload_quad(renderer.queue());

        // Decode the project's PNG assets and upload them as GPU textures.
        let (textures, errors) = TextureLibrary::upload_for_project(
            &mut sprite_renderer,
            renderer.device(),
            renderer.queue(),
            &self.project,
            &self.assets,
        );
        for error in &errors {
            log::warn!("{error}");
        }
        info!("Uploaded {} textures", textures.len());
        self.textures = textures;

        // Now that the textures exist, spawn the scene with them bound.
        let spawned = spawn::spawn_scene_by_id_with_textures(
            &mut self.world,
            &self.project,
            &self.scene_id,
            &self.textures,
        );
        info!("Spawned {} entities", spawned.len());

        // Tilemaps are separate components: they are culled and drawn in
        // chunks rather than as individual entities.
        if let Some(scene) = self.project.scenes.get(&self.scene_id) {
            let maps = spawn::spawn_tilemaps(
                &mut self.world,
                &self.project,
                scene,
                kiln_core::SceneOffset::default(),
                &self.textures,
            );
            info!("Spawned {} tilemaps", maps.len());
        }

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
            hud_sprite_renderer,
        });

        // The scene script needs the spawned entities, so it is loaded last.
        self.load_slice();
    }

    /// Compile the scene's gameplay script, if it has one.
    fn load_slice(&mut self) {
        let Some(scene) = self.project.scenes.get(&self.scene_id) else {
            return;
        };
        let Some(path) = scene.script.clone() else {
            return;
        };
        let Some(source) = self.assets.get(&path) else {
            log::warn!("scene script '{path}' not found in the project assets");
            return;
        };
        let source = match std::str::from_utf8(source) {
            Ok(text) => text.to_string(),
            Err(e) => {
                log::warn!("scene script '{path}' is not valid UTF-8: {e}");
                return;
            }
        };
        match SliceSession::new(&source) {
            Ok(session) => {
                info!("Loaded scene script '{path}'");
                self.slice = Some(session);
                // Resume the game from its save, if there is one.
                self.load_game();
            }
            Err(diagnostics) => {
                for diagnostic in diagnostics {
                    log::error!("scene script '{path}': {}", diagnostic.message);
                }
            }
        }
    }

    /// The movement axis from the keys currently held down.
    fn axis(&self) -> (f64, f64) {
        let mut x = 0.0;
        let mut y = 0.0;
        for key in &self.held {
            match key {
                Key::Named(NamedKey::ArrowLeft) => x -= 1.0,
                Key::Named(NamedKey::ArrowRight) => x += 1.0,
                Key::Named(NamedKey::ArrowUp) => y += 1.0,
                Key::Named(NamedKey::ArrowDown) => y -= 1.0,
                Key::Character(c) => match c.as_str() {
                    "a" | "A" => x -= 1.0,
                    "d" | "D" => x += 1.0,
                    "w" | "W" => y += 1.0,
                    "s" | "S" => y -= 1.0,
                    _ => {}
                },
                _ => {}
            }
        }
        (x, y)
    }

    /// Save the game state to the save slot.
    fn save_game(&mut self) {
        let Some(slice) = self.slice.as_ref() else {
            return;
        };
        match kiln_runtime::save_state(&self.save_dir, self.save_slot, slice.state()) {
            Ok(()) => log::info!("game saved to slot {}", self.save_slot),
            Err(e) => log::error!("could not save the game: {e}"),
        }
    }

    /// Load the game state from the save slot.
    fn load_game(&mut self) {
        let Some(slice) = self.slice.as_mut() else {
            return;
        };
        match kiln_runtime::load_state(&self.save_dir, self.save_slot) {
            Ok(state) => {
                slice.set_state(state);
                slice.sync_world(&mut self.world);
                log::info!("game loaded from slot {}", self.save_slot);
            }
            Err(e) => log::warn!("could not load the game: {e}"),
        }
    }

    fn handle_key(&mut self, event: KeyEvent) {
        if event.state.is_pressed() {
            if !self.held.contains(&event.logical_key) {
                self.held.push(event.logical_key.clone());
            }
            match event.logical_key {
                Key::Named(NamedKey::Escape) => {}
                Key::Named(NamedKey::F5) => self.save_game(),
                Key::Named(NamedKey::F9) => self.load_game(),
                Key::Named(NamedKey::Space) => self.camera.zoom(1.1),
                Key::Named(NamedKey::Shift) => self.camera.zoom(1.0 / 1.1),
                _ => {}
            }
        } else {
            self.held.retain(|key| *key != event.logical_key);
        }
    }

    fn update(&mut self) {
        // The scene script drives the game: it reads the input axis and
        // moves the player, collects pickups and keeps the score.
        let axis = self.axis();
        let PlayerApp {
            game_loop,
            world,
            slice,
            ..
        } = self;
        game_loop.tick(|dt| {
            if let Some(slice) = slice.as_mut() {
                if let Err(e) = slice.step(world, axis, dt.as_secs_f64()) {
                    log::error!("scene script failed: {e}");
                }
            }
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

        // Collect sprite instances from the ECS world (CPU gather pass).
        gpu.sprite_renderer.begin_frame();

        // Tilemaps are drawn first so entities appear on top of them.
        // Only the chunks inside the camera view are submitted.
        let view_bounds = self.camera.view_bounds();
        for (_, tilemap) in self.world.world_mut().query_mut::<(&Transform, &Tilemap)>() {
            for (texture, instance) in tilemap.collect_visible(view_bounds) {
                gpu.sprite_renderer.push_textured(texture, instance);
            }
        }

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

        // The HUD is drawn in screen space (top-left origin) on top of the
        // scene, so it uses its own renderer and camera.
        let hud_text = self.hud_text();
        let gpu = self.gpu.as_mut().unwrap();
        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame_encoder"),
                });

        let has_post = gpu.renderer.has_post();

        {
            let target_view = gpu.renderer.scene_target().unwrap_or(&view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
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

            gpu.sprite_renderer
                .flush(&mut pass, &self.camera, gpu.renderer.queue());
        }

        if has_post {
            gpu.renderer.apply_post(&mut encoder, &view);
        }

        // The HUD is drawn in screen space (top-left origin) on top of the
        // scene, so it uses its own renderer and camera.
        if let Some(instances) = hud_text {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud_pass"),
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
            gpu.hud_sprite_renderer.begin_frame();
            for instance in instances {
                gpu.hud_sprite_renderer.push(instance);
            }
            let hud_camera = Camera::new_top_left(size.width as f32, size.height as f32);
            gpu.hud_sprite_renderer
                .flush(&mut pass, &hud_camera, gpu.renderer.queue());
        }

        gpu.renderer.queue().submit([encoder.finish()]);
        frame.present();
        gpu.window.request_redraw();
    }

    /// Build the HUD text of the current frame.
    ///
    /// Returns `None` when the project has no gameplay script, so nothing is
    /// drawn for non-game projects.
    fn hud_text(&self) -> Option<Vec<SpriteInstance>> {
        let slice = self.slice.as_ref()?;
        let text = TextRenderer::new();

        let mut instances = Vec::new();
        for (index, line) in slice.hud_lines().iter().enumerate() {
            let y = 16.0 + index as f32 * 20.0;
            instances.extend(text.render(line, 16.0, y, 16.0, [1.0, 1.0, 1.0, 1.0]));
        }
        Some(instances)
    }
}

impl winit::application::ApplicationHandler for PlayerApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title(self.title.clone())
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.window_width,
                            self.window_height,
                        )),
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
