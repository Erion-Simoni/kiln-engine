//! Shared inspector/viewport transform commands with real history and disk checks.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_editor::{
    project_files, scene_view,
    transform_edit::{SetTransform, TransformTransaction, TranslationDrag},
    transform_gizmo::{apply_group_transform, order_group_targets, GizmoMode},
    SceneHistory,
};
use kiln_runtime::{GameWorld, SourceId, Sprite};

fn sample() -> Scene {
    Scene {
        id: "main".into(),
        name: "Authored level".into(),
        script: Some("scripts/main.ks".into()),
        nested_scenes: vec![],
        entities: vec![
            Entity {
                id: "hero".into(),
                name: "Authored hero".into(),
                components: vec![
                    Component::Transform {
                        x: 10.0,
                        y: 20.0,
                        rotation: 0.3,
                        scale: 1.5,
                    },
                    Component::Sprite {
                        color: [0.2, 0.4, 0.8, 1.0],
                        width: 32.0,
                        height: 32.0,
                        texture: Some("art/hero.png".into()),
                        layer: 7.0,
                        material: Some("ink".into()),
                        animation_clip: Some("idle".into()),
                    },
                    Component::Role {
                        role: "player".into(),
                    },
                    Component::Parent {
                        parent: "marker".into(),
                    },
                ],
            },
            Entity {
                id: "marker".into(),
                name: "Nonvisual marker".into(),
                components: vec![Component::Role {
                    role: "spawn".into(),
                }],
            },
        ],
    }
}
fn spawn(scene: &Scene) -> GameWorld {
    let mut project = Project::default();
    project.scenes.insert(scene.id.clone(), scene.clone());
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        &project,
        &scene.id,
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    world
}
fn entity(world: &GameWorld, id: &str) -> hecs::Entity {
    world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, s)| s.0 == id)
        .unwrap()
        .0
}
fn runtime(world: &GameWorld, id: &str) -> Transform {
    *world
        .world_ref()
        .get::<&Transform>(entity(world, id))
        .unwrap()
}
fn global(world: &GameWorld, id: &str) -> Transform {
    world
        .world_ref()
        .get::<&kiln_runtime::GlobalTransform>(entity(world, id))
        .unwrap()
        .0
}

#[test]
fn local_axis_drag_on_a_rotated_parented_child_roundtrips_through_local_storage() {
    use kiln_editor::transform_gizmo::{GizmoDrag, GizmoSpace, TranslationAxis};
    let mut scene = sample();
    let mut world = spawn(&scene);
    SetTransform {
        scene_id: "main",
        entity_id: "marker",
        value: Transform::new(100.0, 200.0)
            .with_rotation(std::f32::consts::FRAC_PI_2)
            .with_scale(kiln_core::Vec2::splat(2.0)),
    }
    .apply(&mut scene, &mut world)
    .unwrap();
    let initial = global(&world, "hero");
    let drag = GizmoDrag::new(
        GizmoMode::Translate,
        egui::Pos2::ZERO,
        initial,
        [1.0, 1.0],
        initial.position,
    )
    .unwrap()
    .in_space(GizmoSpace::Local, TranslationAxis::X);
    let target = drag.at(egui::pos2(0.0, -25.0), Some(16.0));
    let local = kiln_core::hierarchy_transform::local_from_world(&scene, "hero", target).unwrap();
    apply(&mut scene, &mut world, local);
    let actual = global(&world, "hero");
    assert!((actual.position - target.position).length() < 0.001);
    assert!((actual.rotation - initial.rotation).abs() < 0.001);
    assert_eq!(actual.scale, initial.scale);
    let (sin, cos) = initial.rotation.sin_cos();
    let delta = actual.position - initial.position;
    assert!((delta.dot(kiln_core::Vec2::new(cos, sin)) - 16.0).abs() < 0.001);
    assert!(delta.dot(kiln_core::Vec2::new(-sin, cos)).abs() < 0.001);
}
fn apply(scene: &mut Scene, world: &mut GameWorld, value: Transform) -> bool {
    SetTransform {
        scene_id: "main",
        entity_id: "hero",
        value,
    }
    .apply(scene, world)
    .unwrap()
}
fn json(value: &(impl serde::Serialize + ?Sized)) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn command_updates_document_and_projection_preserving_unrelated_metadata() {
    let mut scene = sample();
    let original = scene.clone();
    let mut world = spawn(&scene);
    let value = Transform::new(42.0, -18.0)
        .with_rotation(0.7)
        .with_scale(kiln_core::Vec2::splat(2.0));
    assert!(apply(&mut scene, &mut world, value));
    assert_eq!(runtime(&world, "hero"), value);
    assert_eq!(
        json(&scene.entities[0].components[1..]),
        json(&original.entities[0].components[1..])
    );
    assert_eq!(scene.name, original.name);
    assert_eq!(scene.script, original.script);
    assert_eq!(scene.entities[0].name, original.entities[0].name);
    assert_eq!(json(&scene.entities[1]), json(&original.entities[1]));
    assert!(!apply(&mut scene, &mut world, value));
}

