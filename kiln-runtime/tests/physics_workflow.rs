#![cfg(feature = "physics")]

use kiln_core::Transform;
use kiln_runtime::physics::{sync_physics_with_ecs, PhysicsWorld};
use kiln_runtime::world::{
    Collider2D, ColliderShape, Joint2D, JointType, RigidBody2D, RigidBodyType,
};
use kiln_runtime::{GameWorld, SourceId};

fn body(
    world: &mut GameWorld,
    id: &str,
    position: [f32; 2],
    kind: RigidBodyType,
    collider: Collider2D,
) -> hecs::Entity {
    world.world_mut().spawn((
        SourceId::new(id),
        Transform::new(position[0], position[1]),
        RigidBody2D {
            body_type: kind,
            ..Default::default()
        },
        collider,
    ))
}

fn joint(kind: JointType) -> Joint2D {
    Joint2D {
        entity_a: "anchor".into(),
        entity_b: "bob".into(),
        joint_type: kind,
        anchor_a: [0.0; 2],
        anchor_b: [0.0; 2],
        reference_angle: 0.0,
        length: 2.0,
        lower_translation: -1.0,
        upper_translation: 1.0,
        enable_limit: true,
        enable_motor: false,
        motor_speed: 0.0,
        max_motor_torque: 0.0,
    }
}

#[test]
fn simulated_ancestors_resolve_intermediate_parents_and_joint_anchors_scale() {
    use kiln_runtime::world::{GlobalTransform, ParentLink};
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    physics.set_gravity(0.0, 0.0);
    let collider = Collider2D {
        mask: 0,
        ..Default::default()
    };
    let ancestor = body(
        &mut world,
        "ancestor",
        [0.0; 2],
        RigidBodyType::Kinematic,
        collider,
    );
    world
        .world_mut()
        .get::<&mut RigidBody2D>(ancestor)
        .unwrap()
        .linear_velocity = [6.0, 0.0];
    let middle = world
        .world_mut()
        .spawn((Transform::new(2.0, 0.0), ParentLink(ancestor)));
    let child = body(
        &mut world,
        "child",
        [2.0, 0.0],
        RigidBodyType::Static,
        collider,
    );
    world
        .world_mut()
        .insert_one(child, ParentLink(middle))
        .unwrap();
    for _ in 0..60 {
        sync_physics_with_ecs(&mut physics, &mut world);
    }
    assert!(
        (world
            .world_ref()
            .get::<&GlobalTransform>(child)
            .unwrap()
            .position
            .x
            - 4.0)
            .abs()
            < 0.0001
    );
    assert!(
        (world
            .world_ref()
            .get::<&GlobalTransform>(middle)
            .unwrap()
            .position
            .x
            - 8.0)
            .abs()
            < 0.001
    );

    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    physics.set_gravity(0.0, 0.0);
    let anchor = body(
        &mut world,
        "anchor",
        [0.0; 2],
        RigidBodyType::Static,
        collider,
    );
    world
        .world_mut()
        .get::<&mut Transform>(anchor)
        .unwrap()
        .scale = kiln_core::Vec2::splat(2.0);
    let bob = body(
        &mut world,
        "bob",
        [4.0, 0.0],
        RigidBodyType::Dynamic,
        collider,
    );
    let mut constraint = joint(JointType::Revolute);
    constraint.anchor_a = [2.0, 0.0];
    world.world_mut().insert_one(anchor, constraint).unwrap();
    for _ in 0..60 {
        sync_physics_with_ecs(&mut physics, &mut world);
    }
    assert!((world.world_ref().get::<&Transform>(bob).unwrap().position.x - 4.0).abs() < 0.001);
}

