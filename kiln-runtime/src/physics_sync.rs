//! ECS/Rapier synchronization, lifetime tracking, joints and contact transitions.

use crate::physics::{CollisionEvent, PhysicsWorld};
use crate::world::{
    Collider2D, ColliderShape, GameWorld, GlobalTransform, Joint2D, JointType, ParentLink,
    PhysicsBody, RigidBody2D, RigidBodyType as BodyType, SourceId, WorldOffset,
};
use kiln_core::{Transform, Vec2};
use rapier2d::prelude::*;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(crate) struct TrackedBody {
    handle: RigidBodyHandle,
    body: RigidBody2D,
    collider: Collider2D,
    pose: Transform,
}

/// Synchronize one fixed tick and report sorted enter/exit transitions.
pub(crate) fn synchronize(physics: &mut PhysicsWorld, game: &mut GameWorld) -> Vec<CollisionEvent> {
    if crate::scene::update_global_transforms(game).is_err() {
        return Vec::new();
    }
    let snapshots: Vec<_> = game
        .world_ref()
        .query::<(
            hecs::Entity,
            &RigidBody2D,
            &Collider2D,
            &GlobalTransform,
            Option<&SourceId>,
        )>()
        .iter()
        .map(|(entity, body, collider, pose, id)| {
            (
                entity,
                *body,
                *collider,
                pose.0,
                id.map(|id| id.0.clone())
                    .unwrap_or_else(|| format!("{entity:?}")),
            )
        })
        .collect();
    let live: HashSet<_> = snapshots.iter().map(|snapshot| snapshot.0).collect();
    let removed: Vec<_> = physics
        .tracked
        .keys()
        .filter(|entity| !live.contains(entity))
        .copied()
        .collect();
    for entity in removed {
        if let Some(tracked) = physics.tracked.remove(&entity) {
            physics.source_ids.remove(&tracked.handle);
            physics.remove_body(tracked.handle);
            let _ = game.world_mut().remove_one::<PhysicsBody>(entity);
        }
    }
    for (entity, body, collider, pose, source) in snapshots {
        let valid = pose.position.is_finite()
            && pose.rotation.is_finite()
            && pose.scale.is_finite()
            && pose.scale.abs().min_element() > 0.000001
            && kiln_core::physics_data::validate_body(&body, &collider).is_ok();
        if !valid {
            if let Some(tracked) = physics.tracked.remove(&entity) {
                physics.source_ids.remove(&tracked.handle);
                physics.remove_body(tracked.handle);
                let _ = game.world_mut().remove_one::<PhysicsBody>(entity);
            }
            continue;
        }
        let rebuild = physics.tracked.get(&entity).is_some_and(|tracked| {
            tracked.body.body_type != body.body_type
                || tracked.body.can_sleep != body.can_sleep
                || tracked.collider != collider
                || tracked.pose.scale != pose.scale
                || physics.rigid_body_set.get(tracked.handle).is_none()
        });
        if rebuild {
            if let Some(tracked) = physics.tracked.remove(&entity) {
                physics.source_ids.remove(&tracked.handle);
                physics.remove_body(tracked.handle);
            }
        }
        if !physics.tracked.contains_key(&entity) {
            let builder = match body.body_type {
                BodyType::Static => RigidBodyBuilder::fixed(),
                BodyType::Dynamic => RigidBodyBuilder::dynamic(),
                BodyType::Kinematic => RigidBodyBuilder::kinematic_position_based(),
            }
            .translation(rapier2d::math::Vector::new(
                pose.position.x,
                pose.position.y,
            ))
            .rotation(pose.rotation)
            .linvel(rapier2d::math::Vector::new(
                body.linear_velocity[0],
                body.linear_velocity[1],
            ))
            .angvel(body.angular_velocity)
            .gravity_scale(body.gravity_scale)
            .can_sleep(body.can_sleep)
            .ccd_enabled(true);
            let handle = physics.create_rigid_body(builder);
            let scale = pose.scale.abs();
            let shape = match collider.shape {
                ColliderShape::Circle { radius } => {
                    ColliderBuilder::ball(radius * scale.x.max(scale.y))
                }
                ColliderShape::Rectangle {
                    half_width,
                    half_height,
                } => ColliderBuilder::cuboid(half_width * scale.x, half_height * scale.y),
            };
            let groups = InteractionGroups::new(
                Group::from_bits_truncate(collider.layer as u32),
                Group::from_bits_truncate(collider.mask as u32),
                InteractionTestMode::And,
            );
            physics.create_collider(
                shape
                    .friction(collider.friction)
                    .restitution(collider.restitution)
                    .density(collider.density)
                    .sensor(collider.is_sensor)
                    .collision_groups(groups)
                    .active_collision_types(ActiveCollisionTypes::all()),
                handle,
            );
            physics.tracked.insert(
                entity,
                TrackedBody {
                    handle,
                    body,
                    collider,
                    pose,
                },
            );
            physics.source_ids.insert(handle, source);
            let (handle, generation) = handle.into_raw_parts();
            let _ = game
                .world_mut()
                .insert_one(entity, PhysicsBody { handle, generation });
        }
        let Some(tracked) = physics.tracked.get_mut(&entity) else {
            continue;
        };
        if let Some(physical) = physics.rigid_body_set.get_mut(tracked.handle) {
            if body.gravity_scale != tracked.body.gravity_scale {
                physical.set_gravity_scale(body.gravity_scale, true);
            }
            if body.linear_velocity != tracked.body.linear_velocity {
                physical.set_linvel(
                    rapier2d::math::Vector::new(body.linear_velocity[0], body.linear_velocity[1]),
                    true,
                );
            }
            if body.angular_velocity != tracked.body.angular_velocity {
                physical.set_angvel(body.angular_velocity, true);
            }
            let changed =
                pose.position != tracked.pose.position || pose.rotation != tracked.pose.rotation;
            if body.body_type == BodyType::Kinematic {
                let position = if changed {
                    pose.position
                } else {
                    pose.position
                        + Vec2::from_array(body.linear_velocity)
                            * crate::play_clock::PLAY_STEP as f32
                };
                let rotation = if changed {
                    pose.rotation
                } else {
                    pose.rotation + body.angular_velocity * crate::play_clock::PLAY_STEP as f32
                };
                physical.set_next_kinematic_position(rapier2d::math::Pose::new(
                    rapier2d::math::Vector::new(position.x, position.y),
                    rotation,
                ));
            } else if changed {
                physical.set_position(
                    rapier2d::math::Pose::new(
                        rapier2d::math::Vector::new(pose.position.x, pose.position.y),
                        pose.rotation,
                    ),
                    true,
                );
            }
            tracked.body = body;
        }
    }
    crate::physics_tilemap::synchronize(physics, game);
    synchronize_joints(physics, game);
    physics.step();
    let mut pairs = BTreeSet::new();
    let mut record = |a: ColliderHandle, b: ColliderHandle| {
        let Some(a) = physics
            .collider_set
            .get(a)
            .and_then(|collider| collider.parent())
            .and_then(|handle| physics.source_ids.get(&handle))
        else {
            return;
        };
        let Some(b) = physics
            .collider_set
            .get(b)
            .and_then(|collider| collider.parent())
            .and_then(|handle| physics.source_ids.get(&handle))
        else {
            return;
        };
        pairs.insert(if a < b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        });
    };
    for pair in physics.narrow_phase.contact_pairs() {
        if pair.has_any_active_contact() {
            record(pair.collider1, pair.collider2);
        }
    }
    for (a, b, intersects) in physics.narrow_phase.intersection_pairs() {
        if intersects {
            record(a, b);
        }
    }
    let mut events: Vec<_> = pairs
        .difference(&physics.active_pairs)
        .map(|(a, b)| CollisionEvent {
            entity_a: a.clone(),
            entity_b: b.clone(),
            started: true,
        })
        .collect();
    events.extend(
        physics
            .active_pairs
            .difference(&pairs)
            .map(|(a, b)| CollisionEvent {
                entity_a: a.clone(),
                entity_b: b.clone(),
                started: false,
            }),
    );
    physics.active_pairs = pairs;
    let poses: HashMap<_, _> = physics
        .tracked
        .iter()
        .filter_map(|(entity, tracked)| {
            physics.rigid_body_set.get(tracked.handle).map(|body| {
                let mut pose = tracked.pose;
                pose.position = Vec2::new(body.translation().x, body.translation().y);
                pose.rotation = body.rotation().angle();
                (*entity, pose)
            })
        })
        .collect();
    // Resolve non-physical intermediate parents using the simulated poses of
    // their ancestors before converting bodies back into authored local space.
    let locals = game
        .world_ref()
        .query::<(hecs::Entity, &Transform)>()
        .iter()
        .map(|(entity, pose)| (entity, *pose))
        .collect();
    let parents = game
        .world_ref()
        .query::<(hecs::Entity, &ParentLink)>()
        .iter()
        .map(|(entity, parent)| (entity, parent.0))
        .collect();
    let offsets = game
        .world_ref()
        .query::<(hecs::Entity, &WorldOffset)>()
        .iter()
        .map(|(entity, offset)| (entity, offset.0))
        .collect();
    let mut resolved = poses.clone();
    for (&entity, &pose) in &poses {
        let parent = game
            .world_ref()
            .get::<&ParentLink>(entity)
            .ok()
            .map(|parent| parent.0);
        let mut local = pose;
        if let Some(parent) = parent {
            let parent_pose = crate::scene::resolve_runtime_transform(
                parent,
                &locals,
                &parents,
                &offsets,
                &mut resolved,
                &mut Vec::new(),
            )
            .ok();
            if let Some(parent_pose) = parent_pose {
                local.position = (pose.position - parent_pose.position)
                    .rotate(Vec2::from_angle(-parent_pose.rotation))
                    / parent_pose.scale;
                local.rotation = pose.rotation - parent_pose.rotation;
            }
        } else if let Ok(offset) = game.world_ref().get::<&WorldOffset>(entity) {
            local.position -= Vec2::from_array(offset.0);
        }
        if let Ok(mut transform) = game.world_mut().get::<&mut Transform>(entity) {
            transform.position = local.position;
            transform.rotation = local.rotation;
        }
        if let Some(tracked) = physics.tracked.get_mut(&entity) {
            tracked.pose = pose;
            if let Some(physical) = physics.rigid_body_set.get(tracked.handle) {
                let velocity = [physical.linvel().x, physical.linvel().y];
                if let Ok(mut body) = game.world_mut().get::<&mut RigidBody2D>(entity) {
                    body.linear_velocity = velocity;
                    body.angular_velocity = physical.angvel();
                    tracked.body = *body;
                }
            }
        }
    }
    let _ = crate::scene::update_global_transforms(game);
    events.sort_by(|a, b| {
        (&a.entity_a, &a.entity_b, a.started).cmp(&(&b.entity_a, &b.entity_b, b.started))
    });
    events
}

