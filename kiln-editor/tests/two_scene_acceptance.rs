//! Combined two-scene authoring, hierarchy, history, and disk-restart acceptance.
use kiln_core::{
    hierarchy_transform::world_transforms, Component, Entity, Project, Scene, Transform, Vec2,
};
use kiln_editor::{
    entity_command::{EntityCommand, EntityEdit},
    project_files,
    scene_document::snapshot_for_scene,
    scene_workspace::SceneWorkspace,
    transform_edit::{SceneTransaction, SetTransform},
};
use kiln_runtime::{GameWorld, SourceId, Sprite};

fn scene(id: &str, actor_x: f32) -> Scene {
    Scene {
        id: id.into(),
        name: format!("{id} level"),
        script: Some(format!("scripts/{id}.ks")),
        nested_scenes: vec![],
        entities: vec![
            Entity {
                id: "actor".into(),
                name: "Actor".into(),
                components: vec![
                    Component::Transform {
                        x: actor_x,
                        y: 20.0,
                        rotation: 0.25,
                        scale: 1.5,
                    },
                    Component::Sprite {
                        color: [0.2, 0.4, 0.8, 1.0],
                        width: 48.0,
                        height: 24.0,
                        texture: Some("sprites/actor.png".into()),
                        layer: 7.0,
                        material: Some("ink".into()),
                        animation_clip: Some("idle".into()),
                    },
                    Component::Role {
                        role: "player".into(),
                    },
                ],
            },
            Entity {
                id: "child".into(),
                name: "Child".into(),
                components: vec![
                    Component::Transform {
                        x: 3.0,
                        y: 4.0,
                        rotation: -0.1,
                        scale: 1.0,
                    },
                    Component::Parent {
                        parent: "actor".into(),
                    },
                ],
            },
            Entity {
                id: "anchor".into(),
                name: "Anchor".into(),
                components: vec![Component::Transform {
                    x: 100.0,
                    y: 50.0,
                    rotation: -0.2,
                    scale: 2.0,
                }],
            },
        ],
    }
}

fn project() -> Project {
    let mut project = Project::default();
    project.scenes.insert("alpha".into(), scene("alpha", 10.0));
    project.scenes.insert("beta".into(), scene("beta", 80.0));
    project
}

fn spawn(project: &Project, scene_id: &str) -> GameWorld {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        scene_id,
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    kiln_runtime::spawn_tilemaps(
        &mut world,
        project,
        &project.scenes[scene_id],
        Default::default(),
        &mut |_| None,
    );
    world
}

fn snapshot(project: &Project, world: &GameWorld, scene_id: &str) -> Scene {
    snapshot_for_scene(
        world,
        &Default::default(),
        &Default::default(),
        Some(project),
        Some(scene_id),
    )
}

fn set_transform(scene: &mut Scene, world: &mut GameWorld, entity_id: &str, value: Transform) {
    let scene_id = scene.id.clone();
    SetTransform {
        scene_id: &scene_id,
        entity_id,
        value,
    }
    .apply(scene, world)
    .unwrap();
}

fn entity_edit(
    project: &mut Project,
    history: &mut kiln_editor::SceneHistory,
    edit: EntityEdit,
) -> Option<String> {
    let mut scene = project.scenes["alpha"].clone();
    let result = EntityCommand {
        scene_id: "alpha".into(),
        edit,
    }
    .execute(&mut scene, project, history)
    .unwrap();
    project.scenes.insert("alpha".into(), scene);
    result
}

