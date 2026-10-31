//! Physics module (feature-gated)
//! Uses rapier2d for 2D rigid body physics

use std::collections::HashMap;

#[cfg(feature = "physics")]
use rapier2d::prelude::*;

/// Physics world wrapper
#[cfg(feature = "physics")]
pub struct PhysicsWorld {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    gravity: Vec2,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    soft_body_set: SoftBodySet,
}

#[cfg(feature = "physics")]
impl PhysicsWorld {
    /// Create a new physics world with default gravity
    pub fn new() -> Self {
        PhysicsWorld {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: Vec2::new(0.0, -9.81),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            soft_body_set: SoftBodySet::new(),
        }
    }

    /// Set gravity
    pub fn set_gravity(&mut self, x: f32, y: f32) {
        self.gravity = Vec2::new(x, y);
    }

    /// Create a rigid body
    pub fn create_rigid_body(&mut self, body: RigidBodyBuilder) -> RigidBodyHandle {
        self.rigid_body_set.insert(body)
    }

    /// Create a collider attached to a rigid body
    pub fn create_collider(
        &mut self,
        collider: ColliderBuilder,
        parent: RigidBodyHandle,
    ) -> ColliderHandle {
        self.collider_set
            .insert_with_parent(collider, parent, &mut self.rigid_body_set)
    }

    /// Step the physics simulation
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.joint_set,
            &mut self.multibody_joint_set,
            &mut self.soft_body_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    /// Get rigid body position
    pub fn get_rigid_body_position(&self, handle: RigidBodyHandle) -> Option<Vec2> {
        self.rigid_body_set
            .get(handle)
            .map(|body| body.translation())
    }

    /// Set rigid body velocity
    pub fn set_rigid_body_velocity(&mut self, handle: RigidBodyHandle, velocity: Vec2) -> bool {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_linvel(velocity, true);
            true
        } else {
            false
        }
    }

    /// Raycast from origin in direction, returning the first hit body index and distance.
    pub fn raycast(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
    ) -> Option<(u32, f32)> {
        use rapier2d::math::Vector;
        let origin = Vector::new(origin[0], origin[1]);
        let dir = Vector::new(direction[0], direction[1]).normalize();
        let mut hit_info = None;
        let mut max_toi = max_distance;

        let _ = self.physics_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            origin,
            dir,
            max_toi,
            true,
            rapier2d::prelude::QueryFilter::default(),
            &mut |handle, toi| {
                if toi < max_toi {
                    max_toi = toi;
                    hit_info = Some((handle.index() as u32, toi));
                }
            },
        );

        hit_info
    }
}

#[cfg(feature = "physics")]
impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "physics"))]
/// No-op physics world stub when physics feature is disabled
pub struct PhysicsWorld;

#[cfg(not(feature = "physics"))]
impl PhysicsWorld {
    pub fn new() -> Self {
        PhysicsWorld
    }

    pub fn set_gravity(&mut self, _x: f32, _y: f32) {}

    pub fn step(&mut self) {}
}

#[cfg(not(feature = "physics"))]
impl Default for PhysicsWorld {
    fn default() -> Self {
        PhysicsWorld
    }
}

