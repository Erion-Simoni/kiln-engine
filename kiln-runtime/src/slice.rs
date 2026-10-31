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

use std::collections::HashMap;
use std::path::Path;

use kiln_ir::{HostContext, Program, Value, Vm, VmError};
use kiln_save::slot::SaveSlotError;
use kiln_save::SaveSlotManager;
use kiln_script::{Diagnostic, KilnScriptFrontend, LanguageFrontend};
use serde::{Deserialize, Serialize};

#[cfg(feature = "audio")]
use crate::audio::AudioEngine;

use crate::world::{GameWorld, Role, SourceId};
use crate::sprite::AnimationPlayer;

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
    // Entity management (17-22)
    ("spawn_entity", 17, 4),       // name, x, y, texture_path?
    ("despawn_entity", 18, 1),     // source_id
    ("get_entity_pos", 19, 1),     // source_id -> (x, y)
    ("set_entity_pos", 20, 3),     // source_id, x, y
    // Query builtins (21-25)
    ("get_entities_with_role", 21, 1),  // role -> array of source_ids
    ("get_entity_component", 22, 2),    // source_id, component_name -> value
    ("set_entity_component", 23, 3),    // source_id, component_name, value
    // World queries (24-25)
    ("find_nearest", 24, 3),         // x, y, role -> source_id
    ("raycast", 25, 4),              // x, y, dx, dy -> hit info
    // Audio extended (26-27)
    ("sound_playing", 26, 1),        // sound_name -> bool
    ("music_playing", 27, 0),        // -> bool
    // Time/frame (28-29)
    ("get_frame_count", 28, 0),      // -> frame number
    ("get_time_scale", 29, 0),       // -> time scale
    // Extended entity API (30-34)
    ("set_entity_vel", 30, 3),       // source_id, vx, vy
    ("get_entities_at_position", 31, 3), // x, y, radius -> array of source_ids
    ("get_entity_role", 32, 1),      // source_id -> role string
    ("add_component", 33, 3),        // source_id, component_type, value_json
    ("remove_component", 34, 2),     // source_id, component_type
    // Animation API (35-37)
    ("play_animation", 35, 2),       // source_id, clip_name
    ("stop_animation", 36, 1),       // source_id
    ("set_animation_speed", 37, 2),  // source_id, speed
    // Collision events (38)
    ("get_collision_events", 38, 0), // -> array of [entity_a, entity_b, started]
    // UI widgets (39-42)
    ("ui_button", 39, 3),        // id, x, y -> clicked bool
    ("ui_label", 40, 4),         // id, x, y, text -> void
    ("ui_panel", 41, 5),         // id, x, y, w, h -> void
    ("ui_clicked", 42, 1),       // id -> clicked bool
    // Input actions (43-44)
    ("get_action", 43, 1),       // action_name -> pressed bool
    ("get_action_axis", 44, 1),  // action_name -> -1 to 1 value
    // Localization (45)
    ("localize", 45, 1),         // key -> localized string
    // Save/load (46-47)
    ("save_game", 46, 1),        // slot_id -> bool
    ("load_game", 47, 1),        // slot_id -> bool
    // Animation clip creation (48-50)
    ("create_animation_clip", 48, 3),  // source_id, clip_name, texture_path -> void
    ("add_animation_frame", 49, 6),    // source_id, clip_name, x, y, w, h, duration -> void
    ("set_animation_loop", 50, 2),     // source_id, clip_name, loops -> void
    // Particle system (51-54)
    ("create_particle_system", 51, 1),  // source_id -> void
    ("add_particle_emitter", 52, 16),   // source_id, rate, speed_min, speed_max, angle_min, angle_max, lifetime_min, lifetime_max, size_min, size_max, r, g, b, a, gx, gy, fade_out -> void
    ("start_particle_system", 53, 1),   // source_id -> void (already implicit with emitter)
    ("stop_particle_system", 54, 1),    // source_id -> void
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
    /// Map of entity source_id -> (x, y) position for entities with Role component.
    pub entity_positions: HashMap<String, (f64, f64)>,
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
    StopAnimation {
        source_id: String,
    },
    /// Set animation playback speed on an entity.
    SetAnimationSpeed {
        source_id: String,
        speed: f64,
    },
    /// Render a UI button at screen position.
    UiButton {
        id: String,
        x: f64,
        y: f64,
    },
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
    LoadGame {
        slot: u32,
    },
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
    CreateParticleSystem {
        source_id: String,
    },
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
    StartParticleSystem {
        source_id: String,
    },
    /// Stop particle system emission.
    StopParticleSystem {
        source_id: String,
    },
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

