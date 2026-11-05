//! Vertical slice runtime: script-driven 2D game logic.
//!
//! A slice runs a KilnScript program once per frame. The script reads *facts*
//! about the current frame (input axes, score, nearby pickups) and emits
//! *commands* (move the player, collect a pickup, change the score). The
//! runtime applies the commands to the world after the script returns, so the
//! script never needs direct access to the ECS world.
//!
//! The same runtime drives the standalone player and the editor, so a game
//! behaves identically in both.

mod authored_ui;

use std::collections::HashMap;
use std::path::Path;

use kiln_input::ActionMap;
use kiln_ir::{Debugger, HostContext, Program, Value, Vm, VmError};

impl HostContext for SliceHost {
    fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
        self.call_host_impl(func_id, args)
    }
}

use kiln_save::slot::SaveSlotError;
use kiln_save::SaveSlotManager;
use kiln_script::{Diagnostic, KilnScriptFrontend, LanguageFrontend};
use serde::{Deserialize, Serialize};

#[cfg(feature = "audio")]
use crate::audio::AudioEngine;

use crate::world::{GameWorld, Role, SourceId};

#[cfg(feature = "physics")]
use crate::physics::PhysicsWorld;

/// The builtin functions a slice script can call.
///
/// Each entry is the function name, its host-call id and its argument count.
pub const BUILTINS: &[(&str, u32, u32)] = &[
    // Core builtins (0-16)
    ("print", 0, 1),
    ("get_input_x", 1, 0),
    ("get_input_y", 2, 0),
    ("get_elapsed", 3, 0),
    ("get_score", 4, 0),
    ("add_score", 5, 1),
    ("get_coin_count", 6, 0),
    ("collect_coin", 7, 0),
    ("move_player", 8, 2),
    ("set_status", 9, 1),
    ("get_status", 10, 0),
    ("get_delta_time", 11, 0),
    ("play_sound", 12, 1),
    ("stop_sound", 13, 1),
    ("play_music", 14, 1),
    ("stop_music", 15, 0),
    ("set_volume", 16, 1),
    // Audio extended (17-25)
    ("set_sfx_volume", 17, 1),   // volume 0-1
    ("set_music_volume", 18, 1), // volume 0-1
    ("pause_sound", 19, 1),      // sound_name -> void
    ("resume_sound", 20, 1),     // sound_name -> void
    ("pause_music", 21, 0),      // -> void
    ("resume_music", 22, 0),     // -> void
    ("stop_all_sounds", 23, 0),  // -> void
    ("sound_playing", 24, 1),    // sound_name -> bool
    ("music_playing", 25, 0),    // -> bool
    // Entity management (26-31)
    ("spawn_entity", 26, 4),   // name, x, y, texture_path?
    ("despawn_entity", 27, 1), // source_id
    ("get_entity_pos", 28, 1), // source_id -> (x, y)
    ("set_entity_pos", 29, 3), // source_id, x, y
    // Query builtins (30-34)
    ("get_entities_with_role", 30, 1), // role -> array of source_ids
    ("get_entity_component", 31, 2),   // source_id, component_name -> value
    ("set_entity_component", 32, 3),   // source_id, component_name, value
    // World queries (33-34)
    ("find_nearest", 33, 3), // x, y, role -> source_id
    ("raycast", 34, 4),      // x, y, dx, dy -> hit info
    // Time/frame (35-36)
    ("get_frame_count", 35, 0), // -> frame number
    ("get_time_scale", 36, 0),  // -> time scale
    // Extended entity API (37-41)
    ("set_entity_vel", 37, 3),           // source_id, vx, vy
    ("get_entities_at_position", 38, 3), // x, y, radius -> array of source_ids
    ("get_entity_role", 39, 1),          // source_id -> role string
    ("add_component", 40, 3),            // source_id, component_type, value_json
    ("remove_component", 41, 2),         // source_id, component_type
    // Animation API (42-44)
    ("play_animation", 42, 2),      // source_id, clip_name
    ("stop_animation", 43, 1),      // source_id
    ("set_animation_speed", 44, 2), // source_id, speed
    // Collision events (45)
    ("get_collision_events", 45, 0), // -> array of [entity_a, entity_b, started]
    // UI widgets (46-49)
    ("ui_button", 46, 3),  // id, x, y -> clicked bool
    ("ui_label", 47, 4),   // id, x, y, text -> void
    ("ui_panel", 48, 5),   // id, x, y, w, h -> void
    ("ui_clicked", 49, 1), // id -> clicked bool
    // Input actions (50-51)
    ("get_action", 50, 1),      // action_name -> pressed bool
    ("get_action_axis", 51, 1), // action_name -> -1 to 1 value
    // Localization (52)
    ("localize", 52, 1), // key -> localized string
    // Save/load (53-54)
    ("save_game", 53, 1), // slot_id -> bool
    ("load_game", 54, 1), // slot_id -> bool
    // Animation clip creation (55)
    ("create_animation_clip", 55, 3), // source_id, clip_name, texture_path -> void
    ("self_entity", 56, 0),
    ("state_get", 57, 2),
    ("state_set", 58, 2),
    ("get_property", 59, 1),
    ("script_phase", 60, 0),
    ("array_length", 61, 1),
    ("array_get", 62, 2),
    ("array_push", 63, 2),
    ("emit_signal", 64, 2),
    ("signal_name", 65, 0),
    ("signal_value", 66, 0),
    ("event_entity", 67, 0),
    ("change_scene", 68, 1),
    ("array_new", 69, 0),
    ("overlap_circle", 70, 4),
    ("raycast_query", 71, 6),
    ("get_mouse_world_x", 72, 0),
    ("get_mouse_world_y", 73, 0),
    ("animation_event_name", 74, 0),
    ("animation_event_value", 75, 0),
    ("set_bus_volume", 76, 2),
    ("get_bus_volume", 77, 1),
    ("setting_get", 78, 2),
    ("setting_set", 79, 2),
    ("ui_show_screen", 80, 1),
    ("ui_close_screen", 81, 0),
    ("ui_screen", 82, 0),
    ("ui_event_name", 83, 0),
    ("ui_event_widget", 84, 0),
    ("ui_set_text", 85, 2),
    ("ui_set_visible", 86, 2),
    ("ui_set_enabled", 87, 2),
    ("ui_event_screen", 88, 0),
];

/// Distance (in world units) at which a pickup counts as collected.
pub const PICKUP_RADIUS: f32 = 24.0;

/// The HUD lines that describe a slice state.
///
/// The standalone player and the editor both draw these, so the two hosts
/// show the same information.
pub fn hud_lines(state: &SliceState) -> Vec<String> {
    let mut lines = vec![format!("SCORE {}", state.score)];
    if !state.status.is_empty() {
        lines.push(state.status.clone());
    }
    lines.push("ARROWS MOVE   F5 SAVE   F9 LOAD".to_string());
    lines
}

/// Role name of the entity the player controls.
pub const ROLE_PLAYER: &str = "player";

/// Role name of the entities the player can collect.
pub const ROLE_COIN: &str = "coin";

/// What the script can read about the current frame.
#[derive(Debug, Clone, Default)]
pub struct SliceFacts {
    /// Horizontal input axis, -1 to 1.
    pub input_x: f64,
    /// Vertical input axis, -1 to 1.
    pub input_y: f64,
    /// Seconds elapsed since the slice started.
    pub elapsed: f64,
    /// Current score.
    pub score: i64,
    /// Number of pickups still on the field.
    pub coin_count: i64,
    /// Source id of the pickup within reach, if any.
    pub coin_in_reach: Option<String>,
    /// The current status message.
    pub status: String,
    /// Length of the current frame, in seconds.
    pub delta_time: f64,
    /// Current frame number.
    pub frame_count: u64,
    /// Current time scale.
    pub time_scale: f64,
    /// Map of entity source_id -> authored local (x, y) position.
    pub entity_positions: HashMap<String, (f64, f64)>,
    /// World-space linear velocity for physics bodies or Velocity components.
    pub entity_velocities: HashMap<String, (f64, f64)>,
    /// Map of entity source_id -> sprite color [r, g, b, a].
    pub entity_sprite_colors: HashMap<String, [f64; 4]>,
    /// Map of entity source_id -> sprite size [w, h].
    pub entity_sprite_sizes: HashMap<String, [f64; 2]>,
    /// Map of entity source_id -> role name.
    pub entity_roles: HashMap<String, String>,
    /// Collision events detected this frame (physics feature only).
    pub collision_events: Vec<(String, String, bool)>,
}

/// A change the script asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum SliceCommand {
    /// Move the player by the given delta.
    MovePlayer { dx: f64, dy: f64 },
    /// Add to the score.
    AddScore(i64),
    /// Collect the pickup with the given source id.
    CollectCoin(String),
    /// Replace the status message.
    SetStatus(String),
    /// Spawn a new entity.
    SpawnEntity {
        name: String,
        x: f64,
        y: f64,
        texture_path: Option<String>,
    },
    /// Despawn an entity by source id.
    DespawnEntity(String),
    /// Set entity position.
    SetEntityPos { source_id: String, x: f64, y: f64 },
    /// Set entity velocity.
    SetEntityVel { source_id: String, vx: f64, vy: f64 },
    /// Set entity component value.
    SetEntityComponent {
        source_id: String,
        component: String,
        value: Value,
    },
    /// Add a component to an entity.
    AddComponent {
        source_id: String,
        component_type: String,
        value: Value,
    },
    /// Remove a component from an entity.
    RemoveComponent {
        source_id: String,
        component_type: String,
    },
    /// Play an animation clip on an entity.
    PlayAnimation {
        source_id: String,
        clip_name: String,
    },
    /// Stop the current animation on an entity.
    StopAnimation { source_id: String },
    /// Set animation playback speed on an entity.
    SetAnimationSpeed { source_id: String, speed: f64 },
    /// Render a UI button at screen position.
    UiButton { id: String, x: f64, y: f64 },
    /// Render a UI label at screen position.
    UiLabel {
        id: String,
        x: f64,
        y: f64,
        text: String,
    },
    /// Render a UI panel at screen position.
    UiPanel {
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    /// Load a save slot.
    LoadGame { slot: u32 },
    /// Create a new animation clip on an entity.
    CreateAnimationClip {
        source_id: String,
        clip_name: String,
        texture_path: String,
    },
    /// Add a frame to an animation clip.
    AddAnimationFrame {
        source_id: String,
        clip_name: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        duration: f64,
    },
    /// Set whether an animation clip loops.
    SetAnimationLoop {
        source_id: String,
        clip_name: String,
        loops: bool,
    },
    /// Create a particle system on an entity.
    CreateParticleSystem { source_id: String },
    /// Add a particle emitter to an entity's particle system.
    AddParticleEmitter {
        source_id: String,
        rate: f64,
        speed_min: f64,
        speed_max: f64,
        angle_min: f64,
        angle_max: f64,
        lifetime_min: f64,
        lifetime_max: f64,
        size_min: f64,
        size_max: f64,
        color: [f64; 4],
        gravity: [f64; 2],
        fade_out: bool,
    },
    /// Start particle system emission.
    StartParticleSystem { source_id: String },
    /// Stop particle system emission.
    StopParticleSystem { source_id: String },
}

/// The persistent state of a slice.
///
/// This is what gets written to a save slot.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SliceState {
    /// Current score.
    pub score: i64,
    /// Seconds the slice has been running.
    pub elapsed: f64,
    /// The status message shown in the HUD.
    pub status: String,
    /// Source ids of the pickups already collected.
    pub collected: Vec<String>,
    /// Player position on the x axis.
    pub player_x: f64,
    /// Player position on the y axis.
    pub player_y: f64,
}