#[test]
fn moving_a_parent_recomputes_descendants_while_children_keep_local_values() {
    let mut scene = sample();
    let mut world = spawn(&scene);
    let parent_value = Transform::new(100.0, 200.0)
        .with_rotation(std::f32::consts::FRAC_PI_2)
        .with_scale(kiln_core::Vec2::splat(2.0));
    assert!(SetTransform {
        scene_id: "main",
        entity_id: "marker",
        value: parent_value,
    }
    .apply(&mut scene, &mut world)
    .unwrap());

    let child = global(&world, "hero");
    assert!((child.position.x - 60.0).abs() < 0.0001);
    assert!((child.position.y - 220.0).abs() < 0.0001);
    assert!((child.rotation - (std::f32::consts::FRAC_PI_2 + 0.3)).abs() < 0.0001);
    assert_eq!(child.scale.x, 3.0);

    let local = scene.entities[0]
        .components
        .iter()
        .find_map(|component| match component {
            Component::Transform { x, y, .. } => Some((*x, *y)),
            _ => None,
        });
    assert_eq!(local, Some((10.0, 20.0)));
    let project = Project {
        scenes: std::collections::HashMap::from([("main".into(), scene.clone())]),
        ..Project::default()
    };
    let snapshot = kiln_editor::scene_document::snapshot_for_scene(
        &world,
        &Default::default(),
        &Default::default(),
        Some(&project),
        Some("main"),
    );
    assert_eq!(
        json(&snapshot.entities[0].components[0]),
        json(&scene.entities[0].components[0])
    );
}

#[test]
fn translating_parent_and_child_selection_keeps_their_world_offset() {
    let mut scene = sample();
    let mut world = spawn(&scene);
    let original_parent = global(&world, "marker");
    let original_child = global(&world, "hero");
    let delta = kiln_core::Vec2::new(12.0, -7.0);

    for (entity_id, initial) in [("marker", original_parent), ("hero", original_child)] {
        let mut target = initial;
        target.position += delta;
        let local =
            kiln_core::hierarchy_transform::local_from_world(&scene, entity_id, target).unwrap();
        SetTransform {
            scene_id: "main",
            entity_id,
            value: local,
        }
        .apply(&mut scene, &mut world)
        .unwrap();
    }

    assert_eq!(
        global(&world, "marker").position,
        original_parent.position + delta
    );
    assert_eq!(
        global(&world, "hero").position,
        original_child.position + delta
    );
}

#[test]
fn group_rotation_and_scale_preserve_parented_world_targets() {
    let selection = |world: &GameWorld| {
        vec![
            ("marker".into(), global(world, "marker")),
            ("hero".into(), global(world, "hero")),
        ]
    };

    let mut scene = sample();
    let mut world = spawn(&scene);
    let initial = selection(&world);
    let child_selected_first = vec![initial[1].clone(), initial[0].clone()];
    let primary = initial[0].1;
    let result = Transform {
        rotation: std::f32::consts::FRAC_PI_2,
        ..primary
    };
    let targets = apply_group_transform(GizmoMode::Rotate, "marker", result, &child_selected_first);
    let targets = order_group_targets(&scene, &targets);
    for (id, target) in &targets {
        let local = kiln_core::hierarchy_transform::local_from_world(&scene, id, *target).unwrap();
        SetTransform {
            scene_id: "main",
            entity_id: id,
            value: local,
        }
        .apply(&mut scene, &mut world)
        .unwrap();
    }
    assert!((global(&world, "hero").position.x + 20.0).abs() < 0.0001);
    assert!((global(&world, "hero").position.y - 10.0).abs() < 0.0001);
    assert!((global(&world, "hero").rotation - (std::f32::consts::FRAC_PI_2 + 0.3)).abs() < 0.0001);

    let mut scene = sample();
    let mut world = spawn(&scene);
    let initial = selection(&world);
    let mut result = initial[0].1;
    result.scale = kiln_core::Vec2::splat(2.0);
    let targets = apply_group_transform(GizmoMode::Scale, "marker", result, &child_selected_first);
    let targets = order_group_targets(&scene, &targets);
    for (id, target) in &targets {
        let local = kiln_core::hierarchy_transform::local_from_world(&scene, id, *target).unwrap();
        SetTransform {
            scene_id: "main",
            entity_id: id,
            value: local,
        }
        .apply(&mut scene, &mut world)
        .unwrap();
    }
    assert_eq!(
        global(&world, "hero").position,
        kiln_core::Vec2::new(20.0, 40.0)
    );
    assert!((global(&world, "hero").scale.x - 3.0).abs() < 0.0001);
}