#[test]
fn distance_hinge_and_slider_constraints_hold_and_survive_body_rebuild() {
    for kind in [
        JointType::Distance,
        JointType::Revolute,
        JointType::Prismatic,
    ] {
        let mut world = GameWorld::new();
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(0.0, 0.0);
        let collider = Collider2D {
            mask: 0,
            shape: ColliderShape::Circle { radius: 0.25 },
            ..Default::default()
        };
        let anchor = body(
            &mut world,
            "anchor",
            [0.0; 2],
            RigidBodyType::Static,
            collider,
        );
        let bob = body(
            &mut world,
            "bob",
            [2.0, 0.0],
            RigidBodyType::Dynamic,
            collider,
        );
        world
            .world_mut()
            .insert_one(anchor, joint(kind.clone()))
            .unwrap();
        world
            .world_mut()
            .get::<&mut RigidBody2D>(bob)
            .unwrap()
            .linear_velocity = [5.0, 3.0];
        for _ in 0..180 {
            sync_physics_with_ecs(&mut physics, &mut world);
        }
        let pose = *world.world_ref().get::<&Transform>(bob).unwrap();
        match kind {
            JointType::Distance => assert!(
                (pose.position.length() - 2.0).abs() < 0.03,
                "distance {pose:?}"
            ),
            JointType::Revolute => assert!(pose.position.length() < 0.03, "hinge {pose:?}"),
            JointType::Prismatic => assert!(
                pose.position.y.abs() < 0.03 && pose.position.x.abs() < 1.03,
                "slider {pose:?}"
            ),
        }
        assert_eq!(physics.joint_set.len(), 1);
        world
            .world_mut()
            .get::<&mut Collider2D>(bob)
            .unwrap()
            .density = 2.0;
        sync_physics_with_ecs(&mut physics, &mut world);
        assert_eq!(
            physics.joint_set.len(),
            1,
            "constraint must reconnect after shape/material change"
        );
        world.world_mut().despawn(bob).unwrap();
        sync_physics_with_ecs(&mut physics, &mut world);
        assert_eq!(physics.joint_set.len(), 0);
    }
}

#[test]
fn queries_use_shapes_masks_and_sensor_flags_and_despawn_emits_exit() {
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    let sensor = body(
        &mut world,
        "trigger",
        [0.0; 2],
        RigidBodyType::Static,
        Collider2D {
            shape: ColliderShape::Circle { radius: 1.0 },
            layer: 1,
            mask: 2,
            is_sensor: true,
            ..Default::default()
        },
    );
    body(
        &mut world,
        "bob",
        [0.0; 2],
        RigidBodyType::Kinematic,
        Collider2D {
            shape: ColliderShape::Circle { radius: 0.5 },
            layer: 2,
            mask: 1,
            ..Default::default()
        },
    );
    assert_eq!(sync_physics_with_ecs(&mut physics, &mut world).len(), 1);
    assert_eq!(physics.overlap_circle([0.0, 0.0], 0.1, 1), vec!["trigger"]);
    // Within the AABB corner but outside the actual circle.
    assert!(physics.overlap_circle([0.95, 0.95], 0.01, 1).is_empty());
    assert!(physics
        .raycast_hit([-3.0, 0.0], [1.0, 0.0], 10.0, 1, None, false)
        .is_none());
    assert_eq!(
        physics
            .raycast_hit([-3.0, 0.0], [1.0, 0.0], 10.0, 1, None, true)
            .unwrap()
            .source_id,
        "trigger"
    );
    assert!(physics
        .raycast_hit([-3.0, 0.0], [1.0, 0.0], 1.0, 3, None, true)
        .is_none());
    world.world_mut().despawn(sensor).unwrap();
    let events = sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(events.len(), 1);
    assert!(!events[0].started);
    assert!(sync_physics_with_ecs(&mut physics, &mut world).is_empty());
    // Generational ECS handles must not attach a replacement to a deleted body.
    body(
        &mut world,
        "replacement",
        [20.0, 0.0],
        RigidBodyType::Static,
        Collider2D {
            layer: 1,
            ..Default::default()
        },
    );
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.rigid_body_set.len(), 2);
    assert!(!physics
        .overlap_circle([0.0, 0.0], 0.1, 1)
        .contains(&"trigger".into()));
}

#[test]
fn falling_body_grounds_and_queries_report_geometry_identity_and_normal() {
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    body(
        &mut world,
        "floor",
        [0.0, -1.0],
        RigidBodyType::Static,
        Collider2D {
            shape: ColliderShape::Rectangle {
                half_width: 10.0,
                half_height: 1.0,
            },
            ..Default::default()
        },
    );
    let player = body(
        &mut world,
        "player",
        [0.0, 5.0],
        RigidBodyType::Dynamic,
        Collider2D {
            shape: ColliderShape::Circle { radius: 0.5 },
            ..Default::default()
        },
    );
    let mut entered = false;
    for _ in 0..180 {
        entered |= sync_physics_with_ecs(&mut physics, &mut world)
            .iter()
            .any(|event| event.started && event.entity_a == "floor" && event.entity_b == "player");
    }
    let y = world
        .world_ref()
        .get::<&Transform>(player)
        .unwrap()
        .position
        .y;
    assert!((y - 0.5).abs() < 0.03, "resting y={y}");
    assert!(entered);
    let hit = physics
        .raycast_hit(
            [0.0, 3.0],
            [0.0, -5.0],
            5.0,
            u16::MAX,
            Some("player"),
            false,
        )
        .unwrap();
    assert_eq!(hit.source_id, "floor");
    assert!((hit.distance - 3.0).abs() < 0.01);
    assert!(hit.normal[1] > 0.99);
    assert!(physics
        .overlap_circle([0.0, 0.5], 0.1, u16::MAX)
        .contains(&"player".into()));
    assert!(physics
        .raycast_hit([0.0, 3.0], [0.0, 0.0], 5.0, u16::MAX, None, true)
        .is_none());
}

