//! Physics module (feature-gated)
//! Uses rapier2d for 2D rigid body physics

#[cfg(feature = "physics")]
use rapier2d::prelude::*;

/// Physics world wrapper
#[cfg(feature = "physics")]
pub struct PhysicsWorld {
    pub(crate) tracked: std::collections::HashMap<hecs::Entity, crate::physics_sync::TrackedBody>,
    pub(crate) source_ids: std::collections::HashMap<RigidBodyHandle, String>,
    pub(crate) active_pairs: std::collections::BTreeSet<(String, String)>,
    pub(crate) tracked_joints:
        std::collections::HashMap<hecs::Entity, (ImpulseJointHandle, crate::world::Joint2D)>,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    gravity: Vec2,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    pub(crate) narrow_phase: NarrowPhase,
    pub joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    soft_body_set: SoftBodySet,
}

impl std::fmt::Debug for PhysicsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicsWorld").finish()
    }
}

#[cfg(feature = "physics")]
impl PhysicsWorld {
    /// Remove a body and its attached colliders and joints.
    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.joint_set,
            &mut self.multibody_joint_set,
            &mut self.soft_body_set,
            true,
        );
    }
    /// Remove a collider and update its parent body.
    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.collider_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            &mut self.soft_body_set,
            true,
        );
    }
    /// Create a new physics world with default gravity.
    pub fn new() -> Self {
        PhysicsWorld {
            tracked: Default::default(),
            source_ids: Default::default(),
            active_pairs: Default::default(),
            tracked_joints: Default::default(),
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
        self.integration_parameters.dt = crate::play_clock::PLAY_STEP as f32;
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
        self.raycast_hit(origin, direction, max_distance, u16::MAX, None, true)
            .map(|hit| (hit.body_index, hit.distance))
    }

    /// Raycast against real collider geometry, with normalized distance and
    /// source identity, optional self exclusion, sensor control, and mask bits.
    pub fn raycast_hit(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        mask: u16,
        exclude: Option<&str>,
        sensors: bool,
    ) -> Option<PhysicsHit> {
        if !origin
            .iter()
            .chain(direction.iter())
            .all(|value| value.is_finite())
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return None;
        }
        let direction = Vec2::new(direction[0], direction[1]).try_normalize()?;
        let filter = |_: ColliderHandle, collider: &Collider| {
            (sensors || !collider.is_sensor())
                && collider.collision_groups().memberships.bits() & mask as u32 != 0
                && exclude.is_none_or(|id| {
                    collider
                        .parent()
                        .and_then(|body| self.source_ids.get(&body))
                        .is_none_or(|source| source != id)
                })
        };
        let query = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.rigid_body_set,
            &self.collider_set,
            QueryFilter::default().predicate(&filter),
        );
        let ray = Ray::new(Vec2::new(origin[0], origin[1]), direction);
        let (handle, hit) = query.cast_ray_and_get_normal(&ray, max_distance, true)?;
        let collider = self.collider_set.get(handle)?;
        let body = collider.parent()?;
        let point = ray.point_at(hit.time_of_impact);
        Some(PhysicsHit {
            source_id: self.source_ids.get(&body).cloned().unwrap_or_default(),
            body_index: body.into_raw_parts().0,
            distance: hit.time_of_impact,
            normal: [hit.normal.x, hit.normal.y],
            point: [point.x, point.y],
        })
    }

    /// Return stable IDs overlapping a circle, using narrow geometric tests.
    pub fn overlap_circle(&self, center: [f32; 2], radius: f32, mask: u16) -> Vec<String> {
        if !radius.is_finite() || radius <= 0.0 || !center.iter().all(|value| value.is_finite()) {
            return Vec::new();
        }
        let filter = |_: ColliderHandle, collider: &Collider| {
            collider.collision_groups().memberships.bits() & mask as u32 != 0
        };
        let query = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.rigid_body_set,
            &self.collider_set,
            QueryFilter::default().predicate(&filter),
        );
        let shape = Ball::new(radius);
        let mut ids: Vec<_> = query
            .intersect_shape(
                rapier2d::math::Pose::new(Vec2::new(center[0], center[1]), 0.0),
                &shape,
            )
            .filter_map(|(_, collider)| {
                collider
                    .parent()
                    .and_then(|body| self.source_ids.get(&body))
                    .cloned()
            })
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

/// Geometry query result independent of Rapier's internal handles.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsHit {
    pub source_id: String,
    pub body_index: u32,
    pub distance: f32,
    pub normal: [f32; 2],
    pub point: [f32; 2],
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
/// Call once per fixed 60 Hz tick. Returns actual enter/exit transitions.
#[cfg(feature = "physics")]
pub fn sync_physics_with_ecs(
    physics: &mut PhysicsWorld,
    world: &mut crate::GameWorld,
) -> Vec<CollisionEvent> {
    crate::physics_sync::synchronize(physics, world)
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
        let body = RigidBodyBuilder::new(rapier2d::prelude::RigidBodyType::Dynamic)
            .translation(Vec2::new(0.0, 10.0));
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
