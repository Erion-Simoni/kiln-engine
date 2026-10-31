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

        // Update animations (reads AnimationPlayer, writes Sprite uv_rect)
        crate::sprite::update_animations(world.world_mut());

        let facts = self.collect_facts(world, input, dt);
        {
            let host = self.vm.host_mut();
            host.facts = facts;
            host.take_commands();
        }

        self.vm.execute(&self.program)?;

        let commands = self.vm.host_mut().take_commands();
        for command in commands {
            self.apply(command, world);
        }
        Ok(())
    }

    /// Gather the facts the script can read this frame.
    fn collect_facts(&mut self, world: &mut GameWorld, input: (f64, f64), dt: f64) -> SliceFacts {
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
                let source_id = crate::editing::next_entity_id(&[]);
                world.world_mut().spawn((
                    Transform::new(x as f32, y as f32),
                    Sprite::new(1.0, 1.0, 1.0, 1.0).with_size(32.0, 32.0),
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