#[test]
fn shared_group_scale_keeps_selected_child_local_document_exact() {
    use kiln_editor::transform_gizmo::group_root_targets;
    let mut scene = sample();
    let mut world = spawn(&scene);
    SetTransform {
        scene_id: "main",
        entity_id: "marker",
        value: Transform::new(40.0, 60.0)
            .with_rotation(std::f32::consts::FRAC_PI_2)
            .with_scale(kiln_core::Vec2::splat(2.0)),
    }
    .apply(&mut scene, &mut world)
    .unwrap();
    let child_before = json(&scene.entities[0]);
    let initial = vec![
        ("hero".into(), global(&world, "hero")),
        ("marker".into(), global(&world, "marker")),
    ];
    let mut primary = initial[1].1;
    primary.scale = kiln_core::Vec2::splat(2.3);
    let targets = apply_group_transform(GizmoMode::Scale, "marker", primary, &initial);
    let expected_child = targets[0].1;
    for (id, target) in group_root_targets(&scene, &targets) {
        let local = kiln_core::hierarchy_transform::local_from_world(&scene, &id, target).unwrap();
        SetTransform {
            scene_id: "main",
            entity_id: &id,
            value: local,
        }
        .apply(&mut scene, &mut world)
        .unwrap();
    }
    assert_eq!(json(&scene.entities[0]), child_before);
    let actual = global(&world, "hero");
    assert!((actual.position - expected_child.position).length() < 0.001);
    assert!((actual.rotation - expected_child.rotation).abs() < 0.001);
    assert!((actual.scale - expected_child.scale).length() < 0.001);
}

#[test]
fn repeated_preview_frames_form_one_undo_step_and_reopen_exactly() {
    let mut scene = sample();
    let before = scene.clone();
    let mut world = spawn(&scene);
    let mut gesture = TransformTransaction::new(before.clone(), "hero".into());
    let mut history = SceneHistory::new();
    for x in [15.0, 20.0, 45.0, 45.0] {
        let value = Transform {
            position: kiln_core::Vec2::new(x, 20.0),
            ..runtime(&world, "hero")
        };
        apply(&mut scene, &mut world, value);
        gesture.update(scene.clone());
    }
    gesture.commit(&mut history);
    assert_eq!(history.undo_depth(), 1);
    let after = scene.clone();
    let undone = history.undo(scene).unwrap();
    assert_eq!(json(&undone), json(&before));
    let redone = history.redo(undone).unwrap();
    assert_eq!(json(&redone), json(&after));
    let mut project = Project::default();
    project.scenes.insert("main".into(), redone);
    project.scenes.insert(
        "other".into(),
        Scene {
            id: "other".into(),
            ..sample()
        },
    );
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(json(&reopened.scenes["main"]), json(&after));
    assert_eq!(
        runtime(&spawn(&reopened.scenes["main"]), "hero").position.x,
        45.0
    );
    assert_eq!(
        json(&reopened.scenes["other"]),
        json(&project.scenes["other"])
    );
}

#[test]
fn returning_to_the_original_transform_keeps_redo_and_does_not_add_history() {
    let mut scene = sample();
    let original = scene.clone();
    let mut world = spawn(&scene);
    let initial = runtime(&world, "hero");
    let mut history = SceneHistory::new();
    history.record(original.clone());
    history.undo(original.clone()).unwrap();
    let mut gesture = TransformTransaction::new(original.clone(), "hero".into());
    apply(&mut scene, &mut world, Transform::new(99.0, 20.0));
    gesture.update(scene.clone());
    apply(&mut scene, &mut world, initial);
    gesture.update(scene);
    gesture.commit(&mut history);
    assert_eq!(history.undo_depth(), 0);
    assert!(history.can_redo());
}