impl SliceState {
    /// Whether the pickup with this source id has been collected.
    pub fn is_collected(&self, source_id: &str) -> bool {
        self.collected.iter().any(|id| id == source_id)
    }
}

/// Write the state to a save slot in `dir`.
pub fn save_state(dir: &Path, slot: u32, state: &SliceState) -> Result<(), SaveSlotError> {
    let json = serde_json::to_string(state)?;
    let manager = SaveSlotManager::with_directory(dir.to_path_buf());
    manager.save(slot, &json, "vertical slice")
}

/// Read the state from a save slot in `dir`.
pub fn load_state(dir: &Path, slot: u32) -> Result<SliceState, SaveSlotError> {
    let manager = SaveSlotManager::with_directory(dir.to_path_buf());
    let saved = manager.load(slot)?;
    Ok(serde_json::from_str(&saved.data)?)
}

/// Platform-provided sound playback. Browser callbacks may capture JS handles,
/// while native callbacks preserve the host's Send/Sync guarantees.
#[cfg(target_arch = "wasm32")]
pub type AudioPlayer = Box<dyn Fn(&str)>;
#[cfg(not(target_arch = "wasm32"))]
pub type AudioPlayer = Box<dyn Fn(&str) + Send + Sync>;

/// The script host: reads facts and records commands.
#[derive(Default)]
pub struct SliceHost {
    animation_event: (String, String),
    ui: kiln_ui::authored::UiRuntime,
    ui_event: Option<kiln_ui::authored::UiEvent>,
    ui_pointer: Option<[f32; 2]>,
    ui_pointer_down: bool,
    ui_size: [f32; 2],
    ui_key_edges: Vec<String>,
    ui_pointer_edges: Vec<(Option<[f32; 2]>, bool)>,
    mouse_world: [f64; 2],
    self_entity: String,
    script_state: HashMap<String, Value>,
    properties: HashMap<String, f64>,
    phase: String,
    signal_name: String,
    signal_value: Option<Value>,
    emitted_signals: Vec<(String, Value)>,
    event_entity: String,
    scene_request: Option<String>,
    facts: SliceFacts,
    commands: Vec<SliceCommand>,
    #[cfg(feature = "audio")]
    audio: Option<AudioEngine>,
    #[cfg(feature = "physics")]
    physics: Option<PhysicsWorld>,
    /// UI widget state: widget id -> whether it was clicked this frame.
    ui_widgets: HashMap<String, bool>,
    /// UI button rects collected this frame for click detection.
    ui_buttons: Vec<(String, f64, f64, f64, f64)>,
    /// Input action map for get_action/get_action_axis builtins.
    action_map: kiln_input::ActionMap,
    /// Localization data for translating string keys.
    localization: Option<kiln_ui::Localization>,
    /// Directory where save slots are written.
    save_dir: Option<std::path::PathBuf>,
    preferences: crate::preferences::PlayerPreferences,
    preference_fault: Option<String>,
    /// Current persistent state (synced from SliceSession each frame).
    state: SliceState,
    /// Current mouse position (screen space, top-left origin).
    mouse_x: f64,
    /// Current mouse position (screen space, top-left origin).
    mouse_y: f64,
    /// Whether the left mouse button was pressed this frame.
    mouse_pressed: bool,
    /// Currently pressed keys (winit key names as strings).
    pressed_keys: std::collections::HashSet<String>,
    /// Frame counter for get_frame_count.
    frame_count: u64,
    /// Time scale for get_time_scale.
    time_scale: f64,
    /// Audio files registered for playback via script.
    #[cfg(feature = "audio")]
    registered_audio: HashMap<String, String>,
    /// Custom audio player callback for platforms without rodio (e.g., WASM).
    audio_player: Option<AudioPlayer>,
}

