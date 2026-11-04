//! Component command acceptance through the actual runtime projection and disk.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_editor::{
    component_command::{ComponentCommand, ComponentEdit, ComponentKind},
    project_files,
    scene_document::snapshot_for_scene,
    SceneHistory,
};
use kiln_runtime::{GameWorld, SourceId, Sprite, Tilemap};
use std::collections::HashMap;

fn fixture() -> Project {
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Authored level".into(),
            script: Some("scripts/main.ks".into()),
            nested_scenes: vec![],
            entities: vec![
                Entity {
                    id: "hero".into(),
                    name: "Authored hero".into(),
                    components: vec![Component::Parent {
                        parent: "marker".into(),
                    }],
                },
                Entity {
                    id: "marker".into(),
                    name: "Nonvisual marker".into(),
                    components: vec![],
                },
            ],
        },
    );
    project.scenes.insert(
        "other".into(),
        Scene {
            id: "other".into(),
            name: "Other level".into(),
            entities: vec![],
            script: None,
            nested_scenes: vec![],
        },
    );
    project
        .tilemaps
        .insert("level_map".into(), kiln_core::TileMap::default());
    project
}
fn components() -> Vec<Component> {
    vec![
        Component::Transform {
            x: 12.0,
            y: 34.0,
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
        Component::Tilemap {
            map: "level_map".into(),
        },
    ]
}
fn command(edit: ComponentEdit) -> ComponentCommand {
    ComponentCommand {
        scene_id: "main".into(),
        entity_id: "hero".into(),
        edit,
    }
}
fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}
fn spawn(project: &Project) -> GameWorld {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        "main",
        &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    kiln_runtime::spawn_tilemaps(
        &mut world,
        project,
        &project.scenes["main"],
        Default::default(),
        &mut |_| None,
    );
    world
}
fn snapshot(project: &Project, world: &GameWorld) -> Scene {
    snapshot_for_scene(
        world,
        &HashMap::new(),
        &HashMap::new(),
        Some(project),
        Some("main"),
    )
}
fn assert_disk(project: &Project) {
    let directory = tempfile::tempdir().unwrap();
    project_files::save(project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(json(&reopened), json(project));
    assert_eq!(
        json(&snapshot(&reopened, &spawn(&reopened))),
        json(&project.scenes["main"])
    );
}

#[test]
fn every_supported_component_survives_add_remove_undo_redo_and_disk_reopen() {
    for component in components() {
        let mut project = fixture();
        let before = project.scenes["main"].clone();
        let mut scene = before.clone();
        let mut history = SceneHistory::new();
        let kind = ComponentKind::of(&component).unwrap();
        assert!(command(ComponentEdit::Add(component.clone()))
            .execute(&mut scene, &project, &mut history)
            .unwrap());
        assert_eq!(history.undo_depth(), 1);
        assert_eq!(scene.entities[0].name, "Authored hero");
        assert_eq!(
            json(&scene.entities[0].components[0]),
            json(&before.entities[0].components[0])
        );
        project.scenes.insert("main".into(), scene.clone());
        assert_disk(&project);
        let added = scene.clone();
        let undone = history.undo(scene).unwrap();
        assert_eq!(json(&undone), json(&before));
        let mut scene = history.redo(undone).unwrap();
        assert_eq!(json(&scene), json(&added));
        assert!(command(ComponentEdit::Remove(kind))
            .execute(&mut scene, &project, &mut history)
            .unwrap());
        assert_eq!(json(&scene), json(&before));
        project.scenes.insert("main".into(), scene.clone());
        assert_disk(&project);
        let restored = history.undo(scene).unwrap();
        assert_eq!(json(&restored), json(&added));
        assert_eq!(
            json(&project.scenes["other"]),
            json(&fixture().scenes["other"])
        );
    }
}

#[test]
fn sprite_references_are_normalized_undoable_and_saved() {
    let mut project = fixture();
    let original = Component::Sprite {
        color: [1.0; 4],
        width: 32.0,
        height: 32.0,
        texture: None,
        layer: 0.0,
        material: None,
        animation_clip: None,
    };
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(original);
    let mut scene = project.scenes["main"].clone();
    let mut history = SceneHistory::new();
    let edit = || ComponentEdit::SetSpriteReferences {
        texture: Some(" art/hero.png ".into()),
        material: Some(" ink ".into()),
        animation_clip: Some(" idle ".into()),
    };
    assert!(command(edit())
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    assert_eq!(history.undo_depth(), 1);
    assert!(!command(edit())
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    let sprite = scene.entities[0]
        .components
        .iter()
        .find_map(|c| match c {
            Component::Sprite {
                texture,
                material,
                animation_clip,
                ..
            } => Some((texture, material, animation_clip)),
            _ => None,
        })
        .unwrap();
    assert_eq!(sprite.0.as_deref(), Some("art/hero.png"));
    assert_eq!(sprite.1.as_deref(), Some("ink"));
    assert_eq!(sprite.2.as_deref(), Some("idle"));
    project.scenes.insert("main".into(), scene.clone());
    assert_disk(&project);
    let restored = history.undo(scene).unwrap();
    assert!(matches!(
        restored.entities[0].components.last(),
        Some(Component::Sprite {
            texture: None,
            material: None,
            animation_clip: None,
            ..
        })
    ));
}

#[test]
fn tilemap_reference_can_be_changed_and_invalid_choices_are_atomic() {
    let mut project = fixture();
    project
        .tilemaps
        .insert("decor_map".into(), kiln_core::TileMap::default());
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(Component::Tilemap {
            map: "level_map".into(),
        });
    let mut scene = project.scenes["main"].clone();
    let mut history = SceneHistory::new();
    let set_map = |map: &str| command(ComponentEdit::SetTilemapReference { map: map.into() });

    assert!(set_map(" decor_map ")
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    assert_eq!(history.undo_depth(), 1);
    assert!(!set_map("decor_map")
        .execute(&mut scene, &project, &mut history)
        .unwrap());

    let before_invalid = json(&scene);
    assert!(set_map("missing")
        .execute(&mut scene, &project, &mut history)
        .is_err());
    assert_eq!(json(&scene), before_invalid);
    assert_eq!(history.undo_depth(), 1);

    project.scenes.insert("main".into(), scene.clone());
    assert_disk(&project);
    let restored = history.undo(scene).unwrap();
    assert!(
        matches!(restored.entities[0].components.last(), Some(Component::Tilemap { map }) if map == "level_map")
    );
}

#[test]
fn authored_transform_can_be_added_even_when_loading_supplies_a_default() {
    let mut project = fixture();
    let mut scene = project.scenes["main"].clone();
    let world = spawn(&project);
    assert_eq!(world.world_ref().query::<&Transform>().iter().count(), 2);
    assert!(!snapshot(&project, &world).entities[0]
        .components
        .iter()
        .any(|c| matches!(c, Component::Transform { .. })));
    let mut history = SceneHistory::new();
    let transform = ComponentKind::Transform.default_component().unwrap();
    assert!(command(ComponentEdit::Add(transform))
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    project.scenes.insert("main".into(), scene);
    assert_disk(&project);
    assert!(snapshot(&project, &spawn(&project)).entities[0]
        .components
        .iter()
        .any(|c| matches!(c, Component::Transform { .. })));
}

#[test]
fn removing_unresolved_tilemap_is_intentional_and_never_resurrects_on_save() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(Component::Tilemap {
            map: "missing_map".into(),
        });
    let world = spawn(&project);
    assert_eq!(world.world_ref().query::<&Tilemap>().iter().count(), 0);
    let mut scene = snapshot(&project, &world);
    let before = scene.clone();
    let mut history = SceneHistory::new();
    assert!(command(ComponentEdit::Remove(ComponentKind::Tilemap))
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    project.scenes.insert("main".into(), scene.clone());
    assert_disk(&project);
    let restored = history.undo(scene).unwrap();
    assert_eq!(json(&restored), json(&before));
}

#[test]
fn empty_or_unknown_map_and_invalid_component_values_preserve_document_and_redo() {
    let project = fixture();
    let mut scene = project.scenes["main"].clone();
    let before = json(&scene);
    let mut history = SceneHistory::new();
    history.record(scene.clone());
    history.undo(scene.clone());
    for component in [
        Component::Tilemap { map: String::new() },
        Component::Tilemap {
            map: "missing".into(),
        },
        Component::Transform {
            x: f32::NAN,
            y: 0.0,
            rotation: 0.0,
            scale: 1.0,
        },
        Component::Sprite {
            color: [f32::INFINITY; 4],
            width: 32.0,
            height: 32.0,
            texture: None,
            layer: 0.0,
            material: None,
            animation_clip: None,
        },
        Component::Parent {
            parent: "hero".into(),
        },
    ] {
        assert!(command(ComponentEdit::Add(component))
            .execute(&mut scene, &project, &mut history)
            .is_err());
        assert_eq!(json(&scene), before);
        assert_eq!(history.undo_depth(), 0);
        assert!(history.can_redo());
    }
    assert!(ComponentKind::Tilemap.default_component().is_none());
}

#[test]
fn duplicate_add_or_absent_remove_does_not_overwrite_values_or_clear_redo() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(components().remove(1));
    let mut scene = project.scenes["main"].clone();
    let before = json(&scene);
    let mut history = SceneHistory::new();
    history.record(scene.clone());
    history.undo(scene.clone());
    assert!(!command(ComponentEdit::Add(
        ComponentKind::Sprite.default_component().unwrap()
    ))
    .execute(&mut scene, &project, &mut history)
    .unwrap());
    assert!(!command(ComponentEdit::Remove(ComponentKind::Role))
        .execute(&mut scene, &project, &mut history)
        .unwrap());
    assert_eq!(json(&scene), before);
    assert!(history.can_redo());
    assert_eq!(history.undo_depth(), 0);
}

#[test]
fn removing_then_readding_sprite_does_not_revive_old_texture_or_animation() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0].components = components();
    let mut scene = project.scenes["main"].clone();
    let mut history = SceneHistory::new();
    command(ComponentEdit::Remove(ComponentKind::Sprite))
        .execute(&mut scene, &project, &mut history)
        .unwrap();
    project.scenes.insert("main".into(), scene.clone());
    assert_disk(&project);
    assert_eq!(
        spawn(&project)
            .world_ref()
            .query::<&Sprite>()
            .iter()
            .count(),
        0
    );
    command(ComponentEdit::Add(
        ComponentKind::Sprite.default_component().unwrap(),
    ))
    .execute(&mut scene, &project, &mut history)
    .unwrap();
    project.scenes.insert("main".into(), scene);
    assert_disk(&project);
    assert!(project.scenes["main"].entities[0]
        .components
        .iter()
        .any(|c| matches!(
            c,
            Component::Sprite {
                texture: None,
                animation_clip: None,
                ..
            }
        )));
}

#[test]
fn tilemap_addition_rebuilds_one_entity_and_preserves_role_and_transform() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0].components =
        vec![components()[0].clone(), components()[2].clone()];
    let mut scene = project.scenes["main"].clone();
    let mut history = SceneHistory::new();
    command(ComponentEdit::Add(Component::Tilemap {
        map: "level_map".into(),
    }))
    .execute(&mut scene, &project, &mut history)
    .unwrap();
    project.scenes.insert("main".into(), scene.clone());
    let world = spawn(&project);
    assert_eq!(world.world_ref().query::<&SourceId>().iter().count(), 2);
    assert_eq!(world.world_ref().query::<&Tilemap>().iter().count(), 1);
    assert_eq!(
        world
            .world_ref()
            .query::<&kiln_runtime::Role>()
            .iter()
            .count(),
        1
    );
    assert_eq!(json(&snapshot(&project, &world)), json(&scene));
    assert_disk(&project);
}

