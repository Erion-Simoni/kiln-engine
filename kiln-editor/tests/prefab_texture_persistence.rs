//! Explicit texture clears must survive project/recovery persistence and propagation.
use kiln_core::{Component, ComponentOverride, Entity, Prefab, PrefabOverride, Project, Scene};
use kiln_editor::{editing, project_files, recovery, scene_document};
use kiln_runtime::{GameWorld, Sprite};
use std::collections::HashMap;

fn sprite(entity: &Entity) -> &Option<String> {
    entity
        .components
        .iter()
        .find_map(|component| match component {
            Component::Sprite { texture, .. } => Some(texture),
            _ => None,
        })
        .unwrap()
}

#[test]
fn cleared_texture_survives_capture_save_recovery_asset_move_and_cross_scene_apply() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("kiln.toml");
    let definition = Prefab {
        id: "hero".into(),
        name: "Hero".into(),
        root_entity: Entity {
            id: "template".into(),
            name: "Hero".into(),
            components: vec![
                Component::Transform {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Sprite {
                    color: [1.0; 4],
                    width: 32.0,
                    height: 32.0,
                    texture: Some("art/hero.png".into()),
                    layer: 0.0,
                    material: None,
                    animation_clip: None,
                },
            ],
        },
    };
    let mut project = Project::default();
    project.prefabs.insert("hero".into(), definition.clone());
    for (scene_id, texture) in [("main", None), ("other", Some("art/hero.png".into()))] {
        let mut entity = definition.root_entity.clone();
        entity.id = format!("{scene_id}-hero");
        for component in &mut entity.components {
            if let Component::Sprite {
                texture: current, ..
            } = component
            {
                *current = texture.clone();
            }
        }
        entity.components.push(Component::PrefabInstance {
            prefab_id: "hero".into(),
            overrides: PrefabOverride::new(entity.id.clone()),
        });
        project.scenes.insert(
            scene_id.into(),
            Scene {
                id: scene_id.into(),
                name: scene_id.into(),
                script: None,
                entities: vec![entity],
                nested_scenes: vec![],
            },
        );
    }
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        &project,
        "main",
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    let captured = scene_document::snapshot_for_scene(
        &world,
        &HashMap::new(),
        &HashMap::new(),
        Some(&project),
        Some("main"),
    );
    let link = captured.entities[0]
        .components
        .iter()
        .find_map(|component| match component {
            Component::PrefabInstance { overrides, .. } => Some(overrides),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        link.overrides.get("sprite"),
        Some(ComponentOverride::Sprite {
            texture: Some(None),
            ..
        })
    ));
    project.scenes.insert("main".into(), captured);
    project_files::save(&project, &path).unwrap();
    let reopened = project_files::load(&path).unwrap().project;
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    recovery::save(&path, &project).unwrap();
    // Read the actual atomic recovery file without relying on filesystem mtime resolution.
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recovery::path_for(&path)).unwrap()).unwrap();
    let recovered: Project = serde_json::from_value(snapshot["project"].clone()).unwrap();
    for mut persisted in [reopened, recovered] {
        persisted.rewrite_resource_path("art/hero.png", "art/moved.png");
        assert_eq!(sprite(&persisted.scenes["main"].entities[0]), &None);
        let mut updated = persisted.prefabs["hero"].clone();
        if let Component::Sprite { color, .. } = &mut updated.root_entity.components[1] {
            *color = [0.0, 1.0, 0.0, 1.0];
        }
        assert_eq!(
            editing::propagate_prefab_in_scenes(&mut persisted, &updated),
            2
        );
        assert_eq!(sprite(&persisted.scenes["main"].entities[0]), &None);
        assert_eq!(
            sprite(&persisted.scenes["other"].entities[0]),
            &Some("art/moved.png".into())
        );
        // Applying another instance must retain the first instance's clear too.
        editing::apply_prefab_instance_to_definition(&mut persisted, "other", "other-hero")
            .unwrap();
        assert_eq!(sprite(&persisted.scenes["main"].entities[0]), &None);
        project_files::save(&persisted, &path).unwrap();
        let final_project = project_files::load(&path).unwrap().project;
        assert_eq!(
            serde_json::to_value(&final_project).unwrap(),
            serde_json::to_value(&persisted).unwrap()
        );
    }
}