#[test]
fn sensors_filtering_removal_and_kinematic_velocity_produce_one_transition() {
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    physics.set_gravity(0.0, 0.0);
    let sensor = body(
        &mut world,
        "sensor",
        [0.0, 0.0],
        RigidBodyType::Static,
        Collider2D {
            is_sensor: true,
            layer: 1,
            mask: 2,
            shape: ColliderShape::Circle { radius: 1.0 },
            ..Default::default()
        },
    );
    let mover = body(
        &mut world,
        "mover",
        [0.0, 0.0],
        RigidBodyType::Kinematic,
        Collider2D {
            layer: 2,
            mask: 1,
            shape: ColliderShape::Circle { radius: 0.5 },
            ..Default::default()
        },
    );
    let events = sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(events.iter().filter(|event| event.started).count(), 1);
    assert!(sync_physics_with_ecs(&mut physics, &mut world).is_empty());
    world
        .world_mut()
        .get::<&mut RigidBody2D>(mover)
        .unwrap()
        .linear_velocity = [6.0, 0.0];
    let mut exits = 0;
    for _ in 0..60 {
        exits += sync_physics_with_ecs(&mut physics, &mut world)
            .iter()
            .filter(|event| !event.started)
            .count();
    }
    assert_eq!(exits, 1);
    assert!(
        (world
            .world_ref()
            .get::<&Transform>(mover)
            .unwrap()
            .position
            .x
            - 6.0)
            .abs()
            < 0.02
    );
    world.world_mut().despawn(mover).unwrap();
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.rigid_body_set.len(), 1);
    world.world_mut().remove_one::<Collider2D>(sensor).unwrap();
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.rigid_body_set.len(), 0);
}

#[test]
fn masks_prevent_events_and_joints_follow_component_lifetime() {
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    physics.set_gravity(0.0, 0.0);
    let a = body(
        &mut world,
        "a",
        [0.0, 0.0],
        RigidBodyType::Static,
        Collider2D {
            layer: 1,
            mask: 1,
            is_sensor: true,
            ..Default::default()
        },
    );
    body(
        &mut world,
        "b",
        [0.0, 0.0],
        RigidBodyType::Dynamic,
        Collider2D {
            layer: 2,
            mask: 2,
            ..Default::default()
        },
    );
    assert!(sync_physics_with_ecs(&mut physics, &mut world).is_empty());
    world
        .world_mut()
        .insert_one(
            a,
            Joint2D {
                entity_a: "a".into(),
                entity_b: "b".into(),
                joint_type: JointType::Revolute,
                anchor_a: [0.0; 2],
                anchor_b: [0.0; 2],
                reference_angle: 0.0,
                length: 1.0,
                lower_translation: -1.0,
                upper_translation: 1.0,
                enable_limit: false,
                motor_speed: 0.0,
                max_motor_torque: 0.0,
                enable_motor: false,
            },
        )
        .unwrap();
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.joint_set.len(), 1);
    world.world_mut().remove_one::<Joint2D>(a).unwrap();
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.joint_set.len(), 0);
}