fn synchronize_joints(physics: &mut PhysicsWorld, game: &GameWorld) {
    let joints: HashMap<_, _> = game
        .world_ref()
        .query::<(hecs::Entity, &Joint2D)>()
        .iter()
        .map(|(entity, joint)| (entity, joint.clone()))
        .collect();
    let stale: Vec<_> = physics
        .tracked_joints
        .iter()
        .filter(|(entity, (handle, config))| {
            joints.get(entity) != Some(config) || physics.joint_set.get(*handle).is_none()
        })
        .map(|(entity, _)| *entity)
        .collect();
    for entity in stale {
        if let Some((handle, _)) = physics.tracked_joints.remove(&entity) {
            physics.joint_set.remove(handle, true);
        }
    }
    let ids: HashMap<_, _> = physics
        .source_ids
        .iter()
        .map(|(handle, id)| (id.as_str(), *handle))
        .collect();
    for (entity, config) in joints {
        if kiln_core::physics_data::validate_joint(&config).is_err() {
            continue;
        }
        if physics.tracked_joints.contains_key(&entity) {
            continue;
        }
        let (Some(&a), Some(&b)) = (
            ids.get(config.entity_a.as_str()),
            ids.get(config.entity_b.as_str()),
        ) else {
            continue;
        };
        if a == b {
            continue;
        }
        let scale = |handle| {
            physics
                .tracked
                .values()
                .find(|body| body.handle == handle)
                .map(|body| body.pose.scale)
                .unwrap_or(Vec2::ONE)
        };
        let anchor_a = Vec2::from_array(config.anchor_a) * scale(a);
        let anchor_b = Vec2::from_array(config.anchor_b) * scale(b);
        let anchor_a = rapier2d::math::Vector::new(anchor_a.x, anchor_a.y);
        let anchor_b = rapier2d::math::Vector::new(anchor_b.x, anchor_b.y);
        let joint: GenericJoint = match config.joint_type {
            JointType::Distance => GenericJointBuilder::new(JointAxesMask::empty())
                .coupled_axes(JointAxesMask::LIN_AXES)
                .local_anchor1(anchor_a)
                .local_anchor2(anchor_b)
                .limits(JointAxis::LinX, [config.length.max(0.001); 2])
                .build(),
            JointType::Revolute => {
                let mut builder = RevoluteJointBuilder::new()
                    .local_anchor1(anchor_a)
                    .local_anchor2(anchor_b);
                if config.enable_motor {
                    builder = builder
                        .motor_velocity(config.motor_speed, 1.0)
                        .motor_max_force(config.max_motor_torque);
                }
                builder.build().into()
            }
            JointType::Prismatic => {
                let mut builder = PrismaticJointBuilder::new(rapier2d::math::Vector::new(
                    config.reference_angle.cos(),
                    config.reference_angle.sin(),
                ))
                .local_anchor1(anchor_a)
                .local_anchor2(anchor_b);
                if config.enable_limit {
                    builder = builder.limits([config.lower_translation, config.upper_translation]);
                }
                if config.enable_motor {
                    builder = builder
                        .motor_velocity(config.motor_speed, 1.0)
                        .motor_max_force(config.max_motor_torque);
                }
                builder.build().into()
            }
        };
        let handle = physics.joint_set.insert(a, b, joint, true);
        physics.tracked_joints.insert(entity, (handle, config));
    }
}