#[test]
fn cancel_restores_the_scene_and_runtime_without_consuming_redo() {
    let mut scene = sample();
    let before = scene.clone();
    let mut world = spawn(&scene);
    let mut gesture = TransformTransaction::new(before.clone(), "hero".into());
    let mut history = SceneHistory::new();
    history.record(before.clone());
    history.undo(before.clone());
    apply(&mut scene, &mut world, Transform::new(999.0, 888.0));
    gesture.update(scene);
    let canceled = gesture.cancel();
    assert_eq!(json(&canceled), json(&before));
    assert_eq!(
        runtime(&spawn(&canceled), "hero"),
        runtime(&spawn(&before), "hero")
    );
    assert!(history.can_redo());
    assert_eq!(history.undo_depth(), 0);
}

#[test]
fn invalid_values_and_wrong_scene_or_entity_leave_both_states_unchanged() {
    let mut scene = sample();
    let before = json(&scene);
    let mut world = spawn(&scene);
    let initial = runtime(&world, "hero");
    for value in [
        Transform::new(f32::NAN, 1.0),
        Transform::new(1.0, f32::INFINITY),
        Transform::new(1.0, 2.0).with_rotation(f32::NAN),
        Transform::new(1.0, 2.0).with_scale(kiln_core::Vec2::new(1.0, 2.0)),
    ] {
        assert!(SetTransform {
            scene_id: "main",
            entity_id: "hero",
            value
        }
        .apply(&mut scene, &mut world)
        .is_err());
        assert_eq!(json(&scene), before);
        assert_eq!(runtime(&world, "hero"), initial);
    }
    for (scene_id, entity_id) in [("other", "hero"), ("main", "nested-only")] {
        assert!(SetTransform {
            scene_id,
            entity_id,
            value: Transform::new(50.0, 60.0)
        }
        .apply(&mut scene, &mut world)
        .is_err());
        assert_eq!(json(&scene), before);
        assert_eq!(runtime(&world, "hero"), initial);
    }
}

#[test]
fn ambiguous_or_missing_runtime_identity_cannot_partially_change_the_document() {
    let mut scene = sample();
    let before = json(&scene);
    let mut world = spawn(&scene);
    world
        .world_mut()
        .spawn((SourceId::new("hero"), Transform::default()));
    assert!(SetTransform {
        scene_id: "main",
        entity_id: "hero",
        value: Transform::new(99.0, 2.0)
    }
    .apply(&mut scene, &mut world)
    .is_err());
    assert_eq!(json(&scene), before);
    let mut world = GameWorld::new();
    assert!(SetTransform {
        scene_id: "main",
        entity_id: "hero",
        value: Transform::new(99.0, 2.0)
    }
    .apply(&mut scene, &mut world)
    .is_err());
    assert_eq!(json(&scene), before);
}

#[test]
fn editing_a_default_projection_adds_an_authored_transform_that_survives_reopen() {
    let mut scene = sample();
    let mut world = spawn(&scene);
    let value = Transform::new(321.0, 123.0);
    assert!(SetTransform {
        scene_id: "main",
        entity_id: "marker",
        value
    }
    .apply(&mut scene, &mut world)
    .unwrap());
    assert_eq!(scene.entities[1].components.len(), 2);
    let mut project = Project::default();
    project.scenes.insert("main".into(), scene);
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(runtime(&spawn(&reopened.scenes["main"]), "marker"), value);
    assert!(
        matches!(&reopened.scenes["main"].entities[1].components[0],Component::Role {role} if role=="spawn")
    );
}

#[test]
fn translation_is_absolute_with_y_up_and_world_grid_snapping() {
    let drag = TranslationDrag {
        origin: egui::pos2(100.0, 200.0),
        initial: Transform::new(10.0, 20.0).with_rotation(0.3),
    };
    let pointer = egui::pos2(140.0, 170.0);
    let units = [2.0, 2.0];
    let value = drag.at(pointer, units, None);
    assert_eq!(value.position, kiln_core::Vec2::new(90.0, 80.0));
    assert_eq!(value.rotation, 0.3);
    for _ in 0..100 {
        assert_eq!(drag.at(pointer, units, None), value);
    }
    assert_eq!(
        drag.at(pointer, units, Some(32.0)).position,
        kiln_core::Vec2::new(96.0, 96.0)
    );
    assert_eq!(drag.at(pointer, units, Some(0.0)), value);
    assert_eq!(drag.at(pointer, units, Some(f32::NAN)), value);
}

