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

use std::path::Path;

use kiln_ir::{HostContext, Program, Value, Vm, VmError};
use kiln_script::{Diagnostic, KilnScriptFrontend, LanguageFrontend};
use kiln_save::slot::SaveSlotError;
use kiln_save::SaveSlotManager;
use serde::{Deserialize, Serialize};

use crate::world::{GameWorld, Role, SourceId};

/// The builtin functions a slice script can call.
///
/// Each entry is the function name, its host-call id and its argument count.
pub const BUILTINS: &[(&str, u32, u32)] = &[
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
];

/// Distance (in world units) at which a pickup counts as collected.
pub const PICKUP_RADIUS: f32 = 24.0;

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
}

impl SliceHost {
    /// A host with the given facts.
    pub fn new(facts: SliceFacts) -> Self {
        Self {
            facts,
            commands: Vec::new(),
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

impl HostContext for SliceHost {
    fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
        match func_id {
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

        let facts = self.collect_facts(world, input);
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
    fn collect_facts(&mut self, world: &mut GameWorld, input: (f64, f64)) -> SliceFacts {
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

        SliceFacts {
            input_x: input.0,
            input_y: input.1,
            elapsed: self.state.elapsed,
            score: self.state.score,
            coin_count: remaining,
            coin_in_reach,
            status: self.state.status.clone(),
        }
    }

    /// Apply one command to the world and the slice state.
    fn apply(&mut self, command: SliceCommand, world: &mut GameWorld) {
        match command {
            SliceCommand::MovePlayer { dx, dy } => {
                if let Some(player) = find_player(world) {
                    let x = player.x as f64 + dx;
                    let y = player.y as f64 + dy;
                    set_player_position(world, &player.source_id, x, y);
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
}

/// Find the player entity, if the world has one.
fn find_player(world: &mut GameWorld) -> Option<WorldEntity> {
    find_entities(world, ROLE_PLAYER).into_iter().next()
}

/// Find all entities with the given role.
fn find_entities(world: &mut GameWorld, role: &str) -> Vec<WorldEntity> {
    let mut found = Vec::new();
    for (source_id, entity_role, transform) in
        world
            .world_mut()
            .query_mut::<(&SourceId, &Role, &kiln_core::Transform)>()
    {
        if entity_role.as_str() == role {
            found.push(WorldEntity {
                source_id: source_id.0.clone(),
                x: transform.position.x,
                y: transform.position.y,
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
    fn unknown_function_is_a_compile_error() {
        assert!(SliceSession::new("not_a_function();").is_err());
    }
}
