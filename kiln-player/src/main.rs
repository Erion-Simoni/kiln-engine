//! Kiln Player - thin launcher for standalone games
//! Loads a game from a packed archive (`.kar`) or a project directory and runs it.

use anyhow::Result;
use clap::Parser;
use kiln_core::{cache_dir, save_dir, settings_dir, Project, Transform};
use kiln_player::loader;
use kiln_player::spawn;
use kiln_player::textures::TextureLibrary;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, Renderer, SpriteRenderer};
use kiln_runtime::{
    GameLoop, GameWorld, InputState, SliceSession, Sprite, Tilemap,
    sync_physics_with_ecs,
};
use kiln_ui::{
    layout::Rect,
    ui_renderer::UiRenderer,
    widget::{Button, Label, Panel, WidgetId, Widget},
    localization::LocalizationKey,
    render::NineSlice,
};
use log::info;
use std::collections::HashMap;
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

/// Build a default action map with common keyboard/gamepad bindings.
fn default_action_map() -> ActionMap {
    let mut map = ActionMap::new();
    map.add_action(Action::new("jump", "Jump").with_binding(Binding::new(BindingType::Key("Space".to_string()))));
    map.add_action(Action::new("fire", "Fire").with_binding(Binding::new(BindingType::Key("KeyZ".to_string()))));
    map.add_action(Action::new("interact", "Interact").with_binding(Binding::new(BindingType::Key("KeyE".to_string()))));
    map.add_action(Action::new("left", "Move Left").with_binding(Binding::new(BindingType::Key("KeyA".to_string()))));
    map.add_action(Action::new("right", "Move Right").with_binding(Binding::new(BindingType::Key("KeyD".to_string()))));
    map.add_action(Action::new("up", "Move Up").with_binding(Binding::new(BindingType::Key("KeyW".to_string()))));
    map.add_action(Action::new("down", "Move Down").with_binding(Binding::new(BindingType::Key("KeyS".to_string()))));
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
            _ => named.as_ref().to_string(),
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
        let mut sprite_renderer =
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
        });
        self.gpu_initialized = true;
    }

    /// Spawn the scene entities and load the gameplay script.
    /// Called when the user selects "Start Game" from the menu.
    fn spawn_scene_and_load_slice(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

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
        self.load_slice();
    }

    /// Initialize GPU and spawn scene (legacy combined function, kept for reference).
    /// Now split into `init_gpu_only` + `spawn_scene_and_load_slice`.
    async fn init_gpu(&mut self, window: Arc<Window>) {
        self.init_gpu_only(window).await;
        self.spawn_scene_and_load_slice();
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

    /// Check if mouse is over a rect (screen space, top-left origin).
    fn is_mouse_over(&self, bounds: Rect) -> bool {
        self.mouse_x >= bounds.x as f64
            && self.mouse_x < (bounds.x + bounds.width) as f64
            && self.mouse_y >= bounds.y as f64
            && self.mouse_y < (bounds.y + bounds.height) as f64
    }
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
        let key_name = key_name_string(&event.logical_key);
        if event.state.is_pressed() {
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
            Key::Named(NamedKey::ArrowUp) | Key::Character(c) if c.as_str() == "w" || c.as_str() == "W" => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Named(NamedKey::ArrowDown) | Key::Character(c) if c.as_str() == "s" || c.as_str() == "S" => {
                if self.menu_selected < 3 {
                    self.menu_selected += 1;
                }
            }
            Key::Named(NamedKey::Enter) => {
                match self.menu_selected {
                    0 => self.start_game(), // Start Game
                    1 => { self.menu_state = MenuState::LoadGame; self.menu_selected = 0; } // Load Game
                    2 => { self.menu_state = MenuState::Settings; self.menu_selected = 0; } // Settings
                    3 => { std::process::exit(0); } // Quit
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Handle key input when in the load game menu.
    fn handle_load_game_menu(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::ArrowUp) | Key::Character(c) if c.as_str() == "w" || c.as_str() == "W" => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            Key::Named(NamedKey::ArrowDown) | Key::Character(c) if c.as_str() == "s" || c.as_str() == "S" => {
                if self.menu_selected < 9 {
                    self.menu_selected += 1;
                }
            }
            Key::Named(NamedKey::Enter) => {
                self.save_slot = self.menu_selected;
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
        match key {
            Key::Named(NamedKey::Escape) => {
                self.menu_state = MenuState::Main;
                self.menu_selected = 2; // Select "Settings" when returning
            }
            _ => {}
        }
    }

    /// Handle key input when playing the game.
    fn handle_playing_keys(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.menu_state = MenuState::Main;
                self.menu_selected = 0;
            }
            Key::Named(NamedKey::F5) => self.save_game(),
            Key::Named(NamedKey::F9) => self.load_game(),
            Key::Named(NamedKey::Space) => self.camera.zoom(1.1),
            Key::Named(NamedKey::Shift) => self.camera.zoom(1.0 / 1.1),
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
        // The scene script drives the game: it reads the input axis and
        // moves the player, collects pickups and keeps the score.
        let axis = self.axis();
        let PlayerApp {
            game_loop,
            world,
            slice,
            ui_commands,
            action_map,
            mouse_x,
            mouse_y,
            mouse_pressed,
            pressed_keys,
            ..
        } = self;
        game_loop.tick(|dt| {
            if let Some(slice) = slice.as_mut() {
                // Pass mouse state to the slice host for UI click detection.
                slice.set_mouse_state(*mouse_x, *mouse_y, *mouse_pressed);
                // Pass action map for get_action/get_action_axis builtins.
                slice.set_action_map(action_map.clone());
                // Pass pressed keys for get_action/get_action_axis builtins.
                slice.set_pressed_keys(pressed_keys);
                // Pass save directory for save_game/load_game builtins.
                slice.set_save_dir(&self.save_dir);
                if let Err(e) = slice.step(world, axis, dt.as_secs_f64()) {
                    log::error!("scene script failed: {e}");
                }
                // Collect UI commands from the last script frame and build widgets.
                let commands = slice.take_ui_commands();
                *ui_commands = commands.clone();
                *ui_widgets = commands
                    .iter()
                    .filter_map(|cmd| match cmd {
                        kiln_runtime::SliceCommand::UiButton { id, x, y } => {
                            let bounds = Rect::new(*x as f32, *y as f32, 64.0, 32.0);
                            let hovered = self.is_mouse_over(bounds);
                            let pressed = hovered && self.mouse_pressed;
                            Some(UiWidget::Button {
                                id: WidgetId::from_u64(id.hash()),
                                bounds,
                                text_key: LocalizationKey::new(format!("btn_{id}")),
                                hovered,
                                pressed,
                            })
                        }
                        kiln_runtime::SliceCommand::UiLabel { id, x, y, text } => {
                            let bounds = Rect::new(*x as f32, *y as f32, 200.0, 24.0);
                            Some(UiWidget::Label {
                                id: WidgetId::from_u64(id.hash()),
                                bounds,
                                text_key: LocalizationKey::new(text.clone()),
                            })
                        }
                        kiln_runtime::SliceCommand::UiPanel { id, x, y, w, h } => {
                            let bounds = Rect::new(*x as f32, *y as f32, *w as f32, *h as f32);
                            Some(UiWidget::Panel {
                                id: WidgetId::from_u64(id.hash()),
                                bounds,
                            })
                        }
                        _ => None,
                    })
                    .collect();
            }
            // Step physics (if the physics feature is enabled)
            // Physics stepping happens inside slice.step() via sync_physics_with_ecs().
            let _ = (world, dt);

            // Poll gamepad state (gamepad feature only).
            #[cfg(feature = "gamepad")]
            {
                if let Some(gp) = &mut self.gamepad_manager {
                    let _ = gp.update();
                    // Check connected gamepads for button/axis state
                    let gilrs = gp.gilrs();
                    for (_id, gamepad) in gilrs.gamepads() {
                        // Map gamepad buttons to pressed_keys
                        use gilrs::ev::code::*;
                        // Button::South = A/Cross (index 0)
                        // Button::East = B/Circle (index 1)
                        // Button::North = X/Square (index 2)
                        // Button::West = Y/Triangle (index 3)
                        // Button::LeftTrigger = LT (index 4)
                        // Button::RightTrigger = RT (index 5)
                        // Button::LeftBumper = LB (index 6)
                        // Button::RightBumper = RB (index 7)
                        // Button::LeftStick = L3 (index 8)
                        // Button::RightStick = R3 (index 9)
                        // Button::DpadUp = D-pad up (index 10)
                        // Button::DpadDown = D-pad down (index 11)
                        // Button::DpadLeft = D-pad left (index 12)
                        // Button::DpadRight = D-pad right (index 13)
                        // Button::Start = Start (index 14)
                        // Button::Select = Select/Back (index 15)
                        // Button::Home = Home (index 16)
                        for btn_idx in 0..17u8 {
                            if gamepad.is_pressed(Button::from_index(btn_idx)) {
                                pressed_keys.push(format!("GamepadBtn{}", btn_idx));
                            }
                        }
                        // Left stick axis
                        let lx = gamepad.value(Axis::LeftStickX);
                        let ly = gamepad.value(Axis::LeftStickY);
                        if lx.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis0:{:.2}", lx));
                        }
                        if ly.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis1:{:.2}", ly));
                        }
                        // Right stick axis
                        let rx = gamepad.value(Axis::RightStickX);
                        let ry = gamepad.value(Axis::RightStickY);
                        if rx.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis2:{:.2}", rx));
                        }
                        if ry.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis3:{:.2}", ry));
                        }
                        // Left/right triggers
                        let lt = gamepad.value(Axis::LeftTrigger);
                        let rt = gamepad.value(Axis::RightTrigger);
                        if lt.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis4:{:.2}", lt));
                        }
                        if rt.abs() > 0.1 {
                            pressed_keys.push(format!("GamepadAxis5:{:.2}", rt));
                        }
                    }
                }
            }
        });
    }

    fn render(&mut self) {
        // If we're in Playing state but GPU isn't initialized yet, initialize it now.
        if self.menu_state == MenuState::Playing && !self.gpu_initialized {
            // The GPU will be initialized in resumed(), but we need to wait for it.
            // For now, just return and let the next frame handle it.
            return;
        }

        // If in a menu state, render the menu (requires GPU).
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

        // Render UI widgets using UiRenderer (screen-space, top-left origin).
        if !self.ui_widgets.is_empty() {
            let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: gpu.renderer.scene_target().unwrap_or(&view),
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
                    UiWidget::Button { id, bounds, text_key, hovered, pressed } => {
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
                            button.with_normal(pressed_ns.clone())
                        } else if *hovered {
                            button.with_normal(hovered_ns.clone())
                        } else {
                            button.with_normal(normal.clone())
                        };
                        let quads = self.ui_renderer.render_button(&button, *hovered, *pressed);
                        for quad in quads {
                            gpu.hud_sprite_renderer.push_textured(quad.texture, quad.instance);
                        }
                    }
                    UiWidget::Label { id, bounds, text_key } => {
                        let mut label = Label::new(*id, text_key.clone());
                        label.set_bounds(Rect::new(
                            bounds.x,
                            bounds.y,
                            bounds.width,
                            bounds.height,
                        ));
                        let quads = self.ui_renderer.render_label(&label);
                        for instance in quads {
                            gpu.hud_sprite_renderer.push(kiln_render::TextureId::WHITE, instance);
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
                            gpu.hud_sprite_renderer.push_textured(quad.texture, quad.instance);
                        }
                    }
                }
            }

            gpu.hud_sprite_renderer
                .flush(&mut ui_pass, &ui_camera, gpu.renderer.queue());
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
                let title_quads = text.render("Load Game - Select Slot", 16.0, 50.0, 32.0, [1.0, 1.0, 1.0, 1.0]);
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
                    let label = if exists { "Slot {} (saved)" } else { "Slot {} (empty)" };
                    let quads = text.render(&format!("{}{}", prefix, label.replace("{}", &i.to_string())), 16.0, y, 20.0, color);
                    instances.extend(quads);
                }
                let esc_text = text.render("ESC - Back", 16.0, 150.0 + 10.0 * 35.0 + 20.0, 18.0, [0.7, 0.7, 0.7, 1.0]);
                instances.extend(esc_text);
            }
            MenuState::Settings => {
                let title_quads = text.render("Settings", 16.0, 50.0, 32.0, [1.0, 1.0, 1.0, 1.0]);
                instances.extend(title_quads);

                let esc_text = text.render("ESC - Back", 16.0, 150.0, 20.0, [0.7, 0.7, 0.7, 1.0]);
                instances.extend(esc_text);

                let info_text = text.render("Settings UI not yet implemented.", 16.0, 200.0, 20.0, [0.7, 0.7, 0.7, 1.0]);
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
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f64;
                self.mouse_y = position.y as f64;
            }
            winit::event::WindowEvent::Touch(touch) => {
                match touch.phase {
                    winit::event::TouchPhase::Started => {
                        self.mouse_x = touch.location.x as f64;
                        self.mouse_y = touch.location.y as f64;
                        self.mouse_pressed = true;
                    }
                    winit::event::TouchPhase::Moved => {
                        self.mouse_x = touch.location.x as f64;
                        self.mouse_y = touch.location.y as f64;
                    }
                    winit::event::TouchPhase::Ended
                    | winit::event::TouchPhase::Cancelled => {
                        self.mouse_pressed = false;
                    }
                }
            }
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = state.is_pressed();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.update();
        self.render();
    }
}
