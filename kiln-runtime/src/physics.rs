//! Physics module (feature-gated)
//! Uses rapier2d for 2D rigid body physics

#[cfg(feature = "physics")]
use rapier2d::prelude::*;
use std::collections::HashMap;

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
    named_bodies: HashMap<String, RigidBodyHandle>,
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
            named_bodies: HashMap::new(),
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

    /// Register a named body for lookup by name
    pub fn register_body(&mut self, entity_id: &str, handle: RigidBodyHandle) {
        self.named_bodies.insert(entity_id.to_string(), handle);
    }

    /// Apply force to a named rigid body
    pub fn apply_force(&mut self, entity_id: &str, fx: f32, fy: f32) {
        if let Some(handle) = self.named_bodies.get(entity_id) {
            if let Some(body) = self.rigid_body_set.get_mut(*handle) {
                body.apply_force(Vec2::new(fx, fy), true);
            }
        }
    }

    /// Raycast from (x1, y1) to (x2, y2). Returns true if anything is hit.
    pub fn raycast(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
        let origin = Point2::new(x1, y1);
        let direction = Vector2::new(x2 - x1, y2 - y1);
        let distance = direction.norm();
        if distance == 0.0 {
            return false;
        }
        let direction = direction / distance;
        let ray = Ray::new(origin, direction);
        let mut hit = false;
        self.physics_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            ray,
            distance,
            true,
            QueryFilter::default(),
            |_hit| {
                hit = true;
                false
            },
        );
        hit
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