impl std::fmt::Debug for SliceHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliceHost")
            .field("facts", &self.facts)
            .field("commands", &self.commands)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl SliceHost {
    fn normalized_key(key: &str) -> String {
        let candidate = key.strip_prefix("Key").unwrap_or(key);
        if candidate.len() == 1 && candidate.as_bytes()[0].is_ascii_alphabetic() {
            candidate.to_ascii_lowercase()
        } else {
            key.to_owned()
        }
    }
    /// A host with the given facts.
    pub fn new(facts: SliceFacts) -> Self {
        Self {
            animation_event: Default::default(),
            ui: Default::default(),
            ui_event: None,
            ui_pointer: None,
            ui_pointer_down: false,
            ui_size: [0.0; 2],
            ui_key_edges: Vec::new(),
            ui_pointer_edges: Vec::new(),
            mouse_world: [0.0; 2],
            self_entity: String::new(),
            script_state: HashMap::new(),
            properties: HashMap::new(),
            phase: String::new(),
            signal_name: String::new(),
            signal_value: None,
            emitted_signals: Vec::new(),
            event_entity: String::new(),
            scene_request: None,
            facts,
            commands: Vec::new(),
            #[cfg(feature = "audio")]
            audio: None,
            #[cfg(feature = "physics")]
            physics: Some(PhysicsWorld::new()),
            ui_widgets: HashMap::new(),
            ui_buttons: Vec::new(),
            action_map: kiln_input::ActionMap::new(),
            localization: None,
            save_dir: None,
            preferences: Default::default(),
            preference_fault: None,
            state: SliceState::default(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_pressed: false,
            pressed_keys: std::collections::HashSet::new(),
            frame_count: 0,
            time_scale: 1.0,
            #[cfg(feature = "audio")]
            registered_audio: HashMap::new(),
            audio_player: None,
        }
    }

    /// The facts of the current frame.
    pub fn facts(&self) -> &SliceFacts {
        &self.facts
    }

    /// The commands recorded during the last script run.
    pub fn commands(&self) -> &[SliceCommand] {
        &self.commands
    }

    /// Take the recorded commands, leaving the host empty.
    pub fn take_commands(&mut self) -> Vec<SliceCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Set the current mouse position and button state (called by the host before each step).
    pub fn set_mouse_state(&mut self, x: f64, y: f64, pressed: bool) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_pressed = pressed;
    }

    /// Set the pointer position mapped through the active gameplay camera.
    pub fn set_mouse_world(&mut self, point: [f64; 2]) {
        self.mouse_world = point;
    }

    /// Whether a UI widget was clicked this frame.
    pub fn ui_widget_clicked(&self, id: &str) -> bool {
        self.ui_widgets.get(id).copied().unwrap_or(false) || self.ui.clicked(id)
    }

    /// Set the input action map (called by the host before each step).
    pub fn set_action_map(&mut self, action_map: kiln_input::ActionMap) {
        self.action_map = action_map;
    }

    /// Set the localization data (called by the host before each step).
    pub fn set_localization(&mut self, localization: kiln_ui::Localization) {
        self.localization = Some(localization);
    }

    /// Set the save directory (called by the host before each step).
    pub fn set_save_dir(&mut self, dir: &std::path::Path) {
        if self.save_dir.as_deref() == Some(dir) {
            return;
        }
        self.save_dir = Some(dir.to_path_buf());
        match crate::preferences::PlayerPreferences::load(dir) {
            Ok(preferences) => {
                #[cfg(feature = "audio")]
                if let (Some(audio), Some(settings)) = (&mut self.audio, &preferences.audio) {
                    audio.set_settings(settings.clone());
                }
                self.preferences = preferences;
                self.preference_fault = None;
            }
            Err(error) => {
                log::error!("Could not load player preferences: {error}");
                self.preference_fault = Some(error);
            }
        }
    }

    /// Configure project assets once from disk or packed resource bytes.
    /// Decode failures are reported individually; valid clips remain usable.
    #[cfg(feature = "audio")]
    pub fn configure_audio(
        &mut self,
        project: &kiln_core::Project,
        mut resource: impl FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Vec<String> {
        let mut audio = AudioEngine::default();
        audio.set_settings(
            self.preferences
                .audio
                .as_ref()
                .unwrap_or(&project.audio_mixer)
                .clone(),
        );
        let mut errors = Vec::new();
        let mut names = std::collections::HashSet::new();
        for asset in &project.audio_assets {
            if !names.insert(&asset.name) {
                errors.push(format!("Duplicate audio name: {}", asset.name));
                continue;
            }
            if let Err(error) =
                resource(&asset.path).and_then(|bytes| audio.register_bytes(asset, bytes))
            {
                errors.push(format!("Audio '{}': {error}", asset.name));
            }
        }
        self.audio = Some(audio);
        errors
    }

    /// Open a native device only when the editor/player actually starts playback.
    #[cfg(feature = "audio")]
    pub fn open_audio_output(&mut self) -> Result<(), String> {
        self.audio
            .get_or_insert_with(AudioEngine::default)
            .open_output()
    }

    /// Inspect or offline-render the same audio used by gameplay.
    #[cfg(feature = "audio")]
    pub fn audio_mut(&mut self) -> Option<&mut AudioEngine> {
        self.audio.as_mut()
    }

    /// Last preferences failure; a failed load blocks writes to preserve that file.
    pub fn preferences_error(&self) -> Option<&str> {
        self.preference_fault.as_deref()
    }

    /// Read an arbitrary game-specific player setting.
    pub fn player_setting(&self, key: &str) -> Option<Value> {
        self.preferences
            .values
            .get(key)
            .and_then(crate::preferences::decode_value)
    }

    fn write_preferences(
        &mut self,
        candidate: crate::preferences::PlayerPreferences,
    ) -> Result<(), VmError> {
        if let Some(error) = &self.preference_fault {
            return Err(VmError::HostCallError(format!(
                "Preferences unchanged: {error}"
            )));
        }
        let dir = self.save_dir.as_ref().ok_or_else(|| {
            VmError::HostCallError("Player settings directory is not configured".into())
        })?;
        candidate.save(dir).map_err(|error| {
            VmError::HostCallError(format!("Could not save preferences: {error}"))
        })?;
        self.preferences = candidate;
        Ok(())
    }

    /// Persist a player setting immediately. Failure keeps the previous value.
    pub fn set_player_setting(&mut self, key: &str, value: Value) -> Result<(), VmError> {
        if key.is_empty()
            || key.len() > 256
            || (self.preferences.values.len() >= 4096 && !self.preferences.values.contains_key(key))
        {
            return Err(VmError::HostCallError(
                "Player setting limit reached".into(),
            ));
        }
        let json = crate::preferences::encode_value(&value).map_err(VmError::HostCallError)?;
        let mut candidate = self.preferences.clone();
        candidate.values.insert(key.into(), json);
        if candidate == self.preferences && self.preference_fault.is_none() {
            return Ok(());
        }
        self.write_preferences(candidate)
    }

    #[cfg(feature = "audio")]
    fn persist_audio_settings(&mut self) -> Result<(), VmError> {
        if self.save_dir.is_none() {
            return Ok(());
        }
        let mut candidate = self.preferences.clone();
        if let Some(audio) = &self.audio {
            candidate.audio = Some(audio.settings().clone());
        }
        if candidate == self.preferences && self.preference_fault.is_none() {
            return Ok(());
        }
        self.write_preferences(candidate)
    }

    /// Legacy one-time file registration. Prefer configure_audio for authored assets.
    #[cfg(feature = "audio")]
    pub fn register_audio(&mut self, name: &str, path: &str) {
        if self
            .registered_audio
            .get(name)
            .is_some_and(|old| old == path)
        {
            return;
        }
        let audio = self.audio.get_or_insert_with(AudioEngine::default);
        match audio.register_sound(name, crate::audio::AudioCategory::Sfx, path) {
            Ok(()) => {
                self.registered_audio.insert(name.into(), path.into());
            }
            Err(error) => log::error!("Audio '{name}': {error}"),
        }
    }

    /// Set a custom audio player callback for platforms without rodio (e.g., WASM).
    pub fn set_audio_player(&mut self, player: AudioPlayer) {
        self.audio_player = Some(player);
    }

    /// Play a registered audio file by name (used for editor preview).
    #[cfg(feature = "audio")]
    pub fn play_audio_preview(&mut self, name: &str) {
        if let Some(audio) = &mut self.audio {
            audio.play_sound(name);
        }
    }

    /// Stop all audio playback (used when stopping preview).
    #[cfg(feature = "audio")]
    pub fn stop_audio_preview(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.stop_all();
        }
    }

    /// Set the currently pressed keys (called by the host before each step).
    pub fn set_pressed_keys(&mut self, keys: &[String]) {
        self.pressed_keys = keys.iter().map(|key| Self::normalized_key(key)).collect();
    }

    /// Check if an action is currently pressed by checking its key bindings.
    fn is_action_pressed(&self, name: &str) -> bool {
        let Some(action) = self.action_map.get_action(name) else {
            return false;
        };
        action
            .bindings
            .iter()
            .any(|binding| match &binding.binding_type {
                kiln_input::BindingType::Key(key) => {
                    self.pressed_keys.contains(&Self::normalized_key(key))
                }
                kiln_input::BindingType::GamepadButton(btn) => {
                    self.pressed_keys.contains(&format!("GamepadBtn{}", btn))
                }
                kiln_input::BindingType::GamepadAxis { .. } => false,
            })
    }

    /// Get the axis value for an action (-1 to 1).
    fn get_action_axis(&self, name: &str) -> f64 {
        let Some(action) = self.action_map.get_action(name) else {
            return 0.0;
        };
        // Simple axis: check if any binding maps to a negative/positive direction.
        let mut value = 0.0;
        for binding in &action.bindings {
            match &binding.binding_type {
                kiln_input::BindingType::Key(key) => {
                    if self.pressed_keys.contains(&Self::normalized_key(key)) {
                        // Map common keys to axis directions
                        match key.as_str() {
                            "KeyA" | "ArrowLeft" | "a" | "A" => value -= 1.0,
                            "KeyD" | "ArrowRight" | "d" | "D" => value += 1.0,
                            "KeyS" | "ArrowDown" | "s" | "S" => value -= 1.0,
                            "KeyW" | "ArrowUp" | "w" | "W" => value += 1.0,
                            _ => {}
                        }
                    }
                }
                kiln_input::BindingType::GamepadAxis { axis, threshold } => {
                    // Gamepad axis values are tracked in pressed_keys as
                    // "GamepadAxis{axis}:{value}" when the absolute value
                    // exceeds the threshold.
                    let key = format!("GamepadAxis{}", axis);
                    if let Some(val_str) = self.pressed_keys.iter().find(|k| k.starts_with(&key)) {
                        if let Some(val) = val_str
                            .strip_prefix(&format!("GamepadAxis{}:", axis))
                            .and_then(|v| v.parse::<f64>().ok())
                        {
                            if val.abs() >= *threshold as f64 {
                                value += val.signum();
                            }
                        }
                    }
                }
                kiln_input::BindingType::GamepadButton(_) => {
                    // Gamepad buttons don't contribute to axis values
                }
            }
        }
        value.clamp(-1.0, 1.0)
    }

    fn call_host_impl(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
        if func_id >= 56 {
            if let Some((name, _, argc)) = BUILTINS.iter().find(|(_, id, _)| *id == func_id) {
                if args.len() != *argc as usize {
                    return Err(VmError::HostCallError(format!(
                        "{name} expects {argc} arguments"
                    )));
                }
            }
        }
        match func_id {
            // Core builtins (0-16)
            0 => {
                log::info!(
                    "[script] {}",
                    args.first().map(describe).unwrap_or_default()
                );
                Ok(Value::I64(0))
            }
            1 => Ok(Value::F64(self.facts.input_x)),
            2 => Ok(Value::F64(self.facts.input_y)),
            3 => Ok(Value::F64(self.facts.elapsed)),
            4 => Ok(Value::I64(self.facts.score)),
            5 => {
                let amount = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                self.commands.push(SliceCommand::AddScore(amount));
                Ok(Value::I64(self.facts.score + amount))
            }
            6 => Ok(Value::I64(self.facts.coin_count)),
            7 => match self.facts.coin_in_reach.clone() {
                Some(id) => {
                    self.commands.push(SliceCommand::CollectCoin(id));
                    Ok(Value::Bool(true))
                }
                None => Ok(Value::Bool(false)),
            },
            8 => {
                let dx = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let dy = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                self.commands.push(SliceCommand::MovePlayer { dx, dy });
                Ok(Value::I64(0))
            }
            9 => {
                let text = args.first().and_then(|v| v.as_str()).unwrap_or("");
                self.commands
                    .push(SliceCommand::SetStatus(text.to_string()));
                Ok(Value::I64(0))
            }
            10 => Ok(Value::Str(self.facts.status.clone())),
            11 => Ok(Value::F64(self.facts.delta_time)),
            12 => {
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some(player) = &self.audio_player {
                    player(name);
                } else {
                    #[cfg(feature = "audio")]
                    if let Some(audio) = &mut self.audio {
                        audio.play_sound(name);
                    }
                }
                Ok(Value::I64(0))
            }
            13 => {
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(audio) = &mut self.audio {
                        audio.stop_sound(name);
                    }
                }
                Ok(Value::I64(0))
            }
            14 => {
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(audio) = &mut self.audio {
                        audio.play_music(name);
                    }
                }
                Ok(Value::I64(0))
            }
            15 => {
                #[cfg(feature = "audio")]
                {
                    if let Some(audio) = &mut self.audio {
                        audio.stop_music();
                    }
                }
                Ok(Value::I64(0))
            }
            16 => {
                #[cfg(feature = "audio")]
                {
                    let volume = args.first().and_then(|v| v.to_f64()).unwrap_or(1.0) as f32;
                    if let Some(audio) = &mut self.audio {
                        audio.set_master_volume(volume);
                    }
                    self.persist_audio_settings()?;
                }
                Ok(Value::I64(0))
            }
            17 => {
                // set_sfx_volume(volume) -> void
                #[cfg(feature = "audio")]
                {
                    let volume = args.first().and_then(|v| v.to_f64()).unwrap_or(1.0) as f32;
                    if let Some(audio) = &mut self.audio {
                        audio.set_sfx_volume(volume);
                    }
                    self.persist_audio_settings()?;
                }
                Ok(Value::I64(0))
            }
            18 => {
                // set_music_volume(volume) -> void
                #[cfg(feature = "audio")]
                {
                    let volume = args.first().and_then(|v| v.to_f64()).unwrap_or(1.0) as f32;
                    if let Some(audio) = &mut self.audio {
                        audio.set_music_volume(volume);
                    }
                    self.persist_audio_settings()?;
                }
                Ok(Value::I64(0))
            }
            19 => {
                // pause_sound(name) -> void
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(audio) = &mut self.audio {
                        audio.pause_sound(name);
                    }
                }
                Ok(Value::I64(0))
            }
            20 => {
                // resume_sound(name) -> void
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(audio) = &mut self.audio {
                        audio.resume_sound(name);
                    }
                }
                Ok(Value::I64(0))
            }
            21 => {
                // pause_music() -> void
                #[cfg(feature = "audio")]
                {
                    if let Some(audio) = &mut self.audio {
                        audio.pause_music();
                    }
                }
                Ok(Value::I64(0))
            }
            22 => {
                // resume_music() -> void
                #[cfg(feature = "audio")]
                {
                    if let Some(audio) = &mut self.audio {
                        audio.resume_music();
                    }
                }
                Ok(Value::I64(0))
            }
            23 => {
                // stop_all_sounds() -> void
                #[cfg(feature = "audio")]
                {
                    if let Some(audio) = &mut self.audio {
                        audio.stop_all();
                    }
                }
                Ok(Value::I64(0))
            }
            24 => {
                // sound_playing(name) -> bool
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    let playing = self
                        .audio
                        .as_ref()
                        .map(|a| a.is_playing(name))
                        .unwrap_or(false);
                    Ok(Value::Bool(playing))
                }
                #[cfg(not(feature = "audio"))]
                Ok(Value::Bool(false))
            }
            25 => {
                // music_playing() -> bool
                #[cfg(feature = "audio")]
                {
                    let playing = self
                        .audio
                        .as_ref()
                        .map(|a| a.is_music_playing())
                        .unwrap_or(false);
                    Ok(Value::Bool(playing))
                }
                #[cfg(not(feature = "audio"))]
                Ok(Value::Bool(false))
            }
            // Entity management (26-31)
            26 => {
                // spawn_entity(name, x, y, texture_path?)
                let name = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("entity")
                    .to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let texture_path = args.get(3).and_then(|v| v.as_str()).map(|s| s.to_string());
                self.commands.push(SliceCommand::SpawnEntity {
                    name,
                    x,
                    y,
                    texture_path,
                });
                Ok(Value::I64(0))
            }
            27 => {
                // despawn_entity(source_id)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands.push(SliceCommand::DespawnEntity(source_id));
                Ok(Value::I64(0))
            }
            28 => {
                // get_entity_pos(source_id) -> (x, y) or null
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some((x, y)) = self.facts.entity_positions.get(source_id) {
                    Ok(Value::Array(vec![Value::F64(*x), Value::F64(*y)]))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            29 => {
                // set_entity_pos(source_id, x, y)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                self.commands
                    .push(SliceCommand::SetEntityPos { source_id, x, y });
                Ok(Value::I64(0))
            }
            // Query builtins (30-34)
            30 => {
                // get_entities_with_role(role) -> array of source_ids
                let role = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let entities: Vec<Value> = self
                    .facts
                    .entity_roles
                    .iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(id, _)| Value::Str(id.clone()))
                    .collect();
                Ok(Value::Array(entities))
            }
            31 => {
                // get_entity_component(source_id, component_name) -> value
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let component = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
                match component {
                    "velocity" | "Velocity" => Ok(self
                        .facts
                        .entity_velocities
                        .get(source_id)
                        .map(|(x, y)| Value::Array(vec![Value::F64(*x), Value::F64(*y)]))
                        .unwrap_or(Value::Bool(false))),
                    "transform" | "Transform" | "position" | "Position" => {
                        if let Some((x, y)) = self.facts.entity_positions.get(source_id) {
                            Ok(Value::Array(vec![Value::F64(*x), Value::F64(*y)]))
                        } else {
                            Ok(Value::Bool(false))
                        }
                    }
                    "sprite_color" | "SpriteColor" | "color" | "Color" => {
                        if let Some(color) = self.facts.entity_sprite_colors.get(source_id) {
                            Ok(Value::Array(color.iter().map(|&c| Value::F64(c)).collect()))
                        } else {
                            Ok(Value::Bool(false))
                        }
                    }
                    "sprite_size" | "SpriteSize" | "size" | "Size" => {
                        if let Some(size) = self.facts.entity_sprite_sizes.get(source_id) {
                            Ok(Value::Array(size.iter().map(|&s| Value::F64(s)).collect()))
                        } else {
                            Ok(Value::Bool(false))
                        }
                    }
                    _ => Ok(Value::Bool(false)),
                }
            }
            32 => {
                // set_entity_component(source_id, component_name, value)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let component = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = args.get(2).cloned().unwrap_or(Value::I64(0));
                self.commands.push(SliceCommand::SetEntityComponent {
                    source_id,
                    component,
                    value,
                });
                Ok(Value::I64(0))
            }
            // World queries (33-34)
            33 => {
                // find_nearest(x, y, role) -> source_id or null
                let x = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let role = args.get(2).and_then(|v| v.as_str()).unwrap_or("");

                let mut nearest: Option<(String, f64)> = None;
                for (id, (ex, ey)) in &self.facts.entity_positions {
                    if self.facts.entity_roles.get(id) == Some(&role.to_string()) {
                        let dist = ((ex - x).powi(2) + (ey - y).powi(2)).sqrt();
                        if nearest.is_none() || dist < nearest.as_ref().unwrap().1 {
                            nearest = Some((id.clone(), dist));
                        }
                    }
                }
                if let Some((id, _)) = nearest {
                    Ok(Value::Str(id))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            34 => {
                // raycast(x, y, dx, dy) -> [source_id, distance] or false
                let x = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let dx = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let dy = args.get(3).and_then(|v| v.to_f64()).unwrap_or(0.0);
                #[cfg(feature = "physics")]
                {
                    if let Some(physics) = &self.physics {
                        if let Some(hit) = physics.raycast_hit(
                            [x as f32, y as f32],
                            [dx as f32, dy as f32],
                            100.0,
                            u16::MAX,
                            Some(&self.self_entity),
                            false,
                        ) {
                            return Ok(Value::Array(vec![
                                Value::Str(hit.source_id),
                                Value::F64(hit.distance as f64),
                                Value::F64(hit.normal[0] as f64),
                                Value::F64(hit.normal[1] as f64),
                            ]));
                        }
                    }
                }
                #[cfg(not(feature = "physics"))]
                let _ = (x, y, dx, dy);
                Ok(Value::Bool(false))
            }
            // Time/frame (35-36)
            35 => Ok(Value::I64(self.frame_count as i64)),
            36 => Ok(Value::F64(self.time_scale)),
            // Extended entity API (37-40)
            37 => {
                // set_entity_vel(source_id, vx, vy)
                #[cfg(feature = "physics")]
                {
                    let source_id = args
                        .first()
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let vx = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                    let vy = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                    self.commands
                        .push(SliceCommand::SetEntityVel { source_id, vx, vy });
                }
                Ok(Value::I64(0))
            }
            38 => {
                // get_entities_at_position(x, y, radius) -> array of source_ids
                let x = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let radius = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let mut found: Vec<Value> = Vec::new();
                for (source_id, (ex, ey)) in &self.facts.entity_positions {
                    let dx = ex - x;
                    let dy = ey - y;
                    if (dx * dx + dy * dy).sqrt() <= radius {
                        found.push(Value::Str(source_id.clone()));
                    }
                }
                Ok(Value::Array(found))
            }
            39 => {
                // get_entity_role(source_id) -> role string
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some(role) = self.facts.entity_roles.get(source_id) {
                    Ok(Value::Str(role.clone()))
                } else {
                    Ok(Value::Str(String::new()))
                }
            }
            40 => {
                // add_component(source_id, component_type, value_json)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let component_type = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = args.get(2).cloned().unwrap_or(Value::I64(0));
                self.commands.push(SliceCommand::AddComponent {
                    source_id,
                    component_type,
                    value,
                });
                Ok(Value::I64(0))
            }
            41 => {
                // remove_component(source_id, component_type)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let component_type = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands.push(SliceCommand::RemoveComponent {
                    source_id,
                    component_type,
                });
                Ok(Value::I64(0))
            }
            42 => {
                // play_animation(source_id, clip_name)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let clip_name = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands.push(SliceCommand::PlayAnimation {
                    source_id,
                    clip_name,
                });
                Ok(Value::I64(0))
            }
            43 => {
                // stop_animation(source_id)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands
                    .push(SliceCommand::StopAnimation { source_id });
                Ok(Value::I64(0))
            }
            44 => {
                // set_animation_speed(source_id, speed)
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let speed = args.get(1).and_then(|v| v.to_f64()).unwrap_or(1.0);
                self.commands
                    .push(SliceCommand::SetAnimationSpeed { source_id, speed });
                Ok(Value::I64(0))
            }
            45 => {
                // get_collision_events() -> array of [entity_a, entity_b, started]
                let events: Vec<Value> = self
                    .facts
                    .collision_events
                    .iter()
                    .map(|(a, b, started)| {
                        Value::Array(vec![
                            Value::Str(a.clone()),
                            Value::Str(b.clone()),
                            Value::Bool(*started),
                        ])
                    })
                    .collect();
                Ok(Value::Array(events))
            }
            // UI widgets (46-49)
            46 => {
                // ui_button(id, x, y) -> clicked bool
                let id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                // Button is 64x32 screen pixels (matching the player renderer).
                self.ui_buttons.push((id.clone(), x, y, 64.0, 32.0));
                self.commands.push(SliceCommand::UiButton {
                    id: id.clone(),
                    x,
                    y,
                });
                let clicked = self.ui_widget_clicked(&id);
                Ok(Value::Bool(clicked))
            }
            47 => {
                // ui_label(id, x, y, text) -> void
                let id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let text = args
                    .get(3)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands.push(SliceCommand::UiLabel { id, x, y, text });
                Ok(Value::I64(0))
            }
            48 => {
                // ui_panel(id, x, y, w, h) -> void
                let id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let w = args.get(3).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let h = args.get(4).and_then(|v| v.to_f64()).unwrap_or(0.0);
                self.commands.push(SliceCommand::UiPanel { id, x, y, w, h });
                Ok(Value::I64(0))
            }
            49 => {
                // ui_clicked(id) -> clicked bool
                let id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let clicked = self.ui_widget_clicked(&id);
                Ok(Value::Bool(clicked))
            }
            // Input actions (50-51)
            50 => {
                // get_action(action_name) -> pressed bool
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let pressed = self.is_action_pressed(name);
                Ok(Value::Bool(pressed))
            }
            51 => {
                // get_action_axis(action_name) -> -1 to 1 value
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let axis = self.get_action_axis(name);
                Ok(Value::F64(axis))
            }
            // Localization (52)
            52 => {
                // localize(key) -> localized string
                let key = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some(loc) = &self.localization {
                    let loc_key = kiln_ui::LocalizationKey::new(key);
                    if let Some(value) = loc.get(&loc_key) {
                        Ok(Value::Str(value.clone()))
                    } else {
                        Ok(Value::Str(key.to_string()))
                    }
                } else {
                    Ok(Value::Str(key.to_string()))
                }
            }
            // Save/load (53-54)
            53 => {
                // save_game(slot_id) -> bool
                let slot = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                let Some(dir) = &self.save_dir else {
                    return Ok(Value::Bool(false));
                };
                match save_state(dir, slot, &self.state) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            54 => {
                // load_game(slot_id) -> bool
                let slot = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                self.commands.push(SliceCommand::LoadGame { slot });
                Ok(Value::Bool(true))
            }
            // Animation clip creation (55)
            55 => {
                // create_animation_clip(source_id, clip_name, texture_path) -> void
                let source_id = args
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let clip_name = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let texture_path = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.commands.push(SliceCommand::CreateAnimationClip {
                    source_id,
                    clip_name,
                    texture_path,
                });
                Ok(Value::I64(0))
            }
            56 => Ok(Value::Str(self.self_entity.clone())),
            57 => {
                let name = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("state_get expects a string key".into())
                })?;
                Ok(self
                    .script_state
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| args[1].clone()))
            }
            58 => {
                let name = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("state_set expects a string key".into())
                })?;
                if name.len() > 256
                    || (self.script_state.len() >= 4096 && !self.script_state.contains_key(name))
                {
                    return Err(VmError::HostCallError("Script state limit reached".into()));
                }
                self.script_state.insert(name.to_owned(), args[1].clone());
                Ok(args[1].clone())
            }
            59 => {
                let name = args[0].as_str().unwrap_or_default();
                Ok(Value::F64(*self.properties.get(name).unwrap_or(&0.0)))
            }
            60 => Ok(Value::Str(
                if self.phase.is_empty() {
                    "update"
                } else {
                    &self.phase
                }
                .into(),
            )),
            61 => match &args[0] {
                Value::Array(values) => Ok(Value::I64(values.len() as i64)),
                _ => Err(VmError::HostCallError(
                    "array_length expects an array".into(),
                )),
            },
            62 => match (&args[0], args[1].as_i64()) {
                (Value::Array(values), Some(index)) if index >= 0 => values
                    .get(index as usize)
                    .cloned()
                    .ok_or_else(|| VmError::HostCallError("array index is out of range".into())),
                _ => Err(VmError::HostCallError(
                    "array_get expects an array and nonnegative integer".into(),
                )),
            },
            63 => match &args[0] {
                Value::Array(values) if values.len() < 4096 => {
                    let mut values = values.clone();
                    values.push(args[1].clone());
                    Ok(Value::Array(values))
                }
                _ => Err(VmError::HostCallError(
                    "array_push expects an array with fewer than 4096 items".into(),
                )),
            },
            64 => {
                let name = args[0]
                    .as_str()
                    .ok_or_else(|| VmError::HostCallError("signal name must be a string".into()))?;
                if self.emitted_signals.len() >= 256 {
                    return Err(VmError::HostCallError("signal queue limit reached".into()));
                }
                self.emitted_signals.push((name.into(), args[1].clone()));
                Ok(Value::I64(0))
            }
            65 => Ok(Value::Str(self.signal_name.clone())),
            66 => Ok(self.signal_value.clone().unwrap_or(Value::I64(0))),
            67 => Ok(Value::Str(self.event_entity.clone())),
            68 => {
                let id = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("change_scene expects a scene ID".into())
                })?;
                self.scene_request = Some(id.into());
                Ok(Value::I64(0))
            }
            69 => Ok(Value::Array(Vec::new())),
            70 => {
                #[cfg(feature = "physics")]
                if let Some(physics) = &self.physics {
                    let x = args[0].to_f64().unwrap_or(0.0) as f32;
                    let y = args[1].to_f64().unwrap_or(0.0) as f32;
                    let radius = args[2].to_f64().unwrap_or(0.0) as f32;
                    let mask = args[3].as_i64().unwrap_or(65535) as u16;
                    return Ok(Value::Array(
                        physics
                            .overlap_circle([x, y], radius, mask)
                            .into_iter()
                            .map(Value::Str)
                            .collect(),
                    ));
                }
                Ok(Value::Array(Vec::new()))
            }
            71 => {
                #[cfg(feature = "physics")]
                if let Some(physics) = &self.physics {
                    let values: Vec<_> = args
                        .iter()
                        .take(5)
                        .map(|value| value.to_f64().unwrap_or(0.0) as f32)
                        .collect();
                    if let Some(hit) = physics.raycast_hit(
                        [values[0], values[1]],
                        [values[2], values[3]],
                        values[4],
                        args[5].as_i64().unwrap_or(65535) as u16,
                        Some(&self.self_entity),
                        false,
                    ) {
                        return Ok(Value::Array(vec![
                            Value::Str(hit.source_id),
                            Value::F64(hit.distance as f64),
                            Value::F64(hit.normal[0] as f64),
                            Value::F64(hit.normal[1] as f64),
                        ]));
                    }
                }
                Ok(Value::Bool(false))
            }
            72 => Ok(Value::F64(self.mouse_world[0])),
            73 => Ok(Value::F64(self.mouse_world[1])),
            74 => Ok(Value::Str(self.animation_event.0.clone())),
            75 => Ok(Value::Str(self.animation_event.1.clone())),
            76 => {
                #[cfg(feature = "audio")]
                {
                    let name = args[0]
                        .as_str()
                        .ok_or_else(|| VmError::HostCallError("Expected a bus name".into()))?;
                    let volume = args[1]
                        .to_f64()
                        .ok_or_else(|| VmError::HostCallError("Expected a volume".into()))?
                        as f32;
                    if let Some(audio) = &mut self.audio {
                        audio.set_bus_volume(name, volume);
                    }
                    self.persist_audio_settings()?;
                }
                Ok(Value::I64(0))
            }
            77 => {
                #[cfg(feature = "audio")]
                if let Some(audio) = &self.audio {
                    let name = args[0].as_str().unwrap_or_default();
                    let volume = if name == "Master" {
                        audio.master_volume()
                    } else {
                        audio
                            .settings()
                            .buses
                            .get(name)
                            .map_or(1.0, |bus| kiln_core::audio::gain(bus.volume))
                    };
                    return Ok(Value::F64(volume as f64));
                }
                Ok(Value::F64(1.0))
            }
            78 => {
                let key = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("setting_get expects a string key".into())
                })?;
                Ok(self.player_setting(key).unwrap_or_else(|| args[1].clone()))
            }
            79 => {
                let key = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("setting_set expects a string key".into())
                })?;
                self.set_player_setting(key, args[1].clone())?;
                Ok(args[1].clone())
            }
            80..=88 => self.call_ui(func_id, args),
            _ => Err(VmError::HostCallError(format!(
                "unknown slice builtin id {func_id}"
            ))),
        }
    }
}

