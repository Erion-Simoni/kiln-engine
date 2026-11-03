//! Sample-specific runtime code for the vertical slice game.
//!
//! This module contains game-specific logic that was previously mixed into
//! the general-purpose `slice` module. A real game would create its own
//! module like this, tailored to its needs.
//!
//! The general-purpose runtime in `slice.rs` provides:
//! - Generic entities and components
//! - Generic component registration
//! - Entity creation and destruction at runtime
//! - Entity lookup by stable ID
//! - Component access from scripts
//! - Event and message systems
//! - Runtime scene management
//! - A shared host-function registry
//! - Script lifecycle events
//! - Generic state management

use crate::sprite::Sprite;
use crate::world::{GameWorld, Role, SourceId};
use kiln_ir::Value;
use kiln_ir::VmError;
use serde::{Deserialize, Serialize};

/// Sample-specific roles.
pub const ROLE_PLAYER: &str = "player";
pub const ROLE_COIN: &str = "coin";

/// Distance at which a pickup counts as collected.
pub const PICKUP_RADIUS: f32 = 24.0;

/// Sample-specific state tracked by the slice runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SliceSampleState {
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

impl SliceSampleState {
    /// Whether the pickup with this source id has been collected.
    pub fn is_collected(&self, source_id: &str) -> bool {
        self.collected.iter().any(|id| id == source_id)
    }
}

/// Sample-specific commands.
#[derive(Debug, Clone, PartialEq)]
pub enum SliceSampleCommand {
    /// Move the player by the given delta.
    MovePlayer { dx: f64, dy: f64 },
    /// Add to the score.
    AddScore(i64),
    /// Collect a pickup with the given source id.
    CollectCoin(String),
}

/// Sample-specific builtin function IDs (4-8).
pub const SAMPLE_BUILTINS: &[(&str, u32, u32)] = &[
    ("get_score", 4, 0),
    ("add_score", 5, 1),
    ("get_coin_count", 6, 0),
    ("collect_coin", 7, 0),
    ("move_player", 8, 2),
];

/// Sample-specific host call handler.
pub fn call_host_sample(
    func_id: u32,
    args: &[Value],
    state: &mut SliceSampleState,
    commands: &mut Vec<SliceSampleCommand>,
) -> Result<Value, VmError> {
    if func_id == 4 {
        Ok(Value::I64(state.score))
    } else if func_id == 5 {
        let amount = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
        state.score += amount;
        Ok(Value::I64(state.score))
    } else if func_id == 6 {
        Ok(Value::I64(state.collected.len() as i64))
    } else if func_id == 7 {
        match state.collected.last().cloned() {
            Some(id) => {
                commands.push(SliceSampleCommand::CollectCoin(id.clone()));
                Ok(Value::Bool(true))
            }
            None => Ok(Value::Bool(false)),
        }
    } else if func_id == 8 {
        let dx = args.first().and_then(|v| v.to_f64()).unwrap_or(0.0);
        let dy = args.get(1).and_then(|v| v.to_f64()).unwrap_or(0.0);
        commands.push(SliceSampleCommand::MovePlayer { dx, dy });
        Ok(Value::I64(0))
    } else {
        Err(VmError::HostCallError(format!(
            "unknown slice builtin id {func_id}"
        )))
    }
}

/// Find the player entity, if the world has one.
pub fn find_player(world: &mut GameWorld) -> Option<WorldEntity> {
    find_entities(world, ROLE_PLAYER).into_iter().next()
}

/// Find all entities with the given role.
pub fn find_entities(world: &mut GameWorld, role: &str) -> Vec<WorldEntity> {
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
pub fn find_coins(world: &mut GameWorld) -> Vec<WorldEntity> {
    find_entities(world, ROLE_COIN)
}

/// Spawn the player entity.
pub fn spawn_player(world: &mut GameWorld, x: f32, y: f32) {
    use crate::{Role, SourceId};
    use kiln_core::Transform;
    let source_id = crate::world::next_entity_id(&[]);
    world.world_mut().spawn((
        Transform::new(x, y),
        Sprite::new(1.0, 1.0, 1.0, 1.0)
            .with_size(32.0, 32.0)
            .with_layer(0.0),
        SourceId::new(source_id.clone()),
        Role::new("player"),
    ));
}

/// Spawn a pickup entity.
pub fn spawn_coin(world: &mut GameWorld, source_id: &str, x: f32, y: f32) {
    use crate::{Role, SourceId};
    use kiln_core::Transform;
    world.world_mut().spawn((
        Transform::new(x, y),
        Sprite::new(1.0, 1.0, 1.0, 1.0)
            .with_size(16.0, 16.0)
            .with_layer(0.0),
        SourceId::new(source_id.to_string()),
        Role::new("coin"),
    ));
}

/// Remove the pickup with the given source id from the world.
pub fn despawn_coin(world: &mut GameWorld, source_id: &str) {
    let target = {
        let mut target = None;
        for (eid, sid) in world.world_mut().query_mut::<(hecs::Entity, &SourceId)>() {
            if sid.0 == source_id {
                target = Some(eid);
                break;
            }
        }
        target
    };
    if let Some(eid) = target {
        let _ = world.world_mut().despawn(eid);
    }
}

/// The id of the next entity, counted from one.
pub fn next_entity_id(existing: &[String]) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("entity_{index}");
        if !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// A sample-specific world entity.
#[derive(Debug)]
pub struct WorldEntity {
    pub source_id: String,
    pub x: f32,
    pub y: f32,
    pub size: [f32; 2],
}

/// HUD lines for the slice state.
pub fn hud_lines(state: &SliceSampleState) -> Vec<String> {
    let mut lines = vec![format!("SCORE {}", state.score)];
    if !state.status.is_empty() {
        lines.push(state.status.clone());
    }
    lines.push("ARROWS MOVE   F5 SAVE   F9 LOAD".to_string());
    lines
}