/// The script host: reads facts and records commands.
#[derive(Debug, Default)]
pub struct SliceHost {
    facts: SliceFacts,
    commands: Vec<SliceCommand>,
    #[cfg(feature = "audio")]
    audio: Option<AudioEngine>,
    #[cfg(feature = "physics")]
    physics: Option<PhysicsWorld>,
    animation_players: HashMap<String, AnimationPlayer>,
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
}

impl SliceHost {
    /// A host with the given facts.
    pub fn new(facts: SliceFacts) -> Self {
        Self {
            facts,
            commands: Vec::new(),
            #[cfg(feature = "audio")]
            audio: AudioEngine::new().ok(),
            #[cfg(feature = "physics")]
            physics: PhysicsWorld::new().ok(),
            animation_players: HashMap::new(),
            ui_widgets: HashMap::new(),
            ui_buttons: Vec::new(),
            action_map: kiln_input::ActionMap::new(),
            localization: None,
            save_dir: None,
            state: SliceState::default(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_pressed: false,
            pressed_keys: std::collections::HashSet::new(),
            frame_count: 0,
            time_scale: 1.0,
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

    /// Whether a UI widget was clicked this frame.
    pub fn ui_widget_clicked(&self, id: &str) -> bool {
        self.ui_widgets.get(id).copied().unwrap_or(false)
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
        self.save_dir = Some(dir.to_path_buf());
    }

    /// Set the currently pressed keys (called by the host before each step).
    pub fn set_pressed_keys(&mut self, keys: &[String]) {
        self.pressed_keys = keys.iter().cloned().collect();
    }

    /// Check if an action is currently pressed by checking its key bindings.
    fn is_action_pressed(&self, name: &str) -> bool {
        let Some(action) = self.action_map.get_action(name) else {
            return false;
        };
        action.bindings.iter().any(|binding| {
            match &binding.binding_type {
                kiln_input::BindingType::Key(key) => {
                    self.pressed_keys.contains(key)
                }
                kiln_input::BindingType::GamepadButton(btn) => {
                    self.pressed_keys
                        .contains(&format!("GamepadBtn{}", btn))
                }
                kiln_input::BindingType::GamepadAxis { .. } => false,
            }
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
                    if self.pressed_keys.contains(key) {
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
                    if let Some(val_str) = self.pressed_keys.iter().find(|k| {
                        k.starts_with(&key)
                    }) {
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
            }
        }
        value.clamp(-1.0, 1.0)
    }
}

        fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
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
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
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
                        audio.set_volume(volume);
                    }
                }
                Ok(Value::I64(0))
            }
            // Entity management (17-20)
            17 => {
                // spawn_entity(name, x, y, texture_path?)
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("entity").to_string();
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
            18 => {
                // despawn_entity(source_id)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::DespawnEntity(source_id));
                Ok(Value::I64(0))
            }
            19 => {
                // get_entity_pos(source_id) -> (x, y) or null
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some((x, y)) = self.facts.entity_positions.get(source_id) {
                    Ok(Value::Array(vec![Value::F64(*x), Value::F64(*y)]))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            20 => {
                // set_entity_pos(source_id, x, y)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                self.commands.push(SliceCommand::SetEntityPos { source_id, x, y });
                Ok(Value::I64(0))
            }
            // Query builtins (21-25)
            21 => {
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
            22 => {
                // get_entity_component(source_id, component_name) -> value
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let component = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
                match component {
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
            23 => {
                // set_entity_component(source_id, component_name, value)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let component = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let value = args.get(2).cloned().unwrap_or(Value::I64(0));
                self.commands.push(SliceCommand::SetEntityComponent {
                    source_id,
                    component,
                    value,
                });
                Ok(Value::I64(0))
            }
            // World queries (24-25)
            24 => {
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
            25 => {
                // raycast(x, y, dx, dy) -> [source_id, distance] or false
                let x = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let dx = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let dy = args.get(3).and_then(|v| v.to_f64()).unwrap_or(0.0);
                #[cfg(feature = "physics")]
                {
                    if let Some(physics) = &self.physics {
                        if let Some((body_index, distance)) =
                            physics.raycast([x as f32, y as f32], [dx as f32, dy as f32], 100.0)
                        {
                            // Find the entity source_id for this physics body handle
                            for (sid, pb) in self.facts.entity_positions.iter() {
                                // We can't easily map physics handle to source_id here without
                                // storing it in facts. Return distance-based hit info instead.
                                let _ = (sid, pb, body_index, distance);
                            }
                            return Ok(Value::Array(vec![
                                Value::F64(distance as f64),
                                Value::I64(body_index as i64),
                            ]));
                        }
                    }
                }
                #[cfg(not(feature = "physics"))]
                let _ = (x, y, dx, dy);
                Ok(Value::Bool(false))
            }
            // Audio extended (26-27)
            26 => {
                // sound_playing(name) -> bool
                #[cfg(feature = "audio")]
                {
                    let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    let playing = self.audio.as_ref().map(|a| a.is_playing(name)).unwrap_or(false);
                    Ok(Value::Bool(playing))
                }
                #[cfg(not(feature = "audio"))]
                Ok(Value::Bool(false))
            }
            27 => {
                // music_playing() -> bool
                #[cfg(feature = "audio")]
                {
                    let playing = self.audio.as_ref().map(|a| a.is_music_playing()).unwrap_or(false);
                    Ok(Value::Bool(playing))
                }
                #[cfg(not(feature = "audio"))]
                Ok(Value::Bool(false))
            }
            // Time/frame (28-29)
            28 => Ok(Value::I64(self.frame_count as i64)),
            29 => Ok(Value::F64(self.time_scale)),
            // Extended entity API (30-34)
            30 => {
                // set_entity_vel(source_id, vx, vy)
                #[cfg(feature = "physics")]
                {
                    let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let vx = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                    let vy = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                    self.commands.push(SliceCommand::SetEntityVel { source_id, vx, vy });
                }
                Ok(Value::I64(0))
            }
            31 => {
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
            32 => {
                // get_entity_role(source_id) -> role string
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("");
                if let Some(role) = self.facts.entity_roles.get(source_id) {
                    Ok(Value::Str(role.clone()))
                } else {
                    Ok(Value::Str(String::new()))
                }
            }
            33 => {
                // add_component(source_id, component_type, value_json)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let component_type = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let value = args.get(2).cloned().unwrap_or(Value::I64(0));
                self.commands.push(SliceCommand::AddComponent {
                    source_id,
                    component_type,
                    value,
                });
                Ok(Value::I64(0))
            }
            34 => {
                // remove_component(source_id, component_type)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let component_type = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::RemoveComponent {
                    source_id,
                    component_type,
                });
                Ok(Value::I64(0))
            }
            35 => {
                // play_animation(source_id, clip_name)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let clip_name = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::PlayAnimation { source_id, clip_name });
                Ok(Value::I64(0))
            }
            36 => {
                // stop_animation(source_id)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::StopAnimation { source_id });
                Ok(Value::I64(0))
            }
            37 => {
                // set_animation_speed(source_id, speed)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let speed = args.get(1).and_then(|v| v.to_f64()).unwrap_or(1.0);
                self.commands.push(SliceCommand::SetAnimationSpeed { source_id, speed });
                Ok(Value::I64(0))
            }
            38 => {
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
            // UI widgets (39-42)
            39 => {
                // ui_button(id, x, y) -> clicked bool
                let id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                // Button is 64x32 screen pixels (matching the player renderer).
                self.ui_buttons.push((id.clone(), x, y, 64.0, 32.0));
                self.commands.push(SliceCommand::UiButton { id, x, y });
                let clicked = self.ui_widgets.get(&id).copied().unwrap_or(false);
                Ok(Value::Bool(clicked))
            }
            40 => {
                // ui_label(id, x, y, text) -> void
                let id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let text = args.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::UiLabel { id, x, y, text });
                Ok(Value::I64(0))
            }
            41 => {
                // ui_panel(id, x, y, w, h) -> void
                let id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let x = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let w = args.get(3).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let h = args.get(4).and_then(|v| v.to_f64()).unwrap_or(0.0);
                self.commands.push(SliceCommand::UiPanel { id, x, y, w, h });
                Ok(Value::I64(0))
            }
            42 => {
                // ui_clicked(id) -> clicked bool
                let id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let clicked = self.ui_widgets.get(&id).copied().unwrap_or(false);
                Ok(Value::Bool(clicked))
            }
            // Input actions (43-44)
            43 => {
                // get_action(action_name) -> pressed bool
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let pressed = self.is_action_pressed(name);
                Ok(Value::Bool(pressed))
            }
            44 => {
                // get_action_axis(action_name) -> -1 to 1 value
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let axis = self.get_action_axis(name);
                Ok(Value::F64(axis))
            }
            // Localization (45)
            45 => {
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
            // Save/load (46-47)
            46 => {
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
            47 => {
                // load_game(slot_id) -> bool
                let slot = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                self.commands.push(SliceCommand::LoadGame { slot });
                Ok(Value::Bool(true))
            }
            // Animation clip creation (48-50)
            48 => {
                // create_animation_clip(source_id, clip_name, texture_path) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let clip_name = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let texture_path = args.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::CreateAnimationClip {
                    source_id,
                    clip_name,
                    texture_path,
                });
                Ok(Value::I64(0))
            }
            49 => {
                // add_animation_frame(source_id, clip_name, x, y, w, h, duration) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let clip_name = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let x = args.get(2).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let y = args.get(3).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let w = args.get(4).and_then(|v| v.to_f64()).unwrap_or(32.0);
                let h = args.get(5).and_then(|v| v.to_f64()).unwrap_or(32.0);
                let duration = args.get(6).and_then(|v| v.to_f64()).unwrap_or(0.1);
                self.commands.push(SliceCommand::AddAnimationFrame {
                    source_id,
                    clip_name,
                    x,
                    y,
                    w,
                    h,
                    duration,
                });
                Ok(Value::I64(0))
            }
            50 => {
                // set_animation_loop(source_id, clip_name, loops) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let clip_name = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let loops = args.get(2).and_then(|v| v.as_bool()).unwrap_or(true);
                self.commands.push(SliceCommand::SetAnimationLoop {
                    source_id,
                    clip_name,
                    loops,
                });
                Ok(Value::I64(0))
            }
            // Particle system (51-54)
            51 => {
                // create_particle_system(source_id) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::CreateParticleSystem { source_id });
                Ok(Value::I64(0))
            }
            52 => {
                // add_particle_emitter(source_id, rate, speed_min, speed_max, angle_min, angle_max,
                //   lifetime_min, lifetime_max, size_min, size_max, r, g, b, a, gx, gy, fade_out)
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let rate = args.get(1).and_then(|v| v.to_f64()).unwrap_or(10.0);
                let speed_min = args.get(2).and_then(|v| v.to_f64()).unwrap_or(10.0);
                let speed_max = args.get(3).and_then(|v| v.to_f64()).unwrap_or(20.0);
                let angle_min = args.get(4).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let angle_max = args.get(5).and_then(|v| v.to_f64()).unwrap_or(6.28);
                let lifetime_min = args.get(6).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let lifetime_max = args.get(7).and_then(|v| v.to_f64()).unwrap_or(2.0);
                let size_min = args.get(8).and_then(|v| v.to_f64()).unwrap_or(0.5);
                let size_max = args.get(9).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let r = args.get(10).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let g = args.get(11).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let b = args.get(12).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let a = args.get(13).and_then(|v| v.to_f64()).unwrap_or(1.0);
                let gx = args.get(14).and_then(|v| v.to_f64()).unwrap_or(0.0);
                let gy = args.get(15).and_then(|v| v.to_f64()).unwrap_or(-9.8);
                let fade_out = args.get(16).and_then(|v| v.as_bool()).unwrap_or(true);
                self.commands.push(SliceCommand::AddParticleEmitter {
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
                    color: [r, g, b, a],
                    gravity: [gx, gy],
                    fade_out,
                });
                Ok(Value::I64(0))
            }
            53 => {
                // start_particle_system(source_id) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::StartParticleSystem { source_id });
                Ok(Value::I64(0))
            }
            54 => {
                // stop_particle_system(source_id) -> void
                let source_id = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.commands.push(SliceCommand::StopParticleSystem { source_id });
                Ok(Value::I64(0))
            }
            _ => Err(VmError::HostCallError(format!(
                "unknown slice builtin id {func_id}"
            ))),
        }
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
    }
}

/// A running slice: the compiled script plus its state.
pub struct SliceSession {
    program: Program,
    vm: Vm<SliceHost>,
    state: SliceState,
    /// UI commands recorded during the last script run (buttons, labels, panels).
    ui_commands: Vec<SliceCommand>,
}

impl SliceSession {
    /// Compile a slice script.
    pub fn new(source: &str) -> Result<Self, Vec<Diagnostic>> {
        let mut frontend = KilnScriptFrontend::new();
        for (name, id, argc) in BUILTINS {
            frontend = frontend.with_builtin(*name, *id, *argc);
        }
        let program = frontend.compile(source)?;
        let vm = Vm::new(SliceHost::default());
        Ok(Self {
            program,
            vm,
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
        self.state.elapsed += dt;

        // Sync persistent state to host so save_game can access it.
        self.vm.host_mut().state = self.state.clone();

        // Reset per-frame UI click state (clicks only last one frame).
        self.ui_widgets.clear();

        // Step physics and collect collision events (if physics feature enabled)
        let collision_events = #[cfg(feature = "physics")]
        {
            use crate::physics::sync_physics_with_ecs;
            let host = self.vm.host_mut();
            if let Some(physics) = &mut host.physics {
                sync_physics_with_ecs(physics, world.world_mut())
            } else {
                Vec::new()
            }
        };
        #[cfg(not(feature = "physics"))]
        let collision_events: Vec<(String, String, bool)> = Vec::new();

        // Update animations (reads AnimationPlayer, writes Sprite uv_rect)
        crate::sprite::update_animations(world.world_mut());
        // Update particle systems
        crate::sprite::update_particle_systems(world.world_mut(), dt as f32);

        let facts = self.collect_facts(world, input, dt, collision_events);
        {
            let host = self.vm.host_mut();
            host.facts = facts;
            host.take_commands();
        }

        self.vm.execute(&self.program)?;

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

        // Sync host state back to session (e.g., after LoadGame).
        self.state = self.vm.host_mut().state.clone();

        // Detect UI button clicks: check if any button rect contains the mouse
        // and the left button was pressed this frame.
        if self.mouse_pressed {
            for (id, bx, by, bw, bh) in &self.ui_buttons {
                if self.mouse_x >= *bx
                    && self.mouse_x < bx + bw
                    && self.mouse_y >= *by
                    && self.mouse_y < by + bh
                {
                    self.ui_widgets.insert(id.clone(), true);
                }
            }
        }

        // Clear per-frame UI tracking for next frame.
        self.ui_buttons.clear();

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

        use kiln_runtime::{Role, SourceId, Transform, Velocity};
        use kiln_core::project::Component;
        use crate::sprite::Sprite;

        for (source_id, transform) in world.world_ref().query::<(&SourceId, &Transform)>().iter() {
            entity_positions.insert(source_id.0.clone(), (transform.position.x as f64, transform.position.y as f64));
        }

        for (source_id, role) in world.world_ref().query::<(&SourceId, &Role)>().iter() {
            entity_roles.insert(source_id.0.clone(), role.0.clone());
        }

        for (source_id, sprite) in world.world_ref().query::<(&SourceId, &Sprite)>().iter() {
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
            entity_sprite_colors,
            entity_sprite_sizes,
            entity_roles,
            collision_events,
        }
    }

    /// Access the mutable host (for updating frame_count/time_scale).
    fn host_mut(&mut self) -> &mut SliceHost {
        self.vm.host_mut()
    }

    /// Apply one command to the world and the slice state.
    fn apply(&mut self, command: SliceCommand, world: &mut GameWorld) {
        match command {
            SliceCommand::MovePlayer { dx, dy } => {
                if let Some(player) = find_player(world) {
                    let target = crate::tilemap_collision::move_with_collision(
                        world,
                        [player.x, player.y],
                        player.size,
                        [dx as f32, dy as f32],
                    );
                    let x = target[0] as f64;
                    let y = target[1] as f64;
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
            SliceCommand::SpawnEntity { name, x, y, texture_path: _ } => {
                use kiln_runtime::{SourceId, Transform, Sprite, Role};
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
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        let _ = world.world_mut().despawn(eid);
                        break;
                    }
                }
            }
            SliceCommand::SetEntityPos { source_id, x, y } => {
                use kiln_runtime::{SourceId, Transform};
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut transform) = world.world_mut().get_mut::<&mut Transform>(eid) {
                            transform.position.x = x as f32;
                            transform.position.y = y as f32;
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetEntityVel { source_id, vx, vy } => {
                #[cfg(feature = "physics")]
                {
                    use kiln_runtime::{SourceId, Velocity};
                    for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                        if sid.0 == source_id {
                            if let Ok(mut velocity) = world.world_mut().get_mut::<&mut Velocity>(eid) {
                                velocity.0.x = vx as f32;
                                velocity.0.y = vy as f32;
                            }
                            break;
                        }
                    }
                }
            }
            SliceCommand::SetEntityComponent { source_id, component, value } => {
                use kiln_runtime::{SourceId, Transform, Sprite};
                #[cfg(feature = "physics")]
                use kiln_runtime::Velocity;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        match component.as_str() {
                            "transform" | "Transform" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 2 {
                                        if let (Some(Value::F64(x)), Some(Value::F64(y))) = (arr.get(0), arr.get(1)) {
                                            if let Ok(mut transform) = world.world_mut().get_mut::<&mut Transform>(eid) {
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
                                            if let (Some(Value::F64(vx)), Some(Value::F64(vy))) = (arr.get(0), arr.get(1)) {
                                                if let Ok(mut velocity) = world.world_mut().get_mut::<&mut Velocity>(eid) {
                                                    velocity.0.x = *vx as f32;
                                                    velocity.0.y = *vy as f32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "sprite_color" | "SpriteColor" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 4 {
                                        if let (Some(Value::F64(r)), Some(Value::F64(g)), Some(Value::F64(b)), Some(Value::F64(a))) = (arr.get(0), arr.get(1), arr.get(2), arr.get(3)) {
                                            if let Ok(mut sprite) = world.world_mut().get_mut::<&mut Sprite>(eid) {
                                                sprite.color = [*r as f32, *g as f32, *b as f32, *a as f32];
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
                use kiln_runtime::{SourceId, Transform, Sprite};
                #[cfg(feature = "physics")]
                use kiln_runtime::Velocity;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        match component_type.as_str() {
                            "transform" | "Transform" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 2 {
                                        if let (Some(Value::F64(x)), Some(Value::F64(y))) = (arr.get(0), arr.get(1)) {
                                            if let Ok(mut transform) = world.world_mut().get_mut::<&mut Transform>(eid) {
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
                                            if let (Some(Value::F64(vx)), Some(Value::F64(vy))) = (arr.get(0), arr.get(1)) {
                                                if let Ok(mut velocity) = world.world_mut().get_mut::<&mut Velocity>(eid) {
                                                    velocity.0.x = *vx as f32;
                                                    velocity.0.y = *vy as f32;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "sprite" | "Sprite" => {
                                if let Value::Array(arr) = value {
                                    if arr.len() >= 4 {
                                        if let (Some(Value::F64(r)), Some(Value::F64(g)), Some(Value::F64(b)), Some(Value::F64(a))) = (arr.get(0), arr.get(1), arr.get(2), arr.get(3)) {
                                            if let Ok(mut sprite) = world.world_mut().get_mut::<&mut Sprite>(eid) {
                                                sprite.color = [*r as f32, *g as f32, *b as f32, *a as f32];
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
            SliceCommand::RemoveComponent { source_id, component_type } => {
                use kiln_runtime::{SourceId, Transform, Sprite, Velocity};
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
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
            SliceCommand::PlayAnimation { source_id, clip_name } => {
                use crate::sprite::AnimationPlayer;
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
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
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
                            anim.stop();
                        }
                        break;
                    }
                }
            }
            SliceCommand::SetAnimationSpeed { source_id, speed } => {
                use crate::sprite::AnimationPlayer;
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
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
            SliceCommand::UiButton { .. } | SliceCommand::UiLabel { .. } | SliceCommand::UiPanel { .. } => {}
            SliceCommand::LoadGame { slot } => {
                // Load state from disk and apply it to the session + world.
                if let Some(dir) = &self.save_dir {
                    if let Ok(loaded) = load_state(dir, *slot) {
                        self.state = loaded;
                    }
                }
            }
            SliceCommand::CreateAnimationClip {
                source_id,
                clip_name,
                texture_path,
            } => {
                use crate::sprite::AnimationPlayer;
                use kiln_anim::Clip;
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
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
                use kiln_anim::{Clip, Frame};
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
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
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut anim) = world.world_mut().get_mut::<&mut AnimationPlayer>(eid) {
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
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut ps) =
                            world.world_mut().get_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            // Already has a particle system
                        } else {
                            let _ = world.world_mut().insert_one(eid, ParticleSystemComponent::new());
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
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut ps) =
                            world.world_mut().get_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            ps.system_mut().add_emitter(kiln_vfx::Emitter::new(
                                kiln_vfx::EmitterConfig {
                                    shape: kiln_vfx::EmissionShape::Point,
                                    rate: rate as f32,
                                    speed_range: [speed_min as f32, speed_max as f32],
                                    angle_range: [angle_min as f32, angle_max as f32],
                                    lifetime_range: [lifetime_min as f32, lifetime_max as f32],
                                    size_range: [size_min as f32, size_max as f32],
                                    color: [color[0] as f32, color[1] as f32, color[2] as f32, color[3] as f32],
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
                                    color: [color[0] as f32, color[1] as f32, color[2] as f32, color[3] as f32],
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
                // Particle systems emit continuously; no explicit start needed
                let _ = source_id;
            }
            SliceCommand::StopParticleSystem { source_id } => {
                use crate::sprite::ParticleSystemComponent;
                use kiln_runtime::SourceId;
                for (sid, eid) in world.world_ref().query::<&SourceId>().iter() {
                    if sid.0 == source_id {
                        if let Ok(mut ps) =
                            world.world_mut().get_mut::<&mut ParticleSystemComponent>(eid)
                        {
                            // Stop all emitters by setting rate to 0
                            // (No direct stop method; emitters with rate 0 don't emit)
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
        &crate::sprite::Sprite,
    )>() {
        if entity_role.as_str() == role {
            found.push(WorldEntity {
                source_id: source_id.0.clone(),
                x: transform.position.x,
                y: transform.position.y,
                size: sprite.size,
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
