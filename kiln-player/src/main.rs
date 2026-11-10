//! Kiln Player - thin launcher for standalone games
//! Loads a game from a packed archive (`.kar`) or a project directory and runs it.

use anyhow::Result;
use clap::Parser;
use kiln_core::{cache_dir, save_dir, settings_dir, Project};
use kiln_player::loader;
use kiln_player::spawn;
use kiln_player::textures::TextureLibrary;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{GameLoop, GameWorld, InputState, SliceSession};
use kiln_ui::{
    layout::Rect,
    localization::LocalizationKey,
    render::NineSlice,
    text::TextRenderer,
    ui_renderer::UiRenderer,
    widget::{Button, Label, Panel, Widget, WidgetId},
};
use log::info;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use winit::event::KeyEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

#[cfg(feature = "gamepad")]
use kiln_input::GamepadManager;
use kiln_input::{Action, ActionMap, Binding, BindingType};

/// The current state of the player application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuState {
    /// Main menu: Start, Load, Settings, Quit.
    Main,
    /// Save slot selection for loading a game.
    LoadGame,
    /// In-game: the actual game is running.
    Playing,
    /// Settings menu.
    Settings,
}

/// Kiln Player - run standalone games
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the game archive (.kar file) or project directory
    #[arg(short, long)]
    archive: Option<PathBuf>,

    /// Print the export runtime protocol without opening a window or audio device.
    #[arg(long)]
    runtime_info: bool,
    /// Print installed-tool identity without graphics/audio initialization.
    #[arg(long, exclusive = true)]
    tool_info: bool,

    /// Scene to start (defaults to the first scene in the project)
    #[arg(long)]
    scene: Option<String>,

    /// Game name (used for save/settings folders)
    #[arg(short, long)]
    game_name: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    if args.tool_info {
        let mut features = Vec::new();
        if cfg!(feature = "audio") {
            features.push("audio".into());
        }
        if cfg!(feature = "fonts") {
            features.push("fonts".into());
        }
        if cfg!(feature = "gamepad") {
            features.push("gamepad".into());
        }
        let info = kiln_core::distribution::ToolInfo::current(
            "kiln-player",
            features,
            cfg!(debug_assertions),
        );
        println!("{}", serde_json::to_string(&info)?);
        return Ok(());
    }
    if args.runtime_info {
        let runtime = kiln_core::export::RuntimeInfo {
            protocol: kiln_core::export::EXPORT_PROTOCOL,
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            audio: cfg!(feature = "audio"),
            fonts: cfg!(feature = "fonts"),
            gamepad: cfg!(feature = "gamepad"),
            physics: true,
            materials: true,
        };
        println!("{}", serde_json::to_string(&runtime)?);
        return Ok(());
    }
    let archive = args
        .archive
        .unwrap_or(std::env::current_exe()?.with_file_name("assets.kar"));
    info!("Loading game from: {}", archive.display());

    // Scope defaults to the actual project; explicit names select an alternate profile.
    let game = loader::load(&archive, args.scene.as_deref())?;
    let profile = args.game_name.as_deref().unwrap_or(&game.project.id);
    let save_path = save_dir(profile);
    let settings_path = settings_dir(profile);
    let cache_path = cache_dir(profile);

    info!("Save directory: {}", save_path.display());
    info!("Settings directory: {}", settings_path.display());
    info!("Cache directory: {}", cache_path.display());

    // Create directories if they don't exist
    std::fs::create_dir_all(&save_path)?;
    std::fs::create_dir_all(&settings_path)?;
    std::fs::create_dir_all(&cache_path)?;

    // Load the project and its assets
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

/// Build a default action map with common keyboard/gamepad bindings.
fn default_action_map() -> ActionMap {
    let mut map = ActionMap::new();
    map.add_action(
        Action::new("jump", "Jump")
            .with_binding(Binding::new(BindingType::Key("Space".to_string()))),
    );
    map.add_action(
        Action::new("fire", "Fire")
            .with_binding(Binding::new(BindingType::Key("KeyZ".to_string()))),
    );
    map.add_action(
        Action::new("interact", "Interact")
            .with_binding(Binding::new(BindingType::Key("KeyE".to_string()))),
    );
    map.add_action(
        Action::new("left", "Move Left")
            .with_binding(Binding::new(BindingType::Key("KeyA".to_string()))),
    );
    map.add_action(
        Action::new("right", "Move Right")
            .with_binding(Binding::new(BindingType::Key("KeyD".to_string()))),
    );
    map.add_action(
        Action::new("up", "Move Up")
            .with_binding(Binding::new(BindingType::Key("KeyW".to_string()))),
    );
    map.add_action(
        Action::new("down", "Move Down")
            .with_binding(Binding::new(BindingType::Key("KeyS".to_string()))),
    );
    map
}

