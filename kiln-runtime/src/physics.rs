//! Physics module (feature-gated)
//! Uses rapier2d for 2D rigid body physics

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
        self.collider_set.insert_with_parent(collider, parent, &mut self.rigid_body_set)
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
        self.rigid_body_set.get(handle).map(|body| body.translation())
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