/// Render a value for the log.
fn describe(value: &Value) -> String {
    match value {
        Value::I64(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Str(v) => v.clone(),
        Value::Array(arr) => format!(
            "[{}]",
            arr.iter().map(describe).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// A running slice: the compiled script plus its state.
pub struct SliceSession {
    source: Option<crate::script_source::ExpandedSource>,
    external_collisions: Vec<(String, String, bool)>,
    attached: Vec<AttachedScript>,
    signals: Vec<(String, Value)>,
    program: Program,
    vm: Vm<SliceHost>,
    debugger: Option<Debugger<SliceHost>>,
    state: SliceState,
    /// UI commands recorded during the last script run (buttons, labels, panels).
    ui_commands: Vec<SliceCommand>,
}

struct AttachedScript {
    frame: u64,
    entity_id: String,
    path: String,
    session: SliceSession,
    started: bool,
}

/// Snapshot of the actual entity VM state, used by the live editor debugger.
#[derive(Debug, Clone)]
pub struct EntityScriptDebug {
    /// Stable authored/runtime source ID.
    pub entity_id: String,
    /// Script resource path.
    pub path: String,
    /// Last dispatched lifecycle phase.
    pub phase: String,
    /// Persistent instance values, ordered by key for deterministic inspection.
    pub values: std::collections::BTreeMap<String, Value>,
}

impl AttachedScript {
    fn run(
        &mut self,
        _parent: &mut SliceHost,
        phase: &str,
        world: &mut GameWorld,
        input: (f64, f64),
        dt: f64,
        collisions: &[(String, String, bool)],
    ) -> Result<(), VmError> {
        self.session.vm.host_mut().phase = phase.into();
        self.session.vm.host_mut().frame_count = self.frame.saturating_sub(1);
        #[cfg(feature = "physics")]
        std::mem::swap(
            &mut _parent.physics,
            &mut self.session.vm.host_mut().physics,
        );
        self.session
            .vm
            .host_mut()
            .save_dir
            .clone_from(&_parent.save_dir);
        std::mem::swap(
            &mut _parent.audio_player,
            &mut self.session.vm.host_mut().audio_player,
        );
        std::mem::swap(
            &mut _parent.preferences,
            &mut self.session.vm.host_mut().preferences,
        );
        std::mem::swap(
            &mut _parent.preference_fault,
            &mut self.session.vm.host_mut().preference_fault,
        );
        #[cfg(feature = "audio")]
        std::mem::swap(&mut _parent.audio, &mut self.session.vm.host_mut().audio);
        std::mem::swap(&mut _parent.ui, &mut self.session.vm.host_mut().ui);
        let result = self
            .session
            .step_program(world, input, dt, false, collisions.to_vec());
        std::mem::swap(&mut _parent.ui, &mut self.session.vm.host_mut().ui);
        #[cfg(feature = "audio")]
        std::mem::swap(&mut _parent.audio, &mut self.session.vm.host_mut().audio);
        std::mem::swap(
            &mut _parent.audio_player,
            &mut self.session.vm.host_mut().audio_player,
        );
        std::mem::swap(
            &mut _parent.preferences,
            &mut self.session.vm.host_mut().preferences,
        );
        std::mem::swap(
            &mut _parent.preference_fault,
            &mut self.session.vm.host_mut().preference_fault,
        );
        #[cfg(feature = "physics")]
        std::mem::swap(
            &mut _parent.physics,
            &mut self.session.vm.host_mut().physics,
        );
        result.map_err(|error| {
            VmError::HostCallError(format!(
                "{} (entity {}, {phase}): {error}",
                self.path, self.entity_id
            ))
        })
    }
}

impl SliceSession {
    /// Inspect the same per-instance state used to execute gameplay.
    pub fn entity_script_debug(&self) -> Vec<EntityScriptDebug> {
        self.attached
            .iter()
            .map(|attachment| EntityScriptDebug {
                entity_id: attachment.entity_id.clone(),
                path: attachment.path.clone(),
                phase: attachment.session.vm.host().phase.clone(),
                values: attachment
                    .session
                    .vm
                    .host()
                    .script_state
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            })
            .collect()
    }
    /// Queue a collision event from a platform/custom physics backend.
    pub fn queue_collision_event(
        &mut self,
        a: String,
        b: String,
        started: bool,
    ) -> Result<(), String> {
        if self.external_collisions.len() >= 256 {
            return Err("Collision event queue limit reached".into());
        }
        self.external_collisions.push((a, b, started));
        Ok(())
    }

    /// Inspect an attachment's persistent runtime value without editing defaults.
    pub fn entity_script_state(&self, entity_id: &str, key: &str) -> Option<&Value> {
        self.attached
            .iter()
            .find(|attachment| attachment.entity_id == entity_id)
            .and_then(|attachment| attachment.session.vm.host().script_state.get(key))
    }
    /// Take one deterministic scene transition request after a gameplay tick.
    pub fn take_scene_request(&mut self) -> Option<String> {
        let mut request = self.vm.host_mut().scene_request.take();
        for attached in &mut self.attached {
            if let Some(next) = attached.session.vm.host_mut().scene_request.take() {
                if request.is_none() {
                    request = Some(next);
                }
            }
        }
        request
    }
    /// Compile the scene program and every entity attachment through one loader.
    /// Both native hosts use this path, including projects with only attachments.
    pub fn with_scene_sources(
        project: &kiln_core::Project,
        scene_id: &str,
        read: impl Fn(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        fn compile_resource(
            path: &str,
            project: &kiln_core::Project,
            read: &impl Fn(&str) -> Result<String, String>,
        ) -> Result<SliceSession, String> {
            let expanded = crate::script_source::expand(path, read)?;
            let mut session =
                SliceSession::with_action_map(&expanded.source, project.action_map.clone())
                    .map_err(|errors| {
                        errors
                            .iter()
                            .map(|error| expanded.diagnostic(error))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })?;
            session.source = Some(expanded);
            Ok(session)
        }
        fn gather<'a>(
            project: &'a kiln_core::Project,
            id: &str,
            stack: &mut Vec<String>,
            entities: &mut Vec<&'a kiln_core::Entity>,
        ) -> Result<(), String> {
            if stack.iter().any(|parent| parent == id) {
                return Err(format!("Cyclic nested scene '{id}'"));
            }
            let scene = project
                .scenes
                .get(id)
                .ok_or_else(|| format!("Missing scene '{id}'"))?;
            stack.push(id.into());
            entities.extend(scene.entities.iter());
            for nested in &scene.nested_scenes {
                gather(project, &nested.scene_id, stack, entities)?;
            }
            stack.pop();
            Ok(())
        }
        let scene = project.scenes.get(scene_id).ok_or("Scene is missing")?;
        let mut session = if let Some(path) = &scene.script {
            compile_resource(path, project, &read)?
        } else {
            Self::with_action_map("return 0;", project.action_map.clone())
                .map_err(|errors| format!("{errors:?}"))?
        };
        #[cfg(feature = "physics")]
        {
            session.vm.host_mut().physics = Some(PhysicsWorld::new());
        }
        let mut entities = Vec::new();
        gather(project, scene_id, &mut Vec::new(), &mut entities)?;
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        let mut ids = HashMap::<String, usize>::new();
        for entity in &entities {
            *ids.entry(entity.id.clone()).or_default() += 1;
        }
        for entity in entities {
            let scripts: Vec<_> = entity
                .components
                .iter()
                .filter_map(|component| match component {
                    kiln_core::Component::Script { path, properties } => Some((path, properties)),
                    _ => None,
                })
                .collect();
            if scripts.len() > 1 || (!scripts.is_empty() && ids.get(&entity.id) != Some(&1)) {
                return Err(format!("Ambiguous script attachment for '{}'", entity.id));
            }
            for (path, properties) in scripts {
                if !properties.values().all(|value| value.is_finite()) {
                    return Err(format!("Non-finite script property for '{}'", entity.id));
                }
                let mut child = compile_resource(path, project, &read)?;
                child.vm.host_mut().self_entity = entity.id.clone();
                child.vm.host_mut().properties = properties.clone();
                session.attached.push(AttachedScript {
                    frame: 0,
                    entity_id: entity.id.clone(),
                    path: path.clone(),
                    session: child,
                    started: false,
                });
            }
        }
        session.vm.host_mut().ui = kiln_ui::authored::UiRuntime::new(&project.ui, scene_id)?;
        Ok(session)
    }

    /// Invoke stop callbacks for started attachments before discarding a world.
    pub fn stop(&mut self, world: &mut GameWorld) -> Result<(), VmError> {
        for attached in &mut self.attached {
            if attached.started {
                attached.run(self.vm.host_mut(), "stop", world, (0.0, 0.0), 0.0, &[])?;
                attached.started = false;
            }
        }
        Ok(())
    }
    /// Compile a slice script with the default action map.
    pub fn new(source: &str) -> Result<Self, Vec<Diagnostic>> {
        Self::with_action_map(source, ActionMap::new())
    }

    /// Compile a slice script with a custom action map.
    pub fn with_action_map(source: &str, action_map: ActionMap) -> Result<Self, Vec<Diagnostic>> {
        let mut frontend = KilnScriptFrontend::new();
        for (name, id, argc) in BUILTINS {
            frontend = frontend.with_builtin(*name, *id, *argc);
        }
        let program = frontend.compile(source)?;
        let host = SliceHost {
            action_map: action_map.clone(),
            time_scale: 1.0,
            ..Default::default()
        };
        let vm = Vm::new(host);
        let host = SliceHost {
            action_map,
            time_scale: 1.0,
            ..Default::default()
        };
        let debugger = Debugger::new(program.clone(), host);
        Ok(Self {
            source: None,
            external_collisions: Vec::new(),
            attached: Vec::new(),
            signals: Vec::new(),
            program,
            vm,
            debugger: Some(debugger),
            state: SliceState::default(),
            ui_commands: Vec::new(),
        })
    }

    /// The persistent state of the slice.
    pub fn state(&self) -> &SliceState {
        &self.state
    }

    /// The HUD lines that describe the current state.
    ///
    /// Both hosts draw the same lines, so the standalone player and the
    /// editor show the same information.
    pub fn hud_lines(&self) -> Vec<String> {
        hud_lines(&self.state)
    }

    /// UI commands recorded during the last script run.
    ///
    /// Call after `step()` to retrieve buttons, labels, and panels
    /// that the script requested to be drawn this frame.
    pub fn take_ui_commands(&mut self) -> Vec<SliceCommand> {
        std::mem::take(&mut self.ui_commands)
    }

    /// Get a mutable reference to the debugger, if available.
    pub fn debugger_mut(&mut self) -> Option<&mut Debugger<SliceHost>> {
        self.debugger.as_mut()
    }

    /// Get an immutable reference to the debugger, if available.
    pub fn debugger(&self) -> Option<&Debugger<SliceHost>> {
        self.debugger.as_ref()
    }

    /// Take the debugger out of the session (for transferring to another context).
    pub fn take_debugger(&mut self) -> Option<Debugger<SliceHost>> {
        self.debugger.take()
    }

    /// Replace the debugger (e.g., after hot-reload).
    pub fn set_debugger(&mut self, debugger: Debugger<SliceHost>) {
        self.debugger = Some(debugger);
    }

    /// Get a mutable reference to the host context.
    pub fn host_mut(&mut self) -> &mut SliceHost {
        self.vm.host_mut()
    }

    /// Replace the state of the slice.
    pub fn set_state(&mut self, state: SliceState) {
        self.state = state;
    }

    /// Run the script for one frame.
    ///
    /// `input` is the (x, y) movement axis and `dt` the frame time in
    /// seconds. The commands the script emits are applied to `world`.
    pub fn step(
        &mut self,
        world: &mut GameWorld,
        input: (f64, f64),
        dt: f64,
    ) -> Result<(), VmError> {
        #[cfg(feature = "audio")]
        if let Some(audio) = &mut self.vm.host_mut().audio {
            audio.advance_silent(dt);
        }
        self.dispatch_saved_ui(world)?;
        let paused = self.vm.host().ui.pauses_game();
        let input = if self.vm.host().ui.captures_input() {
            (0.0, 0.0)
        } else {
            input
        };
        let captures = self.vm.host().ui.captures_input();
        if !paused {
            let keys = if captures {
                Some(std::mem::take(&mut self.vm.host_mut().pressed_keys))
            } else {
                None
            };
            let result = self.step_program(world, input, dt, true, Vec::new());
            if let Some(keys) = keys {
                self.vm.host_mut().pressed_keys = keys;
            }
            result?;
        }
        let collisions = if paused {
            Vec::new()
        } else {
            self.vm.host_mut().facts.collision_events.clone()
        };
        let animation_events: Vec<_> = world
            .world_mut()
            .query_mut::<(&SourceId, &mut crate::AnimationPlayer)>()
            .into_iter()
            .flat_map(|(source, animation)| {
                animation
                    .take_events()
                    .into_iter()
                    .map(|event| (source.0.clone(), event))
                    .collect::<Vec<_>>()
            })
            .collect();
        let signals = std::mem::take(&mut self.signals);
        let keys = if captures {
            Default::default()
        } else {
            self.vm.host_mut().pressed_keys.clone()
        };
        let mouse_world = self.vm.host().mouse_world;
        let save_dir = self.vm.host_mut().save_dir.clone();
        let frame = self.vm.host().frame_count;
        self.signals.append(&mut self.vm.host_mut().emitted_signals);
        for attached in &mut self.attached {
            attached.frame = frame;
            if !world
                .world_ref()
                .query::<&SourceId>()
                .iter()
                .any(|id| id.0 == attached.entity_id)
            {
                if attached.started {
                    attached.run(self.vm.host_mut(), "stop", world, input, 0.0, &collisions)?;
                    attached.started = false;
                    self.signals
                        .append(&mut attached.session.vm.host_mut().emitted_signals);
                }
                continue;
            }
            attached.session.vm.host_mut().pressed_keys = keys.clone();
            attached.session.vm.host_mut().mouse_world = mouse_world;
            attached.session.vm.host_mut().save_dir = save_dir.clone();
            if !attached.started {
                attached.started = true;
                attached.run(self.vm.host_mut(), "start", world, input, 0.0, &collisions)?;
            }
            for (id, event) in &animation_events {
                if id != &attached.entity_id {
                    continue;
                }
                attached.session.vm.host_mut().animation_event = (
                    event.name.clone(),
                    event.payload.clone().unwrap_or_default(),
                );
                attached.run(
                    self.vm.host_mut(),
                    "animation_event",
                    world,
                    input,
                    0.0,
                    &collisions,
                )?;
            }
            for (name, value) in &signals {
                attached.session.vm.host_mut().signal_name = name.clone();
                attached.session.vm.host_mut().signal_value = Some(value.clone());
                attached.run(self.vm.host_mut(), "signal", world, input, 0.0, &collisions)?;
            }
            for (a, b, started) in &collisions {
                let other = if a == &attached.entity_id {
                    b
                } else if b == &attached.entity_id {
                    a
                } else {
                    continue;
                };
                attached.session.vm.host_mut().event_entity = other.clone();
                attached.run(
                    self.vm.host_mut(),
                    if *started {
                        "collision_enter"
                    } else {
                        "collision_exit"
                    },
                    world,
                    input,
                    0.0,
                    &collisions,
                )?;
            }
            if !self.vm.host().ui.pauses_game() {
                attached.run(self.vm.host_mut(), "update", world, input, dt, &collisions)?;
            }
            self.ui_commands.append(&mut attached.session.ui_commands);
            self.signals
                .append(&mut attached.session.vm.host_mut().emitted_signals);
        }
        if self.signals.len() > 256 {
            return Err(VmError::HostCallError(
                "scene signal queue limit reached".into(),
            ));
        }
        Ok(())
    }

    fn step_program(
        &mut self,
        world: &mut GameWorld,
        input: (f64, f64),
        dt: f64,
        advance_systems: bool,
        inherited_collisions: Vec<(String, String, bool)>,
    ) -> Result<(), VmError> {
        self.state.elapsed += dt;

        // Sync persistent state to host so save_game can access it.
        self.vm.host_mut().state = self.state.clone();

        // Reset per-frame UI click state (clicks only last one frame).
        self.host_mut().ui_widgets.clear();

        // Step physics and collect collision events (if physics feature enabled)
        #[cfg(feature = "physics")]
        let collision_events = {
            use crate::physics::sync_physics_with_ecs;
            let host = self.vm.host_mut();
            if let Some(physics) = host.physics.as_mut().filter(|_| advance_systems) {
                sync_physics_with_ecs(physics, world)
            } else {
                Vec::new()
            }
        };
        #[cfg(feature = "physics")]
        let collision_events: Vec<(String, String, bool)> = collision_events
            .into_iter()
            .map(|e| (e.entity_a, e.entity_b, e.started))
            .collect();
        #[cfg(not(feature = "physics"))]
        let collision_events: Vec<(String, String, bool)> = Vec::new();

        let mut collision_events = if advance_systems {
            collision_events
        } else {
            inherited_collisions
        };
        collision_events.append(&mut self.external_collisions);

        // Update animations (reads AnimationPlayer, writes Sprite uv_rect)
        if advance_systems {
            crate::sprite::update_animations_dt(world.world_mut(), dt as f32);
            // Update particle systems
            crate::sprite::update_particle_systems(world.world_mut(), dt as f32);
        }

        let facts = self.collect_facts(world, input, dt, collision_events);
        {
            let host = self.vm.host_mut();
            host.facts = facts;
            host.take_commands();
        }

        if let Err(error) = self.vm.execute(&self.program) {
            if let Some(source) = &self.source {
                let offset = self
                    .program
                    .source_offset(self.vm.instruction_pointer())
                    .unwrap_or(0);
                let diagnostic =
                    Diagnostic::new(kiln_script::Span::new(offset, offset), error.to_string());
                return Err(VmError::HostCallError(source.diagnostic(&diagnostic)));
            }
            return Err(error);
        }

        // Loading can replace host state during script execution. Adopt it before
        // applying this frame's commands so score, pickups and movement survive.
        self.state = self.vm.host_mut().state.clone();

        let commands = self.vm.host_mut().take_commands();
        for command in commands {
            match &command {
                SliceCommand::UiButton { .. }
                | SliceCommand::UiLabel { .. }
                | SliceCommand::UiPanel { .. } => {
                    // UI commands are rendering instructions, not world mutations.
                    self.ui_commands.push(command);
                }
                _ => {
                    self.apply(command, world);
                }
            }
        }

        // Detect UI button clicks: check if any button rect contains the mouse
        // and the left button was pressed this frame.
        {
            let host = self.host_mut();
            if host.mouse_pressed {
                for (id, bx, by, bw, bh) in &host.ui_buttons {
                    if host.mouse_x >= *bx
                        && host.mouse_x < bx + bw
                        && host.mouse_y >= *by
                        && host.mouse_y < by + bh
                    {
                        host.ui_widgets.insert(id.clone(), true);
                    }
                }
            }

            // Clear per-frame UI tracking for next frame.
            host.ui_buttons.clear();
        }

        Ok(())
    }

    /// Gather the facts the script can read this frame.
    fn collect_facts(
        &mut self,
        world: &mut GameWorld,
        input: (f64, f64),
        dt: f64,
        collision_events: Vec<(String, String, bool)>,
    ) -> SliceFacts {
        let player = find_player(world);
        let (player_x, player_y) = player.as_ref().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));

        let coins = find_coins(world);
        let coin_in_reach = coins
            .iter()
            .filter(|coin| !self.state.is_collected(&coin.source_id))
            .find(|coin| {
                let dx = coin.x - player_x;
                let dy = coin.y - player_y;
                (dx * dx + dy * dy).sqrt() <= PICKUP_RADIUS
            })
            .map(|coin| coin.source_id.clone());

        let remaining = coins
            .iter()
            .filter(|coin| !self.state.is_collected(&coin.source_id))
            .count() as i64;

        // Collect entity positions, velocities, and roles for script queries.
        let mut entity_positions = HashMap::new();
        let mut entity_velocities = HashMap::new();
        let mut entity_roles = HashMap::new();
        let mut entity_sprite_colors = HashMap::new();
        let mut entity_sprite_sizes = HashMap::new();

        use crate::sprite::Sprite;
        use crate::{Role, SourceId};

        use kiln_core::Transform;

        for (source_id, transform) in world.world_mut().query_mut::<(&SourceId, &Transform)>() {
            entity_positions.insert(
                source_id.0.clone(),
                (transform.position.x as f64, transform.position.y as f64),
            );
        }
        for (source_id, velocity) in world
            .world_ref()
            .query::<(&SourceId, &crate::Velocity)>()
            .iter()
        {
            entity_velocities.insert(
                source_id.0.clone(),
                (velocity.0[0] as f64, velocity.0[1] as f64),
            );
        }
        for (source_id, body) in world
            .world_ref()
            .query::<(&SourceId, &crate::world::RigidBody2D)>()
            .iter()
        {
            entity_velocities.insert(
                source_id.0.clone(),
                (
                    body.linear_velocity[0] as f64,
                    body.linear_velocity[1] as f64,
                ),
            );
        }

        for (source_id, role) in world.world_mut().query_mut::<(&SourceId, &Role)>() {
            entity_roles.insert(source_id.0.clone(), role.0.clone());
        }

        for (source_id, sprite) in world.world_mut().query_mut::<(&SourceId, &Sprite)>() {
            entity_sprite_colors.insert(source_id.0.clone(), sprite.color.map(|c| c as f64));
            entity_sprite_sizes.insert(source_id.0.clone(), sprite.size.map(|s| s as f64));
        }

        self.host_mut().frame_count += 1;

        SliceFacts {
            input_x: input.0,
            input_y: input.1,
            elapsed: self.state.elapsed,
            score: self.state.score,
            coin_count: remaining,
            coin_in_reach,
            status: self.state.status.clone(),
            delta_time: dt,
            frame_count: self.host_mut().frame_count,
            time_scale: self.host_mut().time_scale,
            entity_positions,
            entity_velocities,
            entity_sprite_colors,
            entity_sprite_sizes,
            entity_roles,
            collision_events,
        }
    }

    /// Apply one command to the world and the slice state.
    fn apply(&mut self, command: SliceCommand, world: &mut GameWorld) {
        match command {
            SliceCommand::MovePlayer { dx, dy } => {
                if let Some(player) = find_player(world) {
                    let target = crate::tilemap_collision::move_with_collision(
                        world,
                        [
                            player.x + player.collision_offset[0],
                            player.y + player.collision_offset[1],
                        ],
                        player.size,
                        [dx as f32, dy as f32],
                    );
                    let x = (target[0] - player.collision_offset[0]) as f64;
                    let y = (target[1] - player.collision_offset[1]) as f64;
                    set_player_position(world, &player.source_id, x, y);
                    self.state.player_x = x;
                    self.state.player_y = y;
                }
            }
            SliceCommand::AddScore(amount) => {
                self.state.score += amount;
            }
            SliceCommand::CollectCoin(source_id) => {
                if self.state.is_collected(&source_id) {
                    return;
                }
                despawn_coin(world, &source_id);
                self.state.collected.push(source_id);
                self.state.collected.sort();
            }
            SliceCommand::SetStatus(text) => {
                self.state.status = text;
            }
            SliceCommand::SpawnEntity {
                name,
                x,
                y,
                texture_path: _,
            } => {
                use crate::{Role, SourceId, Sprite};
                use kiln_core::Transform;
                let source_id = crate::world::next_entity_id(&[]);
                world.world_mut().spawn((
                    Transform::new(x as f32, y as f32),
                    Sprite::new(1.0, 1.0, 1.0, 1.0)
                        .with_size(32.0, 32.0)
                        .with_layer(0.0),
                    SourceId::new(source_id.clone()),
                    Role::new(name),
                ));
            }
            SliceCommand::DespawnEntity(source_id) => {
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        let _ = world.world_mut().despawn(eid);
                        break;
                    }
                }
            }
            SliceCommand::SetEntityPos { source_id, x, y } => {
                use crate::SourceId;
                use kiln_core::Transform;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(transform) =
                            world.world_mut().query_one_mut::<&mut Transform>(eid)
                        {
                            transform.position.x = x as f32;
                            transform.position.y = y as f32;
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetEntityVel { source_id, vx, vy } => {
                use crate::{SourceId, Velocity};
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(body) = world
                            .world_mut()
                            .query_one_mut::<&mut crate::world::RigidBody2D>(eid)
                        {
                            body.linear_velocity = [vx as f32, vy as f32];
                        }
                        if let Ok(velocity) = world.world_mut().query_one_mut::<&mut Velocity>(eid)
                        {
                            velocity.0[0] = vx as f32;
                            velocity.0[1] = vy as f32;
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetEntityComponent {
                source_id,
                component,
                value,
            } => {
                #[cfg(feature = "physics")]
                use crate::Velocity;
                use crate::{SourceId, Sprite};
                use kiln_core::Transform;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        match component.as_str() {
                            "transform" | "Transform" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 2 {
                                        if let (Some(Value::F64(x)), Some(Value::F64(y))) =
                                            (arr.first(), arr.get(1))
                                        {
                                            if let Ok(transform) = world
                                                .world_mut()
                                                .query_one_mut::<&mut Transform>(eid)
                                            {
                                                transform.position.x = *x as f32;
                                                transform.position.y = *y as f32;
                                            }
                                        }
                                    }
                                }
                            }
                            "velocity" | "Velocity" => {
                                #[cfg(feature = "physics")]
                                {
                                    if let Value::Array(arr) = value {
                                        if arr.len() >= 2 {
                                            if let (Some(Value::F64(vx)), Some(Value::F64(vy))) =
                                                (arr.first(), arr.get(1))
                                            {
                                                if let Ok(body) = world.world_mut().query_one_mut::<&mut crate::world::RigidBody2D>(eid) {
                                                    body.linear_velocity = [*vx as f32, *vy as f32];
                                                }
                                                if let Ok(velocity) = world
                                                    .world_mut()
                                                    .query_one_mut::<&mut Velocity>(eid)
                                                {
                                                    velocity.0[0] = *vx as f32;
                                                    velocity.0[1] = *vy as f32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "sprite_color" | "SpriteColor" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 4 {
                                        if let (
                                            Some(Value::F64(r)),
                                            Some(Value::F64(g)),
                                            Some(Value::F64(b)),
                                            Some(Value::F64(a)),
                                        ) = (arr.first(), arr.get(1), arr.get(2), arr.get(3))
                                        {
                                            if let Ok(sprite) =
                                                world.world_mut().query_one_mut::<&mut Sprite>(eid)
                                            {
                                                sprite.color =
                                                    [*r as f32, *g as f32, *b as f32, *a as f32];
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
            SliceCommand::AddComponent {
                source_id,
                component_type,
                value,
            } => {
                #[cfg(feature = "physics")]
                use crate::Velocity;
                use crate::{SourceId, Sprite};
                use kiln_core::Transform;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        match component_type.as_str() {
                            "transform" | "Transform" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 2 {
                                        if let (Some(Value::F64(x)), Some(Value::F64(y))) =
                                            (arr.first(), arr.get(1))
                                        {
                                            if let Ok(transform) = world
                                                .world_mut()
                                                .query_one_mut::<&mut Transform>(eid)
                                            {
                                                transform.position.x = *x as f32;
                                                transform.position.y = *y as f32;
                                            }
                                        }
                                    }
                                }
                            }
                            "velocity" | "Velocity" => {
                                #[cfg(feature = "physics")]
                                {
                                    if let Value::Array(arr) = value {
                                        if arr.len() >= 2 {
                                            if let (Some(Value::F64(vx)), Some(Value::F64(vy))) =
                                                (arr.first(), arr.get(1))
                                            {
                                                if let Ok(body) = world.world_mut().query_one_mut::<&mut crate::world::RigidBody2D>(eid) {
                                                    body.linear_velocity = [*vx as f32, *vy as f32];
                                                }
                                                if let Ok(velocity) = world
                                                    .world_mut()
                                                    .query_one_mut::<&mut Velocity>(eid)
                                                {
                                                    velocity.0[0] = *vx as f32;
                                                    velocity.0[1] = *vy as f32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "sprite" | "Sprite" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 4 {
                                        if let (
                                            Some(Value::F64(r)),
                                            Some(Value::F64(g)),
                                            Some(Value::F64(b)),
                                            Some(Value::F64(a)),
                                        ) = (arr.first(), arr.get(1), arr.get(2), arr.get(3))
                                        {
                                            if let Ok(sprite) =
                                                world.world_mut().query_one_mut::<&mut Sprite>(eid)
                                            {
                                                sprite.color =
                                                    [*r as f32, *g as f32, *b as f32, *a as f32];
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
            SliceCommand::RemoveComponent {
                source_id,
                component_type,
            } => {
                #[cfg(feature = "physics")]
                use crate::Velocity;
                use crate::{SourceId, Sprite};
                use kiln_core::Transform;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        match component_type.as_str() {
                            "transform" | "Transform" => {
                                let _ = world.world_mut().remove::<Transform>(eid);
                            }
                            "sprite" | "Sprite" => {
                                let _ = world.world_mut().remove::<Sprite>(eid);
                            }
                            "velocity" | "Velocity" => {
                                #[cfg(feature = "physics")]
                                {
                                    let _ = world.world_mut().remove::<Velocity>(eid);
                                }
                                #[cfg(not(feature = "physics"))]
                                let _ = eid;
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
            SliceCommand::PlayAnimation {
                source_id,
                clip_name,
            } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            anim.play(&clip_name);
                        } else {
                            let mut anim = AnimationPlayer::new();
                            anim.play(&clip_name);
                            let _ = world.world_mut().insert_one(eid, anim);
                        }
                        break;
                    }
                }
            }
            SliceCommand::StopAnimation { source_id } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            anim.stop();
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetAnimationSpeed { source_id, speed } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            anim.set_speed(speed as f32);
                        } else {
                            let mut anim = AnimationPlayer::new();
                            anim.set_speed(speed as f32);
                            let _ = world.world_mut().insert_one(eid, anim);
                        }
                        break;
                    }
                }
            }
            // UI commands are no-ops in apply() — they are handled via take_ui_commands().
            SliceCommand::UiButton { .. }
            | SliceCommand::UiLabel { .. }
            | SliceCommand::UiPanel { .. } => {}
            SliceCommand::LoadGame { slot: _ } => {
                // Load state from disk and apply it to the session + world.
                // TODO: Add save_dir field to SliceSession or use a default path
                log::warn!("LoadGame command not yet implemented - save_dir not set");
            }
            SliceCommand::CreateAnimationClip {
                source_id,
                clip_name,
                texture_path,
            } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                use kiln_anim::Clip;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            anim.add_clip(Clip::new(clip_name, texture_path));
                        } else {
                            let mut anim = AnimationPlayer::new();
                            anim.add_clip(Clip::new(clip_name, texture_path));
                            let _ = world.world_mut().insert_one(eid, anim);
                        }
                        break;
                    }
                }
            }
            SliceCommand::AddAnimationFrame {
                source_id,
                clip_name,
                x,
                y,
                w,
                h,
                duration,
            } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                use kiln_anim::{Clip, Frame};
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            if let Some(clip) = anim.clips_mut().get_mut(&clip_name) {
                                clip.add_frame(Frame {
                                    rect: [x as u32, y as u32, w as u32, h as u32],
                                    pivot: [0.5, 0.5],
                                    duration: duration as f32,
                                    events: Vec::new(),
                                });
                            } else {
                                let mut clip = Clip::new(clip_name.clone(), String::new());
                                clip.add_frame(Frame {
                                    rect: [x as u32, y as u32, w as u32, h as u32],
                                    pivot: [0.5, 0.5],
                                    duration: duration as f32,
                                    events: Vec::new(),
                                });
                                anim.add_clip(clip);
                            }
                        } else {
                            let mut anim = AnimationPlayer::new();
                            let mut clip = Clip::new(clip_name.clone(), String::new());
                            clip.add_frame(Frame {
                                rect: [x as u32, y as u32, w as u32, h as u32],
                                pivot: [0.5, 0.5],
                                duration: duration as f32,
                                events: Vec::new(),
                            });
                            anim.add_clip(clip);
                            let _ = world.world_mut().insert_one(eid, anim);
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetAnimationLoop {
                source_id,
                clip_name,
                loops,
            } => {
                use crate::sprite::AnimationPlayer;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(anim) =
                            world.world_mut().query_one_mut::<&mut AnimationPlayer>(eid)
                        {
                            if let Some(clip) = anim.clips_mut().get_mut(&clip_name) {
                                clip.loops = loops;
                            }
                        }
                        break;
                    }
                }
            }
            SliceCommand::CreateParticleSystem { source_id } => {
                use crate::sprite::ParticleSystemComponent;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(_ps) = world
                            .world_mut()
                            .query_one_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            // Already has a particle system
                        } else {
                            let _ = world
                                .world_mut()
                                .insert_one(eid, ParticleSystemComponent::new());
                        }
                        break;
                    }
                }
            }
            SliceCommand::AddParticleEmitter {
                source_id,
                rate,
                speed_min,
                speed_max,
                angle_min,
                angle_max,
                lifetime_min,
                lifetime_max,
                size_min,
                size_max,
                color,
                gravity,
                fade_out,
            } => {
                use crate::sprite::ParticleSystemComponent;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(ps) = world
                            .world_mut()
                            .query_one_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            ps.system_mut().add_emitter(kiln_vfx::Emitter::new(
                                kiln_vfx::EmitterConfig {
                                    shape: kiln_vfx::EmissionShape::Point,
                                    rate: rate as f32,
                                    speed_range: [speed_min as f32, speed_max as f32],
                                    angle_range: [angle_min as f32, angle_max as f32],
                                    lifetime_range: [lifetime_min as f32, lifetime_max as f32],
                                    size_range: [size_min as f32, size_max as f32],
                                    color: [
                                        color[0] as f32,
                                        color[1] as f32,
                                        color[2] as f32,
                                        color[3] as f32,
                                    ],
                                    gravity: [gravity[0] as f32, gravity[1] as f32],
                                    fade_out,
                                },
                            ));
                        } else {
                            let mut ps = ParticleSystemComponent::new();
                            ps.system_mut().add_emitter(kiln_vfx::Emitter::new(
                                kiln_vfx::EmitterConfig {
                                    shape: kiln_vfx::EmissionShape::Point,
                                    rate: rate as f32,
                                    speed_range: [speed_min as f32, speed_max as f32],
                                    angle_range: [angle_min as f32, angle_max as f32],
                                    lifetime_range: [lifetime_min as f32, lifetime_max as f32],
                                    size_range: [size_min as f32, size_max as f32],
                                    color: [
                                        color[0] as f32,
                                        color[1] as f32,
                                        color[2] as f32,
                                        color[3] as f32,
                                    ],
                                    gravity: [gravity[0] as f32, gravity[1] as f32],
                                    fade_out,
                                },
                            ));
                            let _ = world.world_mut().insert_one(eid, ps);
                        }
                        break;
                    }
                }
            }
            SliceCommand::StartParticleSystem { source_id } => {
                for (id, effect) in world
                    .world_mut()
                    .query_mut::<(&SourceId, &mut crate::ParticleSystemComponent)>()
                {
                    if id.0 == source_id {
                        effect.system_mut().emitting = true;
                    }
                }
            }
            SliceCommand::StopParticleSystem { source_id } => {
                use crate::sprite::ParticleSystemComponent;
                use crate::SourceId;
                for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
                    if sid.0 == source_id {
                        if let Ok(ps) = world
                            .world_mut()
                            .query_one_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            ps.system_mut().emitting = false;
                        }
                        break;
                    }
                }
            }
        }
    }
    /// Make the world match the slice state: collected pickups are removed and
    /// the player is put back where it was saved.
    pub fn sync_world(&mut self, world: &mut GameWorld) {
        for source_id in self.state.collected.clone() {
            despawn_coin(world, &source_id);
        }
        let (x, y) = (self.state.player_x, self.state.player_y);
        if let Some(player) = find_player(world) {
            set_player_position(world, &player.source_id, x, y);
        }
    }
}

/// A world entity with a role and a stable source id.
struct WorldEntity {
    collision_offset: [f32; 2],
    source_id: String,
    x: f32,
    y: f32,
    /// Size of the entity's sprite, used as its collision box.
    size: [f32; 2],
}

/// Find the player entity, if the world has one.
fn find_player(world: &mut GameWorld) -> Option<WorldEntity> {
    find_entities(world, ROLE_PLAYER).into_iter().next()
}

/// Find all entities with the given role.
fn find_entities(world: &mut GameWorld, role: &str) -> Vec<WorldEntity> {
    let mut found = Vec::new();
    for (source_id, entity_role, transform, sprite) in world.world_mut().query_mut::<(
        &SourceId,
        &Role,
        &kiln_core::Transform,
        Option<&crate::sprite::Sprite>,
    )>() {
        if entity_role.as_str() == role {
            let bounds = sprite.map(|sprite| sprite.world_bounds(*transform));
            found.push(WorldEntity {
                collision_offset: bounds
                    .map(|b| {
                        [
                            (b[0] + b[2]) * 0.5 - transform.position.x,
                            (b[1] + b[3]) * 0.5 - transform.position.y,
                        ]
                    })
                    .unwrap_or([0.0; 2]),
                source_id: source_id.0.clone(),
                x: transform.position.x,
                y: transform.position.y,
                size: bounds
                    .map(|b| [b[2] - b[0], b[3] - b[1]])
                    .unwrap_or([0.0; 2]),
            });
        }
    }
    found
}

/// Find all pickup entities.
fn find_coins(world: &mut GameWorld) -> Vec<WorldEntity> {
    find_entities(world, ROLE_COIN)
}

/// Move the player with the given source id.
fn set_player_position(world: &mut GameWorld, source_id: &str, x: f64, y: f64) {
    for (entity_source, transform) in world
        .world_mut()
        .query_mut::<(&SourceId, &mut kiln_core::Transform)>()
    {
        if entity_source.0 == source_id {
            transform.position.x = x as f32;
            transform.position.y = y as f32;
        }
    }
}

/// Remove the pickup with the given source id from the world.
fn despawn_coin(world: &mut GameWorld, source_id: &str) {
    let target = {
        let mut target = None;
        for (entity, entity_source) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
            if entity_source.0 == source_id {
                target = Some(entity);
                break;
            }
        }
        target
    };
    if let Some(entity) = target {
        let _ = world.world_mut().despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::Transform;

    /// Spawn a player entity with the given position.
    fn spawn_player(world: &mut GameWorld, x: f32, y: f32) {
        world.spawn((
            Transform::new(x, y),
            Role::new(ROLE_PLAYER),
            SourceId::new("player"),
        ));
    }

    /// Spawn a pickup entity with the given position.
    fn spawn_coin(world: &mut GameWorld, id: &str, x: f32, y: f32) {
        world.spawn((
            Transform::new(x, y),
            Role::new(ROLE_COIN),
            SourceId::new(id),
        ));
    }

    /// The number of entities currently in the world.
    fn entity_count(world: &GameWorld) -> u32 {
        world.world_ref().len()
    }

    #[test]
    fn script_moves_the_player() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 10.0, 20.0);

        let mut session = SliceSession::new("move_player(5.0, -3.0);").unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();

        let position = find_player(&mut world).unwrap();
        assert!((position.x - 15.0).abs() < 0.001);
        assert!((position.y - 17.0).abs() < 0.001);
    }

    #[test]
    fn platform_sound_callback_works_without_a_native_audio_device() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let mut session = SliceSession::new("play_sound(\"hit\");").unwrap();
        session.host_mut().set_audio_player(Box::new(move |name| {
            recorded.lock().unwrap().push(name.to_string());
        }));
        let mut world = GameWorld::new();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        assert_eq!(*calls.lock().unwrap(), vec!["hit", "hit"]);
    }

    #[test]
    fn commands_persist_across_frames_for_a_nonvisual_player() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 10.0, 20.0);
        let mut session = SliceSession::new("move_player(1.0, 0.0); add_score(3);").unwrap();
        for frame in 1..=4 {
            session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
            assert_eq!(session.state().score, frame * 3);
            assert_eq!(session.state().player_x, 10.0 + frame as f64);
            assert_eq!(find_player(&mut world).unwrap().x, 10.0 + frame as f32);
        }
    }

    #[test]
    fn script_reads_input() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 0.0, 0.0);

        let mut session = SliceSession::new("move_player(get_input_x(), get_input_y());").unwrap();
        session.step(&mut world, (2.0, -1.0), 0.016).unwrap();

        let position = find_player(&mut world).unwrap();
        assert!((position.x - 2.0).abs() < 0.001);
        assert!((position.y + 1.0).abs() < 0.001);
    }

    #[test]
    fn script_collects_a_coin_in_reach() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 0.0, 0.0);
        spawn_coin(&mut world, "coin1", 5.0, 0.0);

        let mut session =
            SliceSession::new("if collect_coin() { add_score(10); set_status(\"got it\"); }")
                .unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();

        assert_eq!(session.state().score, 10);
        assert_eq!(session.state().status, "got it");
        assert_eq!(session.state().collected, vec!["coin1".to_string()]);
        // The coin entity is gone from the world.
        assert_eq!(entity_count(&world), 1);
    }

    #[test]
    fn script_does_not_collect_a_distant_coin() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 0.0, 0.0);
        spawn_coin(&mut world, "coin1", 500.0, 0.0);

        let mut session = SliceSession::new("if collect_coin() { add_score(10); }").unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();

        assert_eq!(session.state().score, 0);
        assert_eq!(entity_count(&world), 2);
    }

    #[test]
    fn state_round_trips_through_json() {
        let state = SliceState {
            score: 42,
            elapsed: 12.5,
            status: "hello".to_string(),
            collected: vec!["coin1".to_string()],
            player_x: 3.0,
            player_y: 4.0,
        };
        let json = serde_json::to_string(&state).unwrap();
        let loaded: SliceState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn sync_world_removes_collected_coins() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 0.0, 0.0);
        spawn_coin(&mut world, "coin1", 5.0, 0.0);

        let mut session = SliceSession::new("if collect_coin() { add_score(10); }").unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        assert_eq!(entity_count(&world), 1);

        // Rebuild the world and restore the state.
        let mut rebuilt = GameWorld::new();
        spawn_player(&mut rebuilt, 0.0, 0.0);
        spawn_coin(&mut rebuilt, "coin1", 5.0, 0.0);
        session.set_state(session.state().clone());
        session.sync_world(&mut rebuilt);

        assert_eq!(entity_count(&rebuilt), 1);
    }

    #[test]
    fn the_player_position_survives_a_save_and_load() {
        let mut world = GameWorld::new();
        spawn_player(&mut world, 0.0, 0.0);

        let mut session = SliceSession::new("move_player(3.0, 4.0);").unwrap();
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        assert!((session.state().player_x - 3.0).abs() < 0.001);
        assert!((session.state().player_y - 4.0).abs() < 0.001);

        // Rebuild the world and restore the state.
        let mut rebuilt = GameWorld::new();
        spawn_player(&mut rebuilt, 0.0, 0.0);
        let saved = session.state().clone();
        session.set_state(saved);
        session.sync_world(&mut rebuilt);

        let position = find_player(&mut rebuilt).unwrap();
        assert!((position.x - 3.0).abs() < 0.001, "the player is back");
        assert!((position.y - 4.0).abs() < 0.001, "the player is back");
    }

    #[test]
    fn unknown_function_is_a_compile_error() {
        assert!(SliceSession::new("not_a_function();").is_err());
    }
}