/// UI widget types that can be rendered by UiRenderer.
#[derive(Debug, Clone)]
enum UiWidget {
    Button {
        id: WidgetId,
        bounds: Rect,
        text_key: LocalizationKey,
        hovered: bool,
        pressed: bool,
    },
    Label {
        id: WidgetId,
        bounds: Rect,
        text_key: LocalizationKey,
    },
    Panel {
        id: WidgetId,
        bounds: Rect,
    },
}

/// Convert a winit logical key to a string name for action map lookup.
fn key_name_string(key: &winit::keyboard::Key) -> String {
    match key {
        winit::keyboard::Key::Named(named) => match named {
            NamedKey::ArrowUp => "ArrowUp".to_string(),
            NamedKey::ArrowDown => "ArrowDown".to_string(),
            NamedKey::ArrowLeft => "ArrowLeft".to_string(),
            NamedKey::ArrowRight => "ArrowRight".to_string(),
            NamedKey::Space => "Space".to_string(),
            NamedKey::Enter => "Enter".to_string(),
            NamedKey::Escape => "Escape".to_string(),
            NamedKey::Shift => "Shift".to_string(),
            NamedKey::Control => "Control".to_string(),
            NamedKey::Alt => "Alt".to_string(),
            _ => format!("{:?}", named).to_string(),
        },
        winit::keyboard::Key::Character(c) => c.to_string(),
        winit::keyboard::Key::Dead(_) => "Dead".to_string(),
        winit::keyboard::Key::Unidentified(_) => "Unidentified".to_string(),
    }
}

/// GPU state that is only available after the window is created.
struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    sprite_renderer: SpriteRenderer,
    /// A second renderer for the screen-space HUD, which uses its own camera.
    hud_sprite_renderer: SpriteRenderer,
    ui_fonts: kiln_ui::authored::UiFonts,
}