#[test]
fn fixed_tick_results_match_at_thirty_and_one_forty_four_render_frames() {
    fn simulate(fps: usize) -> [f32; 2] {
        let mut world = GameWorld::new();
        let mut physics = PhysicsWorld::new();
        body(
            &mut world,
            "floor",
            [0.0, -1.0],
            RigidBodyType::Static,
            Collider2D {
                shape: ColliderShape::Rectangle {
                    half_width: 20.0,
                    half_height: 1.0,
                },
                ..Default::default()
            },
        );
        let player = body(
            &mut world,
            "player",
            [0.0, 4.0],
            RigidBodyType::Dynamic,
            Collider2D {
                shape: ColliderShape::Circle { radius: 0.5 },
                ..Default::default()
            },
        );
        world
            .world_mut()
            .get::<&mut RigidBody2D>(player)
            .unwrap()
            .linear_velocity = [3.0, 0.0];
        let mut clock = kiln_runtime::play_clock::PlayClock::default();
        for _ in 0..fps * 2 {
            for _ in 0..clock.advance(1.0 / fps as f64) {
                sync_physics_with_ecs(&mut physics, &mut world);
            }
        }
        let position = world
            .world_ref()
            .get::<&Transform>(player)
            .unwrap()
            .position
            .to_array();
        position
    }
    let a = simulate(30);
    let b = simulate(144);
    assert!(
        (a[0] - b[0]).abs() < 0.0001 && (a[1] - b[1]).abs() < 0.0001,
        "{a:?} {b:?}"
    );
}

#[test]
fn rotated_slopes_and_parent_local_transforms_use_world_geometry() {
    use kiln_runtime::world::ParentLink;
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    physics.set_gravity(0.0, 0.0);
    let parent = world.world_mut().spawn((Transform::new(10.0, 0.0)
        .with_rotation(std::f32::consts::FRAC_PI_2)
        .with_scale(kiln_core::Vec2::splat(2.0)),));
    let child = body(
        &mut world,
        "child",
        [1.0, 0.0],
        RigidBodyType::Static,
        Collider2D {
            shape: ColliderShape::Circle { radius: 0.5 },
            ..Default::default()
        },
    );
    world
        .world_mut()
        .insert_one(child, ParentLink(parent))
        .unwrap();
    let slope = body(
        &mut world,
        "slope",
        [0.0, 0.0],
        RigidBodyType::Static,
        Collider2D {
            shape: ColliderShape::Rectangle {
                half_width: 5.0,
                half_height: 0.5,
            },
            ..Default::default()
        },
    );
    world
        .world_mut()
        .get::<&mut Transform>(slope)
        .unwrap()
        .rotation = 0.25;
    for _ in 0..3 {
        sync_physics_with_ecs(&mut physics, &mut world);
    }
    let hit = physics
        .raycast_hit([0.0, 5.0], [0.0, -1.0], 10.0, u16::MAX, None, false)
        .unwrap();
    assert_eq!(hit.source_id, "slope");
    assert!((hit.normal[1] - 0.25_f32.cos()).abs() < 0.001);
    assert!(
        (world
            .world_ref()
            .get::<&Transform>(child)
            .unwrap()
            .position
            .x
            - 1.0)
            .abs()
            < 0.001
    );
    world
        .world_mut()
        .get::<&mut Transform>(parent)
        .unwrap()
        .position
        .x = 20.0;
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(
        physics
            .raycast_hit([20.0, 10.0], [0.0, -1.0], 10.0, u16::MAX, None, false)
            .unwrap()
            .source_id,
        "child"
    );
}

#[test]
fn moving_platform_carries_a_grounded_dynamic_body() {
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    let platform = body(
        &mut world,
        "platform",
        [0.0, 0.0],
        RigidBodyType::Kinematic,
        Collider2D {
            shape: ColliderShape::Rectangle {
                half_width: 5.0,
                half_height: 0.5,
            },
            friction: 2.0,
            ..Default::default()
        },
    );
    let player = body(
        &mut world,
        "player",
        [0.0, 1.0],
        RigidBodyType::Dynamic,
        Collider2D {
            shape: ColliderShape::Rectangle {
                half_width: 0.5,
                half_height: 0.5,
            },
            friction: 2.0,
            ..Default::default()
        },
    );
    for _ in 0..30 {
        sync_physics_with_ecs(&mut physics, &mut world);
    }
    world
        .world_mut()
        .get::<&mut RigidBody2D>(platform)
        .unwrap()
        .linear_velocity = [2.0, 0.0];
    for _ in 0..120 {
        sync_physics_with_ecs(&mut physics, &mut world);
    }
    let pose = *world.world_ref().get::<&Transform>(player).unwrap();
    assert!(
        pose.position.x > 2.0 && (pose.position.y - 1.0).abs() < 0.1,
        "{pose:?}"
    );
}

