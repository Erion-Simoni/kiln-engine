//! ECS world wrapper

use hecs::World;
use kiln_core::{EntityId, Transform};
use std::ops::Deref;

/// The resolved world-space transform produced from local Transform and Parent links.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform(pub Transform);

impl Deref for GlobalTransform {
    type Target = Transform;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Runtime link to a parent entity in the same scene hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentLink(pub hecs::Entity);

/// Fixed scene-instance offset applied to a hierarchy root in world space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorldOffset(pub [f32; 2]);

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

pub use kiln_core::physics_data::{
    Collider2D, ColliderShape, Joint2D, JointType, RigidBody2D, RigidBodyType,
};

/// Handle linking an ECS entity to a rapier2d rigid body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsBody {
    /// rapier2d rigid body handle index.
    pub handle: u32,
    /// rapier2d arena generation for [`Self::handle`].
    pub generation: u32,
}

/// Linear velocity in world units per second.
#[derive(Debug, Clone, Copy, PartialEq, Default, hecs::Bundle)]
pub struct Velocity(pub [f32; 2]);

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

/// The id of the next entity, counted from one.
///
/// The ids are unique within a scene, so the editor can name an entity and
/// find it again after a reload.
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