#[test]
fn role_value_edit_is_trimmed_undoable_and_persists_through_projection() {
    let mut project = fixture();
    let mut before = project.scenes["main"].clone();
    before.entities[0].components.push(Component::Role {
        role: "player".into(),
    });
    project.scenes.insert("main".into(), before.clone());
    let mut scene = before.clone();
    let mut history = SceneHistory::new();
    assert!(command(ComponentEdit::SetRole {
        role: "  enemy.guard  ".into(),
    })
    .execute(&mut scene, &project, &mut history)
    .unwrap());
    assert_eq!(history.undo_depth(), 1);
    assert!(scene.entities[0]
        .components
        .iter()
        .any(|component| matches!(component, Component::Role { role } if role == "enemy.guard")));

    assert!(!command(ComponentEdit::SetRole {
        role: "enemy.guard".into(),
    })
    .execute(&mut scene, &project, &mut history)
    .unwrap());
    assert_eq!(history.undo_depth(), 1);

    project.scenes.insert("main".into(), scene.clone());
    let world = spawn(&project);
    assert_eq!(json(&snapshot(&project, &world)), json(&scene));
    assert_eq!(
        world
            .world_ref()
            .query::<&kiln_runtime::Role>()
            .iter()
            .count(),
        1
    );
    assert_disk(&project);

    let undone = history.undo(scene).unwrap();
    assert_eq!(json(&undone), json(&before));
}