#[test]
fn authored_physics_drives_real_script_sensor_callbacks_and_queries() {
    use kiln_core::{Component, Entity, Project, Scene};
    let mut project = Project::default();
    let authored = |id: &str, y: f32, kind: RigidBodyType, sensor: bool| Entity {
        id: id.into(),
        name: id.into(),
        components: vec![
            Component::Transform {
                x: 0.0,
                y,
                rotation: 0.0,
                scale: 1.0,
            },
            Component::Physics {
                body: RigidBody2D {
                    body_type: kind,
                    ..Default::default()
                },
                collider: Collider2D {
                    is_sensor: sensor,
                    shape: ColliderShape::Rectangle {
                        half_width: 2.0,
                        half_height: 0.5,
                    },
                    ..Default::default()
                },
            },
        ],
    };
    let mut mover = authored("mover", 0.0, RigidBodyType::Dynamic, false);
    mover.components.push(Component::Script {
        path: "probe.ks".into(),
        properties: Default::default(),
    });
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                authored("ground", -3.0, RigidBodyType::Static, false),
                authored("sensor", 0.0, RigidBodyType::Static, true),
                mover,
            ],
        },
    );
    let mut world = GameWorld::new();
    kiln_runtime::scene::spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
        kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3])
    });
    let mut session=kiln_runtime::SliceSession::with_scene_sources(&project,"main",|_| Ok(r#"if script_phase() == "collision_enter" { state_set("entered",event_entity()); }
if script_phase() == "update" { let hit=raycast(0.0,5.0,0.0,-1.0); state_set("ground",array_get(hit,0)); }"#.into())).unwrap();
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("mover", "entered"),
        Some(&kiln_ir::Value::Str("sensor".into()))
    );
    assert_eq!(
        session.entity_script_state("mover", "ground"),
        Some(&kiln_ir::Value::Str("ground".into()))
    );
}

#[test]
fn editor_sample_runs_from_source_and_player_assets_with_identical_fixed_tick_results() {
    use kiln_core::Project;
    use kiln_ir::Value;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/physics2d_editor");
    let project =
        Project::from_toml(&std::fs::read_to_string(root.join("kiln.toml")).unwrap()).unwrap();
    fn run(
        project: &Project,
        read: impl Fn(&str) -> Result<String, String>,
        fps: usize,
    ) -> (kiln_core::Transform, Value, [f32; 2]) {
        let mut world = GameWorld::new();
        kiln_runtime::scene::spawn_scene(&mut world, project, "main", &mut |_, color, _| {
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3])
        });
        let mut session =
            kiln_runtime::SliceSession::with_scene_sources(project, "main", read).unwrap();
        let mut clock = kiln_runtime::play_clock::PlayClock::default();
        let mut controller = kiln_runtime::camera::CameraController::default();
        let mut camera = controller
            .update(&project.cameras["main"], &world, [640.0, 360.0], 0.0)
            .unwrap();
        let mut tick = 0;
        let mut jumped = false;
        for _ in 0..fps * 7 {
            for _ in 0..clock.advance(1.0 / fps as f64) {
                // Wait for grounding, jump once, then walk past the sensor and
                // the excluded layer. This exercises the checked-in scripts.
                let axis = if tick < 60 {
                    (0.0, 0.0)
                } else if tick == 60 {
                    (1.0, 1.0)
                } else {
                    (1.0, 0.0)
                };
                let point = camera
                    .screen_to_world([320.0, 180.0], [0.0, 0.0, 640.0, 360.0])
                    .unwrap();
                session
                    .host_mut()
                    .set_mouse_world(point.map(|value| value as f64));
                session.step(&mut world, axis, 1.0 / 60.0).unwrap();
                camera = controller
                    .update(&project.cameras["main"], &world, [640.0, 360.0], 1.0 / 60.0)
                    .unwrap();
                let pose = world
                    .world_ref()
                    .query::<(&SourceId, &Transform)>()
                    .iter()
                    .find(|(source, _)| source.0 == "hero")
                    .unwrap()
                    .1
                    .position;
                if tick > 60 && tick < 90 && pose.y > 70.0 {
                    jumped = true;
                }
                tick += 1;
            }
        }
        assert!(jumped);
        let pose = *world
            .world_ref()
            .query::<(&SourceId, &Transform)>()
            .iter()
            .find(|(source, _)| source.0 == "hero")
            .unwrap()
            .1;
        assert!(pose.position.x > 900.0, "{pose:?}");
        assert!((pose.position.y - 32.0).abs() < 0.2, "{pose:?}");
        assert_eq!(
            session.entity_script_state("hero", "trigger"),
            Some(&Value::Bool(true))
        );
        assert!(
            matches!(session.entity_script_state("hero", "mouse_world"), Some(Value::Array(values)) if values.len() == 2)
        );
        assert!(camera.position[0] <= 760.0 && camera.position[1] == 0.0);
        (
            pose,
            session
                .entity_script_state("hero", "trigger")
                .unwrap()
                .clone(),
            camera.position,
        )
    }
    let before = project.to_toml().unwrap();
    let source = run(
        &project,
        |path| std::fs::read_to_string(root.join(path)).map_err(|error| error.to_string()),
        30,
    );
    let reopened = Project::from_toml(&before).unwrap();
    let assets = std::collections::HashMap::from([
        (
            "scripts/hero.ks",
            std::fs::read(root.join("scripts/hero.ks")).unwrap(),
        ),
        (
            "scripts/platform.ks",
            std::fs::read(root.join("scripts/platform.ks")).unwrap(),
        ),
    ]);
    let player = run(
        &reopened,
        |path| String::from_utf8(assets[path].clone()).map_err(|error| error.to_string()),
        144,
    );
    assert_eq!(source, player);
    assert_eq!(project.to_toml().unwrap(), before);
}