#[test]
fn two_scene_transform_hierarchy_and_restart_preserve_authored_documents() {
    let mut project = project();
    let mut workspace = SceneWorkspace::open(&project);
    assert_eq!(workspace.active_id(), Some("alpha"));

    let mut world = spawn(&project, "alpha");
    let alpha_before = snapshot(&project, &world, "alpha");
    let mut alpha_after = alpha_before.clone();
    set_transform(
        &mut alpha_after,
        &mut world,
        "actor",
        Transform::new(24.0, 36.0)
            .with_rotation(0.75)
            .with_scale(Vec2::splat(2.0)),
    );
    let mut alpha_edit = SceneTransaction::new(alpha_before, "actor".into());
    alpha_edit.update(alpha_after.clone());
    alpha_edit.commit(&mut workspace.history);
    project.scenes.insert("alpha".into(), alpha_after);

    let alpha_current = snapshot(&project, &world, "alpha");
    workspace.switch(&mut project, alpha_current, 1).unwrap();
    let mut world = spawn(&project, "beta");
    let beta_before = snapshot(&project, &world, "beta");
    let mut beta_after = beta_before.clone();
    set_transform(
        &mut beta_after,
        &mut world,
        "actor",
        Transform::new(120.0, 90.0),
    );
    let mut beta_edit = SceneTransaction::new(beta_before, "actor".into());
    beta_edit.update(beta_after.clone());
    beta_edit.commit(&mut workspace.history);
    project.scenes.insert("beta".into(), beta_after);
    let beta_current = snapshot(&project, &world, "beta");
    workspace.switch(&mut project, beta_current, 0).unwrap();
    assert_eq!(workspace.history.undo_depth(), 1);

    let child_before = world_transforms(&project.scenes["alpha"]).unwrap()["child"];
    entity_edit(
        &mut project,
        &mut workspace.history,
        EntityEdit::Reparent {
            entity_id: "child".into(),
            parent_id: Some("anchor".into()),
        },
    );
    let child_after = world_transforms(&project.scenes["alpha"]).unwrap()["child"];
    assert!((child_after.position - child_before.position).length() < 0.0001);
    assert!((child_after.rotation - child_before.rotation).abs() < 0.0001);
    assert!((child_after.scale - child_before.scale).length() < 0.0001);

    let duplicate = entity_edit(
        &mut project,
        &mut workspace.history,
        EntityEdit::Duplicate {
            entity_id: "actor".into(),
        },
    )
    .unwrap();
    entity_edit(
        &mut project,
        &mut workspace.history,
        EntityEdit::Delete {
            entity_id: duplicate.clone(),
        },
    );
    assert_eq!(workspace.history.undo_depth(), 4);
    let mut alpha = project.scenes["alpha"].clone();
    alpha = workspace.history.undo(alpha).unwrap(); // restore deletion
    assert!(alpha.entities.iter().any(|entity| entity.id == duplicate));
    alpha = workspace.history.undo(alpha).unwrap(); // undo duplication
    assert!(!alpha.entities.iter().any(|entity| entity.id == duplicate));
    alpha = workspace.history.undo(alpha).unwrap(); // undo reparent
    assert!(matches!(
        alpha.entities.iter().find(|entity| entity.id == "child").unwrap().components.iter()
            .find(|component| matches!(component, Component::Parent { .. })),
        Some(Component::Parent { parent }) if parent == "actor"
    ));
    alpha = workspace.history.undo(alpha).unwrap(); // undo transform transaction
    assert_eq!(
        world_transforms(&alpha).unwrap()["actor"].position,
        Vec2::new(10.0, 20.0)
    );
    assert_eq!(
        alpha
            .entities
            .iter()
            .find(|entity| entity.id == "actor")
            .unwrap()
            .name,
        "Actor"
    );
    assert!(!workspace.history.can_undo());

    alpha = workspace.history.redo(alpha).unwrap(); // redo transform
    assert_eq!(
        world_transforms(&alpha).unwrap()["actor"].position,
        Vec2::new(24.0, 36.0)
    );
    alpha = workspace.history.redo(alpha).unwrap(); // redo reparent
    assert!(matches!(
        alpha.entities.iter().find(|entity| entity.id == "child").unwrap().components.iter()
            .find(|component| matches!(component, Component::Parent { .. })),
        Some(Component::Parent { parent }) if parent == "anchor"
    ));
    alpha = workspace.history.redo(alpha).unwrap(); // redo duplicate
    assert!(alpha.entities.iter().any(|entity| entity.id == duplicate));
    alpha = workspace.history.redo(alpha).unwrap(); // redo delete
    assert!(!alpha.entities.iter().any(|entity| entity.id == duplicate));
    project.scenes.insert("alpha".into(), alpha);

    // The other scene's edit remains isolated and independently undoable.
    let alpha_current = project.scenes["alpha"].clone();
    workspace.switch(&mut project, alpha_current, 1).unwrap();
    assert_eq!(workspace.history.undo_depth(), 1);
    let beta_undone = workspace
        .history
        .undo(project.scenes["beta"].clone())
        .unwrap();
    assert_eq!(
        world_transforms(&beta_undone).unwrap()["actor"].position,
        Vec2::new(80.0, 20.0)
    );
    let beta_redone = workspace.history.redo(beta_undone).unwrap();
    assert_eq!(
        world_transforms(&beta_redone).unwrap()["actor"].position,
        Vec2::new(120.0, 90.0)
    );
    project.scenes.insert("beta".into(), beta_redone);

    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    for id in ["alpha", "beta"] {
        assert_eq!(
            serde_json::to_value(&reopened.scenes[id]).unwrap(),
            serde_json::to_value(&project.scenes[id]).unwrap()
        );
    }
    let reopened_workspace = SceneWorkspace::open(&reopened);
    let reopened_world = spawn(&reopened, reopened_workspace.active_id().unwrap());
    let runtime_actor = *reopened_world
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &Transform)>()
        .iter()
        .find(|(_, source, _)| source.0 == "actor")
        .unwrap()
        .2;
    assert_eq!(runtime_actor.position, Vec2::new(24.0, 36.0));
    assert!((runtime_actor.rotation - 0.75).abs() < 0.0001);
    assert_eq!(runtime_actor.scale, Vec2::splat(2.0));
    let expected_child = world_transforms(&reopened.scenes["alpha"]).unwrap()["child"];
    let runtime_child = reopened_world
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &kiln_runtime::GlobalTransform)>()
        .iter()
        .find(|(_, source, _)| source.0 == "child")
        .unwrap()
        .2
         .0;
    assert!((runtime_child.position - expected_child.position).length() < 0.0001);
    assert!((runtime_child.rotation - expected_child.rotation).abs() < 0.0001);
}