/// The player application: owns the world, the camera and the GPU state.
struct PlayerApp {
    material_error: Option<String>,
    gpu: Option<GpuState>,
    camera: Camera,
    camera_controller: kiln_runtime::camera::CameraController,
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
    /// UI commands collected from the last script frame (buttons, labels, panels).
    ui_commands: Vec<kiln_runtime::SliceCommand>,
    /// UI widgets built from commands, ready for rendering.
    ui_widgets: Vec<UiWidget>,
    /// UI renderer for drawing widgets with nine-slice backgrounds and text.
    ui_renderer: UiRenderer,
    /// Current mouse position (screen space, top-left origin).
    mouse_x: f64,
    /// Current mouse position (screen space, top-left origin).
    mouse_y: f64,
    /// Whether the left mouse button is currently pressed.
    mouse_pressed: bool,
    /// Input action mapping for get_action/get_action_axis builtins.
    action_map: ActionMap,
    /// Gamepad manager for hot-plug and state polling (gamepad feature only).
    #[cfg(feature = "gamepad")]
    gamepad_manager: Option<GamepadManager>,
    /// Keys currently held down, used to drive the player.
    held: Vec<Key>,
    /// Currently pressed key names as strings (for action map lookup).
    pressed_keys: Vec<String>,
    focused: bool,
    /// Width of the game window, in logical pixels.
    window_width: u32,
    /// Height of the game window, in logical pixels.
    window_height: u32,
    /// The directory save slots are written to.
    save_dir: PathBuf,
    /// The save slot the game state is stored in.
    save_slot: u32,
    /// Current menu state.
    menu_state: MenuState,
    /// Selected index in the current menu.
    menu_selected: usize,
    /// Whether the GPU has been initialized (game started).
    gpu_initialized: bool,
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
            material_error: None,
            gpu: None,
            camera: Camera::new(width as f32, height as f32),
            camera_controller: Default::default(),
            game_loop: GameLoop::new(),
            world,
            input: InputState::new(),
            title: "Kiln Game".to_string(),
            project,
            scene_id,
            assets,
            textures: TextureLibrary::default(),
            slice: None,
            ui_commands: Vec::new(),
            ui_widgets: Vec::new(),
            ui_renderer: UiRenderer::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_pressed: false,
            action_map: default_action_map(),
            #[cfg(feature = "gamepad")]
            gamepad_manager: GamepadManager::new().ok(),
            held: Vec::new(),
            pressed_keys: Vec::new(),
            focused: true,
            window_width: width,
            window_height: height,
            save_dir,
            save_slot: 0,
            menu_state: MenuState::Main,
            menu_selected: 0,
            gpu_initialized: false,
        }
    }

    /// Initialize GPU resources (renderer, sprite renderers) without spawning the scene.
    /// This is called on startup so the menu can render.
    async fn init_gpu_only(&mut self, window: Arc<Window>) {
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
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.scene_format());
        sprite_renderer.upload_quad(renderer.queue());

        let hud_sprite_renderer =
            SpriteRenderer::new(renderer.device(), renderer.queue(), renderer.format());
        hud_sprite_renderer.upload_quad(renderer.queue());

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            sprite_renderer,
            hud_sprite_renderer,
            ui_fonts: Default::default(),
        });
        self.gpu_initialized = true;
    }

    /// Spawn the scene entities and load the gameplay script.
    /// Called when the user selects "Start Game" from the menu.
    fn spawn_scene_and_load_slice(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        if let Some(slice) = self.slice.as_mut() {
            if let Err(error) = slice.stop(&mut self.world) {
                log::error!("Stop callback: {error}");
            }
        }
        let game_progress = self.slice.as_ref().map(|s| s.game_progress());
        self.slice = None;
        self.world = GameWorld::new();
        self.game_loop = GameLoop::new();
        self.camera_controller = Default::default();

        // Decode the project's PNG assets and upload them as GPU textures.
        let (textures, errors) = TextureLibrary::upload_for_project(
            &mut gpu.sprite_renderer,
            gpu.renderer.device(),
            gpu.renderer.queue(),
            &self.project,
            &self.assets,
        );
        for error in &errors {
            log::warn!("{error}");
        }
        info!("Uploaded {} textures", textures.len());
        self.textures = textures;
        if let Err(error) = pollster::block_on(gpu.sprite_renderer.prepare_project_materials(
            gpu.renderer.device(),
            &self.project,
            |p| self.textures.get(p).map(|t| t.id),
        )) {
            log::error!("{error}");
            self.material_error = Some(error);
            return;
        }
        self.material_error = None;

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

        // The scene script needs the spawned entities, so it is loaded last.
        let _ = kiln_runtime::update_global_transforms(&mut self.world);
        let settings = self
            .project
            .cameras
            .get(&self.scene_id)
            .cloned()
            .unwrap_or_default();
        match self.camera_controller.update(
            &settings,
            &self.world,
            [
                self.project.window.width as f32,
                self.project.window.height as f32,
            ],
            0.0,
        ) {
            Ok(camera) => self.camera = camera,
            Err(error) => log::error!("Camera: {error}"),
        }
        self.load_slice(game_progress);
    }

    /// Compile the scene's gameplay script, if it has one.
    fn load_slice(&mut self, game_progress: Option<kiln_runtime::GameProgress>) {
        if let Some(gpu) = &mut self.gpu {
            gpu.hud_sprite_renderer.clear_asset_textures();
            let (fonts, errors) = kiln_ui::authored::UiFonts::load(
                &self.project.ui,
                |path| {
                    self.assets
                        .get(path)
                        .cloned()
                        .ok_or_else(|| format!("Missing font resource: {path}"))
                },
                |id, size, rgba| {
                    gpu.hud_sprite_renderer.create_texture_rgba_with_options(
                        gpu.renderer.device(),
                        gpu.renderer.queue(),
                        id,
                        size,
                        rgba,
                        [false, false],
                    )
                },
            );
            gpu.ui_fonts = fonts;
            for error in errors {
                log::error!("{error}");
            }
        }
        match SliceSession::with_scene_sources(&self.project, &self.scene_id, |path| {
            let bytes = self.assets.get(path).ok_or("Script resource is missing")?;
            String::from_utf8(bytes.clone()).map_err(|error| error.to_string())
        }) {
            Ok(mut session) => {
                #[cfg(feature = "audio")]
                {
                    for error in session.host_mut().configure_audio(&self.project, |path| {
                        self.assets
                            .get(path)
                            .cloned()
                            .ok_or_else(|| format!("Missing audio resource: {path}"))
                    }) {
                        log::error!("{error}");
                    }
                }
                session.host_mut().set_save_dir(&self.save_dir);
                if let Some(progress) = game_progress {
                    if let Err(error) = session.set_game_progress(progress) {
                        log::warn!("Game progress: {error}");
                    }
                }
                #[cfg(feature = "audio")]
                if !self.project.audio_assets.is_empty() {
                    if let Err(error) = session.host_mut().open_audio_output() {
                        log::warn!("Audio output unavailable: {error}");
                    }
                }
                info!("Loaded gameplay for scene '{}'", self.scene_id);
                self.slice = Some(session);
            }
            Err(error) => log::error!("{error}"),
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

    fn save_game(&mut self) {
        let Some(slice) = self.slice.as_mut() else {
            return;
        };
        match slice.save_game(&self.world, self.save_slot) {
            Ok(()) => log::info!("game saved to slot {}", self.save_slot),
            Err(e) => log::error!("could not save game: {e}"),
        }
    }

    fn load_game(&mut self) {
        let Some(slice) = self.slice.as_mut() else {
            return;
        };
        let result = slice.load_game(
            &mut self.world,
            self.save_slot,
            &self.project,
            |path| {
                self.assets
                    .get(path)
                    .ok_or_else(|| format!("Missing script {path}"))
                    .and_then(|bytes| String::from_utf8(bytes.clone()).map_err(|e| e.to_string()))
            },
            |path| self.textures.get(path).map(|t| (t.id, [t.width, t.height])),
        );
        match result {
            Ok(()) => {
                self.scene_id = slice.scene_id().unwrap_or(&self.scene_id).to_owned();
                self.camera_controller = Default::default();
                let settings = self
                    .project
                    .cameras
                    .get(&self.scene_id)
                    .cloned()
                    .unwrap_or_default();
                if let Ok(camera) = self.camera_controller.update(
                    &settings,
                    &self.world,
                    [
                        self.project.window.width as f32,
                        self.project.window.height as f32,
                    ],
                    0.0,
                ) {
                    self.camera = camera;
                }
                log::info!("game loaded from slot {}", self.save_slot);
            }
            Err(e) => log::warn!("could not load game: {e}"),
        }
    }

    fn queue_ui_pointer(&mut self, down: bool) {
        let area = self
            .gpu
            .as_ref()
            .map(|gpu| {
                let size = gpu.window.inner_size();
                [0, 0, size.width, size.height]
            })
            .unwrap_or([0, 0, self.window_width, self.window_height]);
        let viewport = self.camera.fit_viewport(area).map(|v| v as f32);
        if let Some(slice) = &mut self.slice {
            slice.host_mut().queue_ui_pointer(
                viewport,
                [self.mouse_x as f32, self.mouse_y as f32],
                down,
            );
        }
    }
    fn handle_key(&mut self, event: KeyEvent) {
        let key_name = key_name_string(&event.logical_key);
        if event.state.is_pressed() {
            if !event.repeat {
                if let Some(slice) = &mut self.slice {
                    let token =
                        if key_name == "Tab" && self.pressed_keys.iter().any(|k| k == "Shift") {
                            "BackTab"
                        } else {
                            &key_name
                        };
                    slice.host_mut().queue_ui_key(token);
                }
            }
            if !self.held.contains(&event.logical_key) {
                self.held.push(event.logical_key.clone());
            }
            if !self.pressed_keys.contains(&key_name) {
                self.pressed_keys.push(key_name);
            }

            // Handle menu navigation
            match self.menu_state {
                MenuState::Main => self.handle_main_menu(event.logical_key.clone()),
                MenuState::LoadGame => self.handle_load_game_menu(event.logical_key.clone()),
                MenuState::Settings => self.handle_settings_menu(event.logical_key.clone()),
                MenuState::Playing => self.handle_playing_keys(event.logical_key.clone()),
            }
        } else {
            self.held.retain(|key| *key != event.logical_key);
            self.pressed_keys.retain(|k| k != &key_name);
        }
    }

    /// Handle key input when in the main menu.
    fn handle_main_menu(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::ArrowUp) => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Character(c) if matches!(c.as_str(), "w" | "W") => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if self.menu_selected < 3 {
                    self.menu_selected += 1;
                }
            }
            Key::Character(c) if matches!(c.as_str(), "s" | "S") => {
                if self.menu_selected < 3 {
                    self.menu_selected += 1;
                }
            }
            Key::Named(NamedKey::Enter) => {
                match self.menu_selected {
                    0 => self.start_game(), // Start Game
                    1 => {
                        self.menu_state = MenuState::LoadGame;
                        self.menu_selected = 0;
                    } // Load Game
                    2 => {
                        self.menu_state = MenuState::Settings;
                        self.menu_selected = 0;
                    } // Settings
                    3 => {
                        std::process::exit(0);
                    } // Quit
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Handle key input when in the load game menu.
    fn handle_load_game_menu(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::ArrowUp) => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Character(c) if matches!(c.as_str(), "w" | "W") => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if self.menu_selected < 9 {
                    self.menu_selected += 1;
                }
            }
            Key::Character(c) if matches!(c.as_str(), "s" | "S") => {
                if self.menu_selected < 9 {
                    self.menu_selected += 1;
                }
            }
            Key::Named(NamedKey::Enter) => {
                self.save_slot = self.menu_selected as u32;
                self.load_game();
                self.start_game();
            }
            Key::Named(NamedKey::Escape) => {
                self.menu_state = MenuState::Main;
                self.menu_selected = 1; // Select "Load Game" when returning
            }
            _ => {}
        }
    }

    /// Handle key input when in the settings menu.
    fn handle_settings_menu(&mut self, key: Key) {
        if let Key::Named(NamedKey::Escape) = key {
            self.menu_state = MenuState::Main;
            self.menu_selected = 2; // Select "Settings" when returning
        }
    }

    /// Handle key input when playing the game.
    fn handle_playing_keys(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::Escape) if self.project.ui.screens.is_empty() => {
                self.menu_state = MenuState::Main;
                self.menu_selected = 0;
            }
            Key::Named(NamedKey::F5) => self.save_game(),
            Key::Named(NamedKey::F9) => self.load_game(),
            Key::Named(NamedKey::Space) if self.project.ui.screens.is_empty() => {
                self.camera.zoom(1.1)
            }
            Key::Named(NamedKey::Shift) if self.project.ui.screens.is_empty() => {
                self.camera.zoom(1.0 / 1.1)
            }
            _ => {}
        }
    }

    /// Initialize GPU and start the game.
    fn start_game(&mut self) {
        // If GPU not initialized yet, the menu would have initialized it already.
        // Spawn the scene and load the slice.
        self.spawn_scene_and_load_slice();
        self.menu_state = MenuState::Playing;
    }

    fn update(&mut self) {
        if !self.focused || self.menu_state != MenuState::Playing {
            self.game_loop = GameLoop::new();
            return;
        }
        // The scene script drives the game: it reads the input axis and
        // moves the player, collects pickups and keeps the score.
        #[cfg(feature = "gamepad")]
        {
            self.pressed_keys.retain(|key| !key.starts_with("Gamepad"));
            if let Some(gamepad) = &mut self.gamepad_manager {
                let _ = gamepad.update();
                self.pressed_keys.extend(gamepad.pressed_tokens());
            }
        }
        let axis = self.axis();
        let save_dir = self.save_dir.clone();
        let mouse_x = self.mouse_x;
        let mouse_y = self.mouse_y;
        let mouse_pressed = self.mouse_pressed;
        let mouse_area = self
            .gpu
            .as_ref()
            .map(|gpu| {
                let size = gpu.renderer.size();
                [0, 0, size.0, size.1]
            })
            .unwrap_or([0, 0, self.window_width, self.window_height]);
        let mouse_viewport = self
            .camera
            .fit_viewport(mouse_area)
            .map(|value| value as f32);
        let world = &mut self.world;
        let slice = &mut self.slice;
        let ui_commands = &mut self.ui_commands;
        let action_map = &mut self.action_map;
        let pressed_keys = &mut self.pressed_keys;
        let ui_widgets = &mut self.ui_widgets;
        let camera = &mut self.camera;
        let controller = &mut self.camera_controller;
        let camera_settings = self
            .project
            .cameras
            .get(&self.scene_id)
            .cloned()
            .unwrap_or_default();
        let project = &self.project;
        let assets = &self.assets;
        let textures = &self.textures;
        let mut restored_scene = None;
        let mut transition = None;
        self.game_loop.tick(|dt| {
            if transition.is_some() || restored_scene.is_some() {
                return;
            }
            if let Some(slice) = slice.as_mut() {
                if let Some(position) =
                    camera.screen_to_world([mouse_x as f32, mouse_y as f32], mouse_viewport)
                {
                    slice
                        .host_mut()
                        .set_mouse_world(position.map(|value| value as f64));
                }
                slice.host_mut().set_ui_pointer(
                    mouse_viewport,
                    Some([mouse_x as f32, mouse_y as f32]),
                    mouse_pressed,
                );
                // Pass mouse state to the slice host for UI click detection.
                slice
                    .host_mut()
                    .set_mouse_state(mouse_x, mouse_y, mouse_pressed);
                // Pass action map for get_action/get_action_axis builtins.
                slice.host_mut().set_action_map(action_map.clone());
                // Pass pressed keys for get_action/get_action_axis builtins.
                slice.host_mut().set_pressed_keys(pressed_keys);
                // Pass save directory for save_game/load_game builtins.
                slice.host_mut().set_save_dir(&save_dir);
                if let Err(e) = slice.step(world, axis, dt.as_secs_f64()) {
                    log::error!("scene script failed: {e}");
                }
                match slice.service_saves(
                    world,
                    project,
                    |path| {
                        assets
                            .get(path)
                            .ok_or_else(|| format!("Missing script {path}"))
                            .and_then(|bytes| {
                                String::from_utf8(bytes.clone()).map_err(|e| e.to_string())
                            })
                    },
                    |path| textures.get(path).map(|t| (t.id, [t.width, t.height])),
                ) {
                    Ok(true) => {
                        restored_scene = slice.scene_id().map(str::to_owned);
                        return;
                    }
                    Ok(false) => {}
                    Err(error) => log::warn!("Gameplay storage: {error}"),
                }
                transition = slice.take_scene_request();
                let _ = kiln_runtime::scene::update_global_transforms(world);
                match controller.update(
                    &camera_settings,
                    world,
                    camera.viewport_size,
                    dt.as_secs_f32(),
                ) {
                    Ok(updated) => *camera = updated,
                    Err(error) => log::error!("Camera: {error}"),
                }
                // Collect UI commands from the last script frame and build widgets.
                let commands = slice.take_ui_commands();
                *ui_commands = commands.clone();
                *ui_widgets = commands
                    .iter()
                    .filter_map(|cmd| match cmd {
                        kiln_runtime::SliceCommand::UiButton { id, x, y } => {
                            let bounds = Rect::new(*x as f32, *y as f32, 64.0, 32.0);
                            let hovered = mouse_x >= bounds.x as f64
                                && mouse_x < (bounds.x + bounds.width) as f64
                                && mouse_y >= bounds.y as f64
                                && mouse_y < (bounds.y + bounds.height) as f64;
                            let pressed = hovered && mouse_pressed;
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            id.hash(&mut hasher);
                            Some(UiWidget::Button {
                                id: WidgetId::from_u64(hasher.finish()),
                                bounds,
                                text_key: LocalizationKey::new(format!("btn_{id}")),
                                hovered,
                                pressed,
                            })
                        }
                        kiln_runtime::SliceCommand::UiLabel { id, x, y, text } => {
                            let bounds = Rect::new(*x as f32, *y as f32, 200.0, 24.0);
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            id.hash(&mut hasher);
                            Some(UiWidget::Label {
                                id: WidgetId::from_u64(hasher.finish()),
                                bounds,
                                text_key: LocalizationKey::new(text.clone()),
                            })
                        }
                        kiln_runtime::SliceCommand::UiPanel { id, x, y, w, h } => {
                            let bounds = Rect::new(*x as f32, *y as f32, *w as f32, *h as f32);
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            id.hash(&mut hasher);
                            Some(UiWidget::Panel {
                                id: WidgetId::from_u64(hasher.finish()),
                                bounds,
                            })
                        }
                        _ => None,
                    })
                    .collect();
            }
            // Step physics (if the physics feature is enabled)
            // Physics stepping happens inside slice.step() via sync_physics_with_ecs().
        });
        if let Some(id) = restored_scene {
            self.scene_id = id;
            self.camera_controller = Default::default();
            let settings = self
                .project
                .cameras
                .get(&self.scene_id)
                .cloned()
                .unwrap_or_default();
            if let Ok(camera) = self.camera_controller.update(
                &settings,
                &self.world,
                [
                    self.project.window.width as f32,
                    self.project.window.height as f32,
                ],
                0.0,
            ) {
                self.camera = camera;
            }
        }
        if let Some(id) = transition {
            if self.project.scenes.contains_key(&id) {
                if let Some(slice) = self.slice.as_mut() {
                    let _ = slice.stop(&mut self.world);
                }
                self.slice = None;
                self.world = GameWorld::new();
                self.scene_id = id;
                self.spawn_scene_and_load_slice();
            } else {
                log::error!("Unknown scene transition '{id}'");
            }
        }
    }

    fn render(&mut self) {
        // If we're in Playing state but GPU isn't initialized yet, initialize it now.
        if self.menu_state == MenuState::Playing && !self.gpu_initialized {
            // The GPU will be initialized in resumed(), but we need to wait for it.
            // For now, just return and let the next frame handle it.
            return;
        }

        // If in a menu state, render the menu (requires GPU).
        if let Some(error) = &self.material_error {
            if let Some(gpu) = &self.gpu {
                gpu.window
                    .set_title(&format!("Kiln material error: {error}"));
            }
            self.menu_state = MenuState::Main;
        }
        if self.menu_state != MenuState::Playing {
            self.render_menu();
            return;
        }

        // Game rendering (original logic).
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
        let _ = kiln_runtime::update_global_transforms(&mut self.world);
        gpu.sprite_renderer.begin_frame();

        // Tilemaps are drawn first so entities appear on top of them.
        // Only the chunks inside the camera view are submitted.
        let view_bounds = self.camera.view_bounds();
        for (texture, instance) in kiln_runtime::visual::collect_scene_draws_with_materials(
            &self.world,
            view_bounds,
            |name| gpu.sprite_renderer.material_id(name),
        ) {
            gpu.sprite_renderer.push_textured(texture, instance);
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

            let [x, y, width, height] = self.camera.fit_viewport([0, 0, size.width, size.height]);
            pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
            pass.set_scissor_rect(x, y, width, height);
            gpu.sprite_renderer
                .flush(&mut pass, &self.camera, gpu.renderer.queue());
        }

        if has_post {
            gpu.renderer.apply_post(&mut encoder, &view);
        }

        // Render UI widgets using UiRenderer (screen-space, top-left origin).
        if !self.ui_widgets.is_empty()
            || self
                .slice
                .as_ref()
                .is_some_and(|slice| !slice.host_ui().active_screen().is_empty())
        {
            let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_pass"),
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
            let ui_camera = Camera::new_top_left(size.width as f32, size.height as f32);
            gpu.hud_sprite_renderer.begin_frame();

            for widget in &self.ui_widgets {
                match widget {
                    UiWidget::Button {
                        id,
                        bounds,
                        text_key,
                        hovered,
                        pressed,
                    } => {
                        let mut button = Button::new(*id, text_key.clone());
                        button.set_bounds(Rect::new(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        ));
                        // Set button state colors based on hover/press
                        let normal = NineSlice::new(1, 4.0);
                        let hovered_ns = NineSlice::new(2, 4.0);
                        let pressed_ns = NineSlice::new(3, 4.0);
                        let button = if *pressed {
                            button.with_normal(pressed_ns)
                        } else if *hovered {
                            button.with_normal(hovered_ns)
                        } else {
                            button.with_normal(normal)
                        };
                        let quads = self.ui_renderer.render_button(&button, *hovered, *pressed);
                        for quad in quads {
                            gpu.hud_sprite_renderer
                                .push_textured(quad.texture, quad.instance);
                        }
                    }
                    UiWidget::Label {
                        id,
                        bounds,
                        text_key,
                    } => {
                        let mut label = Label::new(*id, text_key.clone());
                        label.set_bounds(Rect::new(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        ));
                        let quads = self.ui_renderer.render_label(&label);
                        for instance in quads {
                            gpu.hud_sprite_renderer
                                .push_textured(instance.texture, instance.instance);
                        }
                    }
                    UiWidget::Panel { id, bounds } => {
                        let mut panel = Panel::new(*id);
                        panel.set_bounds(Rect::new(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        ));
                        let quads = self.ui_renderer.render_panel(&panel);
                        for instance in quads {
                            gpu.hud_sprite_renderer
                                .push_textured(instance.texture, instance.instance);
                        }
                    }
                }
            }

            if let Some(slice) = &self.slice {
                let ui = slice.host_ui();
                let area = self
                    .camera
                    .fit_viewport([0, 0, size.width, size.height])
                    .map(|v| v as f32);
                let viewport =
                    kiln_ui::authored::UiViewport::new(area, ui.document().reference_size);
                for q in ui.draw(&gpu.ui_fonts, viewport) {
                    gpu.hud_sprite_renderer.push_textured(q.texture, q.instance);
                }
            }
            gpu.hud_sprite_renderer
                .flush(&mut ui_pass, &ui_camera, gpu.renderer.queue());
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
        if !self.project.ui.screens.is_empty() {
            return None;
        }
        let slice = self.slice.as_ref()?;
        let text = TextRenderer::new();

        let mut instances = Vec::new();
        for (index, line) in slice.hud_lines().iter().enumerate() {
            let y = 16.0 + index as f32 * 20.0;
            instances.extend(text.render(line, 16.0, y, 16.0, [1.0, 1.0, 1.0, 1.0]));
        }
        Some(instances)
    }

    /// Render the menu (main menu, load game, settings).
    fn render_menu(&mut self) {
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

        // Build menu text using TextRenderer.
        let text = TextRenderer::new();
        let mut instances = Vec::new();

        match self.menu_state {
            MenuState::Main => {
                let options = ["Start Game", "Load Game", "Settings", "Quit"];
                let title = &self.project.name;
                let title_quads = text.render(title, 16.0, 50.0, 32.0, [1.0, 1.0, 1.0, 1.0]);
                instances.extend(title_quads);

                for (i, opt) in options.iter().enumerate() {
                    let y = 150.0 + i as f32 * 40.0;
                    let color = if i == self.menu_selected {
                        [1.0, 1.0, 0.0, 1.0] // Yellow for selected
                    } else {
                        [1.0, 1.0, 1.0, 1.0] // White for normal
                    };
                    let prefix = if i == self.menu_selected { "> " } else { "  " };
                    let quads = text.render(&format!("{}{}", prefix, opt), 16.0, y, 24.0, color);
                    instances.extend(quads);
                }
            }
            MenuState::LoadGame => {
                let title_quads = text.render(
                    "Load Game - Select Slot",
                    16.0,
                    50.0,
                    32.0,
                    [1.0, 1.0, 1.0, 1.0],
                );
                instances.extend(title_quads);

                for i in 0..10 {
                    let y = 150.0 + i as f32 * 35.0;
                    let color = if i == self.menu_selected {
                        [1.0, 1.0, 0.0, 1.0]
                    } else {
                        [1.0, 1.0, 1.0, 1.0]
                    };
                    let prefix = if i == self.menu_selected { "> " } else { "  " };
                    // Check if save exists
                    let save_path = self.save_dir.join(format!("slot_{}.json", i));
                    let exists = save_path.exists();
                    let label = if exists {
                        "Slot {} (saved)"
                    } else {
                        "Slot {} (empty)"
                    };
                    let quads = text.render(
                        &format!("{}{}", prefix, label.replace("{}", &i.to_string())),
                        16.0,
                        y,
                        20.0,
                        color,
                    );
                    instances.extend(quads);
                }
                let esc_text = text.render(
                    "ESC - Back",
                    16.0,
                    150.0 + 10.0 * 35.0 + 20.0,
                    18.0,
                    [0.7, 0.7, 0.7, 1.0],
                );
                instances.extend(esc_text);
            }
            MenuState::Settings => {
                let title_quads = text.render("Settings", 16.0, 50.0, 32.0, [1.0, 1.0, 1.0, 1.0]);
                instances.extend(title_quads);

                let esc_text = text.render("ESC - Back", 16.0, 150.0, 20.0, [0.7, 0.7, 0.7, 1.0]);
                instances.extend(esc_text);

                let info_text = text.render(
                    "Settings UI not yet implemented.",
                    16.0,
                    200.0,
                    20.0,
                    [0.7, 0.7, 0.7, 1.0],
                );
                instances.extend(info_text);
            }
            MenuState::Playing => {}
        }

        if instances.is_empty() {
            return;
        }

        let gpu = self.gpu.as_mut().unwrap();
        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("menu_encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("menu_pass"),
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
        // Initialize GPU only (for menu rendering). Scene is spawned when user starts game.
        pollster::block_on(self.init_gpu_only(window));
        if !self.project.ui.screens.is_empty() {
            self.start_game();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                if let Some(slice) = self.slice.as_mut() {
                    let _ = slice.stop(&mut self.world);
                }
                event_loop.exit();
            }
            winit::event::WindowEvent::Focused(focused) => {
                self.focused = focused;
                if let Some(slice) = self.slice.as_mut() {
                    slice.host_mut().reset_ui_input();
                }
                self.held.clear();
                self.pressed_keys.clear();
                self.mouse_pressed = false;
                self.game_loop = GameLoop::new();
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            winit::event::WindowEvent::Resized(_) => {
                // Keep the configured world extent when the physical window
                // changes. Rendering and input mapping use the same viewport.
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;
            }
            winit::event::WindowEvent::Touch(touch) => match touch.phase {
                winit::event::TouchPhase::Started => {
                    self.mouse_x = touch.location.x;
                    self.mouse_y = touch.location.y;
                    self.mouse_pressed = true;
                    self.queue_ui_pointer(true);
                }
                winit::event::TouchPhase::Moved => {
                    self.mouse_x = touch.location.x;
                    self.mouse_y = touch.location.y;
                }
                winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                    self.mouse_x = touch.location.x;
                    self.mouse_y = touch.location.y;
                    self.mouse_pressed = false;
                    self.queue_ui_pointer(false);
                }
            },
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = state.is_pressed();
                self.queue_ui_pointer(self.mouse_pressed);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.update();
        self.render();
    }
}

#[cfg(test)]
mod saved_ui_host_tests {
    use super::*;
    #[test]
    fn authored_menus_receive_escape_and_space_without_launcher_or_camera_side_effects() {
        let mut project = Project::default();
        project
            .ui
            .screens
            .insert("title".into(), kiln_core::ui::UiScreen::default());
        let mut app = PlayerApp::new(
            GameWorld::new(),
            640,
            360,
            project,
            "main".into(),
            HashMap::new(),
            std::env::temp_dir(),
        );
        app.menu_state = MenuState::Playing;
        let before = app.camera.zoom;
        app.handle_playing_keys(Key::Named(NamedKey::Escape));
        assert_eq!(app.menu_state, MenuState::Playing);
        app.handle_playing_keys(Key::Named(NamedKey::Space));
        assert_eq!(app.camera.zoom, before);
        // Legacy games keep the existing launcher shortcut.
        app.project.ui.screens.clear();
        app.handle_playing_keys(Key::Named(NamedKey::Escape));
        assert_eq!(app.menu_state, MenuState::Main);
    }
}