#[test]
fn painted_solid_tiles_ground_real_bodies_and_queries_follow_map_edits() {
    use kiln_core::{TileMap, TileMapLayer, TileSet};
    let mut world = GameWorld::new();
    let mut physics = PhysicsWorld::new();
    let mut data = TileMap {
        tile_size: [1.0; 2],
        ..Default::default()
    };
    let mut layer = TileMapLayer {
        tileset: "ground".into(),
        ..Default::default()
    };
    for x in 0..8 {
        layer.set_tile_at(x, 0, 0);
    }
    data.layers.push(layer);
    let map = kiln_runtime::Tilemap::new(
        "level".into(),
        data,
        std::collections::HashMap::from([(
            "ground".into(),
            TileSet {
                solid: vec![true],
                ..Default::default()
            },
        )]),
        Default::default(),
    );
    let floor =
        world
            .world_mut()
            .spawn((SourceId::new("tile-ground"), Transform::new(10.0, 0.0), map));
    let player = body(
        &mut world,
        "hero",
        [10.5, 4.0],
        RigidBodyType::Dynamic,
        Collider2D {
            shape: ColliderShape::Circle { radius: 0.5 },
            ..Default::default()
        },
    );
    let mut entered = false;
    for _ in 0..180 {
        entered |= sync_physics_with_ecs(&mut physics, &mut world)
            .iter()
            .any(|event| {
                event.started
                    && (event.entity_a == "tile-ground" || event.entity_b == "tile-ground")
            });
    }
    assert!(entered);
    assert!(
        (world
            .world_ref()
            .get::<&Transform>(player)
            .unwrap()
            .position
            .y
            - 1.5)
            .abs()
            < 0.03
    );
    let hit = physics
        .raycast_hit([10.5, 3.0], [0.0, -1.0], 5.0, 1, Some("hero"), false)
        .unwrap();
    assert_eq!(hit.source_id, "tile-ground");
    assert!((hit.distance - 2.0).abs() < 0.01);
    world
        .world_mut()
        .get::<&mut Transform>(floor)
        .unwrap()
        .position
        .x = 20.0;
    assert!(sync_physics_with_ecs(&mut physics, &mut world)
        .iter()
        .any(|event| !event.started));
    assert!(physics
        .raycast_hit([20.5, 3.0], [0.0, -1.0], 5.0, 1, Some("hero"), false)
        .is_some());
    world
        .world_mut()
        .get::<&mut kiln_runtime::Tilemap>(floor)
        .unwrap()
        .map
        .layers[0]
        .set_tile_at(0, 0, kiln_core::EMPTY_TILE);
    sync_physics_with_ecs(&mut physics, &mut world);
    assert!(physics
        .raycast_hit([20.5, 3.0], [0.0, -1.0], 5.0, 1, Some("hero"), false)
        .is_none());
    world.world_mut().despawn(floor).unwrap();
    sync_physics_with_ecs(&mut physics, &mut world);
    assert_eq!(physics.rigid_body_set.len(), 1);
}
