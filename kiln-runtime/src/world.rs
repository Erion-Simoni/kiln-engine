//! ECS world wrapper

use hecs::World;
use kiln_core::EntityId;

/// The gameplay role of an entity, e.g. `"player"` or `"coin"`.
///
/// Roles let a scene's script find entities by purpose instead of by index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role(pub String);

impl Role {
    /// A role with the given name.
    pub fn new(role: impl Into<String>) -> Self {
        Self(role.into())
    }

    /// The role name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The stable id of the scene entity an entity was spawned from.
///
/// The id survives save/load, so a save file can refer to the same entity
/// after the world is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceId(pub String);

impl SourceId {
    /// A source id with the given value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The source id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct GameWorld {
    world: World,
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    pub fn world(&mut self) -> &mut World {
        &mut self.world
    }

    /// Read-only access to the world.
    pub fn world_ref(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn spawn<T>(&mut self, components: T) -> EntityId
    where
        T: hecs::Bundle,
    {
        let entity = self.world.spawn(components);
        EntityId::new(entity.id() as u64)
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
