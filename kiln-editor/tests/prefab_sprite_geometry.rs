//! Instance dimensions/layer must survive captured overrides and definition changes.
use kiln_core::{Component, Entity, Prefab, PrefabOverride, Project, Scene};
use kiln_editor::{editing, project_files, scene_document};
use kiln_runtime::{GameWorld, Sprite};

fn geometry(entity: &Entity) -> (f32, f32, f32) {
    entity
        .components
        .iter()
        .find_map(|component| match component {
            Component::Sprite {
                width,
                height,
                layer,
                ..
            } => Some((*width, *height, *layer)),
            _ => None,
        })
        .unwrap()
}
fn capture(project: &Project, scene: &str) -> Scene {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        scene,
        &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    scene_document::snapshot_for_scene(
        &world,
        &Default::default(),
        &Default::default(),
        Some(project),
        Some(scene),
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
                material: None,
                animation_clip: None,
            }],
        },
    };
    let mut project = Project::default();
    for scene_id in ["main", "other"] {
        let mut entity = definition.root_entity.clone();
        entity.id = format!("{scene_id}-hero");
        if scene_id == "main" {
            if let Component::Sprite {
                width,
                height,
                layer,
                ..
            } = &mut entity.components[0]
            {
                *width = 48.0;
                *height = 20.0;
                *layer = 7.0;
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
    project.prefabs.insert("hero".into(), definition);
    project
}
#[test]
fn captured_sprite_geometry_survives_persistence_and_cross_scene_apply() {
    let mut project = fixture();
    for scene in ["main", "other"] {
        project
            .scenes
            .insert(scene.into(), capture(&project, scene));
    }
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
        if let Component::Sprite {
            width,
            height,
            layer,
            ..
        } = &mut persisted.scenes.get_mut("other").unwrap().entities[0].components[0]
        {
            *width = 64.0;
            *height = 80.0;
            *layer = 2.0;
        }
        editing::apply_prefab_instance_to_definition(&mut persisted, "other", "other-hero")
            .unwrap();
        assert_eq!(
            geometry(&persisted.scenes["main"].entities[0]),
            (48.0, 20.0, 7.0)
        );
        assert_eq!(
            geometry(&persisted.prefabs["hero"].root_entity),
            (64.0, 80.0, 2.0)
        );
        assert_eq!(
            geometry(&capture(&persisted, "main").entities[0]),
            (48.0, 20.0, 7.0)
        );
        project_files::save(&persisted, &path).unwrap();
        let reopened = project_files::load(&path).unwrap().project;
        assert_eq!(
            serde_json::to_value(&reopened).unwrap(),
            serde_json::to_value(&persisted).unwrap()
        );
    }
}

#[test]
fn sparse_geometry_inherits_other_fields_and_revert_is_one_undoable_edit() {
    let mut project = fixture();
    if let Component::Sprite { height, layer, .. } =
        &mut project.scenes.get_mut("main").unwrap().entities[0].components[0]
    {
        *height = 32.0;
        *layer = 0.0;
    }
    project
        .scenes
        .insert("main".into(), capture(&project, "main"));
    project = Project::from_toml(&project.to_toml().unwrap()).unwrap();
    let mut updated = project.prefabs["hero"].clone();
    if let Component::Sprite {
        width,
        height,
        layer,
        ..
    } = &mut updated.root_entity.components[0]
    {
        *width = 64.0;
        *height = 80.0;
        *layer = 2.0;
    }
    project.prefabs.insert("hero".into(), updated.clone());
    editing::propagate_prefab_in_scenes(&mut project, &updated).unwrap();
    assert_eq!(
        geometry(&project.scenes["main"].entities[0]),
        (48.0, 80.0, 2.0)
    );
    let mut scene = project.scenes["main"].clone();
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
    assert_eq!(geometry(&scene.entities[0]), (64.0, 80.0, 2.0));
    assert_eq!(history.undo_depth(), 1);
    let after = serde_json::to_value(&scene).unwrap();
    let restored = history.undo(scene).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), before);
    assert_eq!(
        serde_json::to_value(history.redo(restored).unwrap()).unwrap(),
        after
    );
}

#[test]
fn live_instance_refresh_preserves_geometry_overrides_and_recapture_removes_inherited_values() {
    let mut project = fixture();
    project
        .scenes
        .insert("main".into(), capture(&project, "main"));
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        &project,
        "main",
        &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    assert_eq!(
        editing::attach_prefab_instances(&mut world, &project.scenes["main"]),
        1
    );
    let mut updated = project.prefabs["hero"].clone();
    if let Component::Sprite {
        width,
        height,
        layer,
        ..
    } = &mut updated.root_entity.components[0]
    {
        *width = 90.0;
        *height = 100.0;
        *layer = 3.0;
    }
    assert_eq!(editing::update_prefab_instances(&mut world, &updated), 1);
    let current = scene_document::snapshot_for_scene(
        &world,
        &Default::default(),
        &Default::default(),
        Some(&project),
        Some("main"),
    );
    assert_eq!(geometry(&current.entities[0]), (48.0, 20.0, 7.0));
    // Returning all three properties to their definition values removes the delta.
    if let Component::Sprite {
        width,
        height,
        layer,
        ..
    } = &mut project.scenes.get_mut("main").unwrap().entities[0].components[0]
    {
        *width = 32.0;
        *height = 32.0;
        *layer = 0.0;
    }
    let inherited = capture(&project, "main");
    let overrides = inherited.entities[0]
        .components
        .iter()
        .find_map(|component| match component {
            Component::PrefabInstance { overrides, .. } => Some(overrides),
            _ => None,
        })
        .unwrap();
    assert!(!overrides.overrides.contains_key("sprite_geometry"));
}