fn rich_project() -> Project {
    use kiln_core::{
        physics_data::{Collider2D, Joint2D, JointType, RigidBody2D},
        prefab::{Prefab, PrefabOverride},
        tilemap::TileMap,
        visual::{ParticleEffect, SpriteLayout},
    };
    let mut project = project();
    project.tilemaps.insert(
        "terrain".into(),
        TileMap {
            id: "terrain".into(),
            name: "Authored terrain".into(),
            tile_size: [24.0, 12.0],
            ..Default::default()
        },
    );
    for scene in project.scenes.values_mut() {
        let actor = &mut scene.entities[0];
        actor.components.extend([
            Component::EditorFlags {
                locked: false,
                hidden: true,
            },
            Component::SpriteLayout {
                settings: SpriteLayout {
                    pivot: [0.25, 0.75],
                    flip_x: true,
                    flip_y: true,
                    pixels_per_unit: 2.0,
                },
            },
            Component::Particles {
                effect: ParticleEffect {
                    max_particles: 37,
                    playing: false,
                    layer: 4.5,
                    ..Default::default()
                },
            },
            Component::Physics {
                body: RigidBody2D {
                    linear_velocity: [3.0, -2.0],
                    angular_velocity: 0.5,
                    gravity_scale: 0.25,
                    can_sleep: false,
                    ..Default::default()
                },
                collider: Collider2D {
                    is_sensor: true,
                    friction: 0.25,
                    restitution: 0.75,
                    density: 2.0,
                    layer: 3,
                    mask: 7,
                    ..Default::default()
                },
            },
            Component::Script {
                path: "scripts/actor.ks".into(),
                properties: [("speed".into(), 12.5), ("health".into(), 87.0)].into(),
            },
            Component::Joint {
                joint: Joint2D {
                    entity_a: "actor".into(),
                    entity_b: "child".into(),
                    joint_type: JointType::Prismatic,
                    anchor_a: [1.0, 2.0],
                    anchor_b: [-3.0, 4.0],
                    reference_angle: 0.5,
                    length: 24.0,
                    lower_translation: -8.0,
                    upper_translation: 12.0,
                    enable_limit: true,
                    motor_speed: 3.5,
                    max_motor_torque: 19.0,
                    enable_motor: true,
                },
            },
        ]);
        let prefab_id = format!("{}-actor", scene.id);
        project.prefabs.insert(
            prefab_id.clone(),
            Prefab {
                id: prefab_id.clone(),
                name: "Authored actor".into(),
                root_entity: actor.clone(),
            },
        );
        actor.components.push(Component::PrefabInstance {
            prefab_id,
            overrides: PrefabOverride {
                entity_id: actor.id.clone(),
                ..Default::default()
            },
        });
        scene.entities[1].components.push(Component::Physics {
            body: Default::default(),
            collider: Default::default(),
        });
        scene.entities[2].components.push(Component::Tilemap {
            map: "terrain".into(),
        });
        project.cameras.insert(
            scene.id.clone(),
            kiln_core::camera::CameraSettings {
                target: Some("child".into()),
                position: [12.0, -8.0],
                zoom: 2.0,
                smoothing: 3.0,
                dead_zone: [32.0, 16.0],
                bounds: Some([-100.0, -80.0, 200.0, 160.0]),
                pixel_perfect: true,
                pixels_per_unit: 2.0,
            },
        );
    }
    project
}

fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn every_component_survives_projection_history_recovery_and_disk_restart() {
    let mut project = rich_project();
    let original = project.clone();
    // Assert every enum variant participates; empty/default-only documents cannot
    // silently turn this acceptance into another transform-only roundtrip.
    let kinds: std::collections::HashSet<_> = project.scenes["alpha"]
        .entities
        .iter()
        .flat_map(|entity| &entity.components)
        .map(|component| json(component)["type"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds.len(), 12);
    for id in ["alpha", "beta"] {
        let observed = snapshot(&project, &spawn(&project, id), id);
        for (actual, expected) in observed.entities.iter().zip(&original.scenes[id].entities) {
            assert_eq!(
                actual.components.len(),
                expected.components.len(),
                "{} component count",
                expected.id
            );
            for (a, e) in actual.components.iter().zip(&expected.components) {
                assert_eq!(
                    json(a),
                    json(e),
                    "{id}/{} component {}",
                    expected.id,
                    json(e)["type"]
                );
            }
        }
        assert_eq!(json(&observed), json(&original.scenes[id]));
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("kiln.toml");
    project_files::save(&project, &path).unwrap();
    let original_bytes = std::fs::read(&path).unwrap();
    let mut history = kiln_editor::SceneHistory::new();
    let duplicate = entity_edit(
        &mut project,
        &mut history,
        EntityEdit::Duplicate {
            entity_id: "actor".into(),
        },
    )
    .unwrap();
    let mut expected_duplicate = original.scenes["alpha"].entities[0].clone();
    expected_duplicate.id = duplicate.clone();
    expected_duplicate.name = "Actor Copy".into();
    for component in &mut expected_duplicate.components {
        match component {
            Component::Joint { joint } => joint.entity_a = duplicate.clone(),
            Component::PrefabInstance { overrides, .. } => overrides.entity_id = duplicate.clone(),
            _ => {}
        }
    }
    let mut duplicated = original.clone();
    duplicated
        .scenes
        .get_mut("alpha")
        .unwrap()
        .entities
        .push(expected_duplicate);
    assert_eq!(json(&project), json(&duplicated));
    let duplicate_entity = project.scenes["alpha"]
        .entities
        .iter()
        .find(|e| e.id == duplicate)
        .unwrap();
    let joint = duplicate_entity
        .components
        .iter()
        .find_map(|c| match c {
            Component::Joint { joint } => Some(joint),
            _ => None,
        })
        .unwrap();
    assert_eq!(joint.entity_a, duplicate);
    assert_eq!(joint.entity_b, "child");
    assert!(project.scenes["alpha"]
        .entities
        .iter()
        .any(|e| e.id == joint.entity_b));
    entity_edit(
        &mut project,
        &mut history,
        EntityEdit::Delete {
            entity_id: duplicate,
        },
    );
    assert_eq!(json(&project), json(&original));
    let undone = history.undo(project.scenes["alpha"].clone()).unwrap();
    assert_eq!(json(&undone), json(&duplicated.scenes["alpha"]));
    let redone = history.redo(undone).unwrap();
    assert_eq!(json(&redone), json(&original.scenes["alpha"]));
    project
        .scenes
        .insert("alpha".into(), history.undo(redone).unwrap());
    assert_eq!(json(&project), json(&duplicated));

    kiln_editor::recovery::save(&path, &project).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
    // Set an explicit saved-file age so the newer-sidecar rule is deterministic,
    // including filesystems whose timestamp granularity exceeds this test's work.
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1))
        .unwrap();
    let (_, recovered) = kiln_editor::recovery::load_if_newer(&path)
        .unwrap()
        .unwrap();
    assert_eq!(json(&recovered), json(&duplicated));
    project_files::save(&recovered, &path).unwrap();
    let restarted = project_files::load(&path).unwrap().project;
    assert_eq!(json(&restarted), json(&duplicated));
    for id in ["alpha", "beta"] {
        let projected = snapshot(&restarted, &spawn(&restarted, id), id);
        assert_eq!(json(&projected), json(&restarted.scenes[id]));
    }
}
