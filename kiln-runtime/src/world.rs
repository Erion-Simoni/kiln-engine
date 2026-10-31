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

/// 2D rigid body physics component.
///
/// When the `physics` feature is enabled, this component is synced with a
/// rapier2d rigid body each frame. When disabled, it is a plain data holder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidBody2D {
    /// Body type: static, dynamic, or kinematic.
    pub body_type: RigidBodyType,
    /// Linear velocity (world units per second).
    pub linear_velocity: [f32; 2],
    /// Angular velocity (radians per second).
    pub angular_velocity: f32,
    /// Gravity scale multiplier (1.0 = normal gravity).
    pub gravity_scale: f32,
    /// Whether the body can sleep.
    pub can_sleep: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigidBodyType {
    Static,
    Dynamic,
    Kinematic,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            linear_velocity: [0.0, 0.0],
            angular_velocity: 0.0,
            gravity_scale: 1.0,
            can_sleep: true,
        }
    }
}

/// 2D collider component.
///
/// Defines the collision shape attached to a rigid body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider2D {
    /// Collider shape.
    pub shape: ColliderShape,
    /// Whether the collider is a sensor (triggers events but no physical response).
    pub is_sensor: bool,
    /// Friction coefficient (0.0 = frictionless, 1.0 = high friction).
    pub friction: f32,
    /// Restitution / bounciness (0.0 = no bounce, 1.0 = super bouncy).
    pub restitution: f32,
    /// Density (used for mass calculation of dynamic bodies).
    pub density: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderShape {
    /// A circle with the given radius.
    Circle { radius: f32 },
    /// A rectangle with the given half-extents.
    Rectangle { half_width: f32, half_height: f32 },
}

/// Handle linking an ECS entity to a rapier2d rigid body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsBody {
    /// rapier2d rigid body handle index.
    pub handle: u32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Rectangle {
                half_width: 16.0,
                half_height: 16.0,
            },
            is_sensor: false,
            friction: 0.5,
            restitution: 0.0,
            density: 1.0,
        }
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
