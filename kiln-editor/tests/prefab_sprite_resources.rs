//! Material/clip assignments and clears must remain per-instance during propagation.
use kiln_core::{Component, Entity, Prefab, PrefabOverride, Project, Scene};
use kiln_editor::{editing, project_files, scene_document};
use kiln_runtime::{GameWorld, Sprite};

fn resources(entity: &Entity) -> (Option<String>, Option<String>) {
    entity
        .components
        .iter()
        .find_map(|c| match c {
            Component::Sprite {
                material,
                animation_clip,
                ..
            } => Some((material.clone(), animation_clip.clone())),
            _ => None,
        })
        .unwrap()
}
fn set(entity: &mut Entity, material: Option<&str>, clip: Option<&str>) {
    for c in &mut entity.components {
        if let Component::Sprite {
            material: current,
            animation_clip,
            ..
        } = c
        {
            *current = material.map(str::to_owned);
            *animation_clip = clip.map(str::to_owned);
        }
    }
}
fn capture(project: &Project) -> Scene {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        "main",
        &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    scene_document::snapshot_for_scene(
        &world,
        &Default::default(),
        &Default::default(),
        Some(project),
        Some("main"),
    )
}
fn fixture() -> Project {
    let definition = Prefab {
        id: "hero".into(),
        name: "Hero".into(),
        root_entity: Entity {
            id: "template".into(),
            name: "Hero".into(),
            components: vec![Component::Sprite {
                color: [1.0; 4],
                width: 32.0,
                height: 32.0,
                layer: 0.0,
                texture: None,
                material: Some("base".into()),
                animation_clip: Some("idle".into()),
            }],
        },
    };
    let mut project = Project::default();
    for scene in ["main", "other"] {
        let mut entity = definition.root_entity.clone();
        entity.id = format!("{scene}-hero");
        entity.components.push(Component::PrefabInstance {
            prefab_id: "hero".into(),
            overrides: PrefabOverride::new(entity.id.clone()),
        });
        project.scenes.insert(
            scene.into(),
            Scene {
                id: scene.into(),
                name: scene.into(),
                script: None,
                entities: vec![entity],
                nested_scenes: vec![],
            },
        );
    }
    project.prefabs.insert("hero".into(), definition);
    project
}
#[test]
fn material_and_clip_set_or_clear_survive_capture_persistence_and_apply() {
    for (material, clip) in [
        (None, None),
        (Some("alternate"), Some("run")),
        (Some("alternate"), Some("idle")),
    ] {
        let mut project = fixture();
        set(
            &mut project.scenes.get_mut("main").unwrap().entities[0],
            material,
            clip,
        );
        project.scenes.insert("main".into(), capture(&project));
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("kiln.toml");
        project_files::save(&project, &path).unwrap();
        let saved = project_files::load(&path).unwrap().project;
        kiln_editor::recovery::save(&path, &project).unwrap();
        let snapshot: serde_json::Value =
            serde_json::from_slice(&std::fs::read(kiln_editor::recovery::path_for(&path)).unwrap())
                .unwrap();
        let recovered: Project = serde_json::from_value(snapshot["project"].clone()).unwrap();
        for mut persisted in [saved, recovered] {
            set(
                &mut persisted.scenes.get_mut("other").unwrap().entities[0],
                Some("new-base"),
                Some("new-idle"),
            );
            editing::apply_prefab_instance_to_definition(&mut persisted, "other", "other-hero")
                .unwrap();
            let expected_clip = if clip == Some("idle") {
                Some("new-idle")
            } else {
                clip
            };
            assert_eq!(
                resources(&persisted.scenes["main"].entities[0]),
                (
                    material.map(str::to_owned),
                    expected_clip.map(str::to_owned)
                )
            );
            project_files::save(&persisted, &path).unwrap();
            let reopened = project_files::load(&path).unwrap().project;
            assert_eq!(
                serde_json::to_value(&reopened).unwrap(),
                serde_json::to_value(&persisted).unwrap()
            );
        }
    }
}

#[test]
fn live_refresh_and_revert_preserve_sparse_resource_overrides() {
    for clear in [true, false] {
        let mut project = fixture();
        set(
            &mut project.scenes.get_mut("main").unwrap().entities[0],
            if clear { None } else { Some("alternate") },
            Some("idle"),
        );
        project.scenes.insert("main".into(), capture(&project));
        let mut world = GameWorld::new();
        kiln_runtime::spawn_scene(
            &mut world,
            &project,
            "main",
            &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
        );
        editing::attach_prefab_instances(&mut world, &project.scenes["main"]);
        let mut updated = project.prefabs["hero"].clone();
        set(&mut updated.root_entity, Some("new-base"), Some("new-idle"));
        editing::update_prefab_instances(&mut world, &updated);
        let current = scene_document::snapshot_for_scene(
            &world,
            &Default::default(),
            &Default::default(),
            Some(&project),
            Some("main"),
        );
        assert_eq!(
            resources(&current.entities[0]),
            (
                if clear {
                    None
                } else {
                    Some("alternate".into())
                },
                Some("new-idle".into())
            )
        );
        project.prefabs.insert("hero".into(), updated);
        let mut scene = current;
        let before = serde_json::to_value(&scene).unwrap();
        let mut history = kiln_editor::SceneHistory::new();
        kiln_editor::entity_command::EntityCommand {
            scene_id: "main".into(),
            edit: kiln_editor::entity_command::EntityEdit::RevertPrefabOverrides {
                entity_id: "main-hero".into(),
            },
        }
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        assert_eq!(
            resources(&scene.entities[0]),
            (Some("new-base".into()), Some("new-idle".into()))
        );
        assert_eq!(history.undo_depth(), 1);
        let after = serde_json::to_value(&scene).unwrap();
        let restored = history.undo(scene).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), before);
        assert_eq!(
            serde_json::to_value(history.redo(restored).unwrap()).unwrap(),
            after
        );
    }
}