#[test]
fn letterboxed_mapping_accounts_for_offsets_dpi_and_camera_zoom() {
    let panel = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 600.0));
    let viewport = scene_view::canvas_viewport(panel, 1.0, (1280.0, 720.0));
    assert_eq!(
        viewport,
        scene_view::ViewportRect {
            x: 100,
            y: 125,
            width: 800,
            height: 450
        }
    );
    let mut camera = scene_view::world_camera(1280.0, 720.0);
    camera.position = [300.0, 400.0];
    camera.zoom = 2.0;
    let units = scene_view::units_per_point(viewport, 1.0, &camera).unwrap();
    assert_eq!(units, [0.8, 0.8]);
    let high_dpi = scene_view::canvas_viewport(panel, 2.0, (1280.0, 720.0));
    assert_eq!(
        scene_view::units_per_point(high_dpi, 2.0, &camera).unwrap(),
        units
    );
    assert!(
        scene_view::units_per_point(scene_view::letterbox(0, 0, 1.0, 1.0), 1.0, &camera).is_none()
    );
    assert!(scene_view::units_per_point(viewport, 0.0, &camera).is_none());
}

#[test]
fn scene_settings_history_keeps_camera_presence_and_interleaved_entity_edits() {
    use kiln_core::camera::CameraSettings;
    let before = sample();
    let mut settings = before.clone();
    settings.name = "Changed level".into();
    settings.script = Some("scripts/other.ks".into());
    settings.nested_scenes.push(kiln_core::NestedScene {
        scene_id: "room".into(),
        offset: kiln_core::SceneOffset { x: 15.0, y: -20.0 },
        additive: true,
    });
    let camera = CameraSettings {
        zoom: 2.5,
        target: Some("hero".into()),
        ..Default::default()
    };
    let mut history = SceneHistory::new();
    let mut edit = TransformTransaction::new_document(before.clone(), None);
    edit.update(settings.clone());
    edit.update_camera(Some(camera.clone()));
    edit.commit(&mut history);
    // An ordinary entity command records a scene, and must leave the camera alone.
    let mut entity_edit = settings.clone();
    entity_edit.entities[0].name = "Renamed hero".into();
    history.record(settings.clone());
    let (scene, replacement) = history
        .undo_document(entity_edit.clone(), Some(camera.clone()))
        .unwrap();
    assert_eq!(json(&scene), json(&settings));
    assert_eq!(replacement, None);
    let (scene, replacement) = history.undo_document(scene, Some(camera.clone())).unwrap();
    assert_eq!(json(&scene), json(&before));
    assert_eq!(replacement, Some(None));
    let (scene, replacement) = history.redo_document(scene, None).unwrap();
    assert_eq!(json(&scene), json(&settings));
    assert_eq!(replacement, Some(Some(camera.clone())));
    let (scene, replacement) = history.redo_document(scene, Some(camera)).unwrap();
    assert_eq!(json(&scene), json(&entity_edit));
    assert_eq!(replacement, None);
}

#[test]
fn cancelled_and_noop_settings_preserve_camera_and_redo_exactly() {
    use kiln_core::camera::CameraSettings;
    let scene = sample();
    let camera = CameraSettings {
        zoom: 2.0,
        ..Default::default()
    };
    let mut changed = scene.clone();
    changed.name = "Changed".into();
    let mut history = SceneHistory::new();
    history.record(scene.clone());
    let _ = history.undo(changed).unwrap();
    let mut edit = TransformTransaction::new_document(scene.clone(), Some(camera.clone()));
    edit.update_camera(None);
    let (restored, replacement) = edit.cancel_document();
    assert_eq!(json(&restored), json(&scene));
    assert_eq!(replacement, Some(Some(camera.clone())));
    assert!(history.can_redo());
    let mut noop = TransformTransaction::new_document(scene, Some(camera.clone()));
    noop.update_camera(None);
    noop.update_camera(Some(camera));
    noop.commit(&mut history);
    assert!(history.can_redo());
    assert_eq!(history.undo_depth(), 0);
}