#[test]
fn stale_scene_missing_entity_or_duplicate_components_fail_without_history_changes() {
    let mut project = fixture();
    let mut scene = project.scenes["main"].clone();
    let original = json(&scene);
    let mut history = SceneHistory::new();
    let edit = ComponentEdit::Add(ComponentKind::Role.default_component().unwrap());
    for command in [
        ComponentCommand {
            scene_id: "other".into(),
            entity_id: "hero".into(),
            edit: edit.clone(),
        },
        ComponentCommand {
            scene_id: "main".into(),
            entity_id: "nested_only".into(),
            edit,
        },
    ] {
        assert!(command.execute(&mut scene, &project, &mut history).is_err());
        assert_eq!(json(&scene), original);
        assert_eq!(history.undo_depth(), 0);
    }
    let duplicated = scene.entities[0].clone();
    scene.entities.push(duplicated);
    let before = json(&scene);
    assert!(command(ComponentEdit::Remove(ComponentKind::Sprite))
        .execute(&mut scene, &project, &mut history)
        .is_err());
    assert_eq!(json(&scene), before);
    scene = project.scenes["main"].clone();
    for _ in 0..2 {
        scene.entities[0]
            .components
            .push(ComponentKind::Role.default_component().unwrap());
    }
    project.scenes.insert("main".into(), scene.clone());
    let before = json(&scene);
    assert!(command(ComponentEdit::Remove(ComponentKind::Role))
        .execute(&mut scene, &project, &mut history)
        .is_err());
    assert_eq!(json(&scene), before);
    assert_eq!(history.undo_depth(), 0);
}