/// Sync physics bodies with ECS transforms.
///
/// Call this once per frame after `PhysicsWorld::step()`. It:
/// 1. Creates rapier2d bodies for ECS entities that have `RigidBody2D` +
///    `Collider2D` but no `PhysicsBody` handle yet.
/// 2. Steps the physics simulation.
/// 3. Writes physics body positions back to ECS `Transform` components.
#[cfg(feature = "physics")]
pub fn sync_physics_with_ecs(
    physics: &mut PhysicsWorld,
    world: &mut hecs::World,
) {
    use crate::world::{
        Collider2D, ColliderShape, PhysicsBody, RigidBody2D, RigidBodyType, SourceId, Transform,
    };

    // Register new bodies: entities with RigidBody2D + Collider2D but no PhysicsBody
    let to_register: Vec<(hecs::Entity, RigidBody2D, Collider2D, Option<SourceId>)> = world
        .query::<(&RigidBody2D, &Collider2D, Option<&SourceId>)>()
        .iter()
        .filter_map(|(e, (rb, col, sid))| {
            if world.get::<PhysicsBody>(e).is_err() {
                Some((e, *rb, *col, sid.cloned()))
            } else {
                None
            }
        })
        .collect();

    for (entity, rb, col, source_id) in to_register {
        let body_builder = match rb.body_type {
            RigidBodyType::Static => rapier2d::prelude::RigidBodyBuilder::static(),
            RigidBodyType::Dynamic => rapier2d::prelude::RigidBodyBuilder::dynamic(),
            RigidBodyType::Kinematic => rapier2d::prelude::RigidBodyBuilder::kinematic(),
        }
        .translation(rapier2d::math::Vector::new(
            rb.linear_velocity[0],
            rb.linear_velocity[1],
        ))
        .linear_velocity(rapier2d::math::Vector::new(rb.linear_velocity[0], rb.linear_velocity[1]))
        .angular_velocity(rb.angular_velocity)
        .gravity_scale(rb.gravity_scale)
        .can_sleep(rb.can_sleep);

        let handle = physics.create_rigid_body(body_builder);

        let collider_builder = match col.shape {
            ColliderShape::Circle { radius } => rapier2d::prelude::ColliderBuilder::ball(radius)
                .friction(col.friction)
                .restitution(col.restitution)
                .density(col.density)
                .sensor(col.is_sensor),
            ColliderShape::Rectangle {
                half_width,
                half_height,
            } => rapier2d::prelude::ColliderBuilder::cuboid(half_width, half_height)
                .friction(col.friction)
                .restitution(col.restitution)
                .density(col.density)
                .sensor(col.is_sensor),
        };

        physics.create_collider(collider_builder, handle);

        let _ = world.insert_one(entity, PhysicsBody { handle: handle.index() as u32 });
    }

/// A collision event between two physics bodies.
#[derive(Debug, Clone)]
pub struct CollisionEvent {
    /// Source id of the first entity.
    pub entity_a: String,
    /// Source id of the second entity.
    pub entity_b: String,
    /// Whether the collision just started (true) or ended (false).
    pub started: bool,
}

/// Sync physics bodies with ECS transforms and collect collision events.
///
/// Call this once per frame. Returns collision events detected this frame.
#[cfg(feature = "physics")]
pub fn sync_physics_with_ecs(
    physics: &mut PhysicsWorld,
    world: &mut hecs::World,
) -> Vec<CollisionEvent> {
    use crate::world::{
        Collider2D, ColliderShape, PhysicsBody, RigidBody2D, RigidBodyType, SourceId, Transform,
    };

    // Register new bodies: entities with RigidBody2D + Collider2D but no PhysicsBody
    let to_register: Vec<(hecs::Entity, RigidBody2D, Collider2D, Option<SourceId>)> = world
        .query::<(&RigidBody2D, &Collider2D, Option<&SourceId>)>()
        .iter()
        .filter_map(|(e, (rb, col, sid))| {
            if world.get::<PhysicsBody>(e).is_err() {
                Some((e, *rb, *col, sid.cloned()))
            } else {
                None
            }
        })
        .collect();

    for (entity, rb, col, source_id) in to_register {
        let body_builder = match rb.body_type {
            RigidBodyType::Static => rapier2d::prelude::RigidBodyBuilder::static(),
            RigidBodyType::Dynamic => rapier2d::prelude::RigidBodyBuilder::dynamic(),
            RigidBodyType::Kinematic => rapier2d::prelude::RigidBodyBuilder::kinematic(),
        }
        .translation(rapier2d::math::Vector::new(
            rb.linear_velocity[0],
            rb.linear_velocity[1],
        ))
        .linear_velocity(rapier2d::math::Vector::new(rb.linear_velocity[0], rb.linear_velocity[1]))
        .angular_velocity(rb.angular_velocity)
        .gravity_scale(rb.gravity_scale)
        .can_sleep(rb.can_sleep);

        let handle = physics.create_rigid_body(body_builder);

        let collider_builder = match col.shape {
            ColliderShape::Circle { radius } => rapier2d::prelude::ColliderBuilder::ball(radius)
                .friction(col.friction)
                .restitution(col.restitution)
                .density(col.density)
                .sensor(col.is_sensor),
            ColliderShape::Rectangle {
                half_width,
                half_height,
            } => rapier2d::prelude::ColliderBuilder::cuboid(half_width, half_height)
                .friction(col.friction)
                .restitution(col.restitution)
                .density(col.density)
                .sensor(col.is_sensor),
        };

        physics.create_collider(collider_builder, handle);

        let _ = world.insert_one(entity, PhysicsBody { handle: handle.index() as u32 });
    }

    // Build a map from rapier2d rigid body handle index -> source id
    let handle_to_source: std::collections::HashMap<u32, String> = world
        .query::<(&PhysicsBody, &SourceId)>()
        .iter()
        .map(|(_, (pb, sid))| (pb.handle, sid.0.clone()))
        .collect();

    // Step the simulation
    physics.step();

    // Collect collision events from the narrow phase
    let mut collision_events = Vec::new();
    let _ = physics.narrow_phase.contact_events().for_each(|event| {
        use rapier2d::prelude::ContactEvent;
        match event {
            ContactEvent::Started(h1, h2) => {
                let a = handle_to_source.get(&(h1.index() as u32)).cloned();
                let b = handle_to_source.get(&(h2.index() as u32)).cloned();
                if let (Some(a), Some(b)) = (a, b) {
                    collision_events.push(CollisionEvent {
                        entity_a: a,
                        entity_b: b,
                        started: true,
                    });
                }
            }
            ContactEvent::Stopped(h1, h2) => {
                let a = handle_to_source.get(&(h1.index() as u32)).cloned();
                let b = handle_to_source.get(&(h2.index() as u32)).cloned();
                if let (Some(a), Some(b)) = (a, b) {
                    collision_events.push(CollisionEvent {
                        entity_a: a,
                        entity_b: b,
                        started: false,
                    });
                }
            }
        }
    });

    // Sync physics positions back to ECS transforms
    for (entity, physics_body, transform) in
        world.query_mut::<(&mut PhysicsBody, &mut Transform)>().iter()
    {
        if let Some(body) = physics
            .rigid_body_set
            .get(rapier2d::prelude::RigidBodyHandle::from_index(physics_body.handle as usize))
        {
            let pos = body.translation();
            transform.position.x = pos.x;
            transform.position.y = pos.y;
        }
    }

    collision_events
}

#[cfg(test)]
#[cfg(feature = "physics")]
mod tests {
    use super::*;

    #[test]
    fn test_physics_world_creation() {
        let world = PhysicsWorld::new();
        assert_eq!(world.gravity.y, -9.81);
    }

    #[test]
    fn test_rigid_body_creation() {
        let mut world = PhysicsWorld::new();
        let body = RigidBodyBuilder::dynamic().translation(Vec2::new(0.0, 10.0));
        let handle = world.create_rigid_body(body);
        let position = world.get_rigid_body_position(handle);
        assert!(position.is_some());
    }

    #[test]
    fn test_physics_step() {
        let mut world = PhysicsWorld::new();
        let body = RigidBodyBuilder::dynamic().translation(Vec2::new(0.0, 10.0));
        let handle = world.create_rigid_body(body);
        let collider = ColliderBuilder::ball(1.0);
        world.create_collider(collider, handle);

        world.step();
        world.step();

        let position = world.get_rigid_body_position(handle);
        assert!(position.is_some());
        // Body should have fallen due to gravity
        assert!(position.unwrap().y < 10.0);
    }
}
