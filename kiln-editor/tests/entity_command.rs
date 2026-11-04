use kiln_core::{Component, Entity, Prefab, PrefabInstance, Project, Scene};
use kiln_editor::{
    editing::attach_prefab_instances,
    entity_command::{EntityCommand, EntityEdit},
    project_files,
    scene_document::snapshot_for_scene,
    SceneHistory,
};
use kiln_runtime::{GameWorld, SourceId, Sprite};

fn project() -> Project {
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                Entity {
                    id: "root".into(),
                    name: "Root".into(),
                    components: vec![Component::Transform {
                        x: 10.0,
                        y: 20.0,
                        rotation: 0.5,
                        scale: 2.0,
                    }],
                },
                Entity {
                    id: "child".into(),
                    name: "Child".into(),
                    components: vec![
                        Component::Parent {
                            parent: "root".into(),
                        },
                        Component::Role {
                            role: "spawn".into(),
                        },
                    ],
                },
            ],
        },
    );
    project.scenes.insert(
        "other".into(),
        Scene {
            id: "other".into(),
            name: "Other".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "entity_1".into(),
                name: "Reserved".into(),
                components: vec![],
            }],
        },
    );
    project
}

fn run(
    project: &mut Project,
    edit: EntityEdit,
    history: &mut SceneHistory,
) -> Result<Option<String>, String> {
    let mut scene = project.scenes["main"].clone();
    let result = EntityCommand {
        scene_id: "main".into(),
        edit,
    }
    .execute(&mut scene, project, history)?;
    project.scenes.insert("main".into(), scene);
    Ok(result)
}

#[test]
fn create_duplicate_and_delete_are_undoable_and_preserve_authored_data() {
    let mut project = project();
    let mut history = SceneHistory::new();
    let id = run(
        &mut project,
        EntityEdit::Create {
            name: " New actor ".into(),
            position: [3.0, 4.0],
        },
        &mut history,
    )
    .unwrap()
    .unwrap();
    assert_eq!(id, "entity_2");
    assert_eq!(
        project.scenes["main"].entities.last().unwrap().name,
        "New actor"
    );
    let duplicate = run(
        &mut project,
        EntityEdit::Duplicate {
            entity_id: "root".into(),
        },
        &mut history,
    )
    .unwrap()
    .unwrap();
    let copy = project.scenes["main"]
        .entities
        .iter()
        .find(|e| e.id == duplicate)
        .unwrap();
    assert_eq!(copy.name, "Root Copy");
    assert_eq!(
        serde_json::to_value(&copy.components).unwrap(),
        serde_json::to_value(&project.scenes["main"].entities[0].components).unwrap()
    );
    run(
        &mut project,
        EntityEdit::Delete {
            entity_id: "root".into(),
        },
        &mut history,
    )
    .unwrap();
    assert!(project.scenes["main"]
        .entities
        .iter()
        .all(|e| e.id != "root"));
    assert!(!matches!(
        project.scenes["main"]
            .entities
            .iter()
            .find(|e| e.id == "child")
            .unwrap()
            .components[0],
        Component::Parent { .. }
    ));
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(
        serde_json::to_value(&reopened.scenes["main"].entities).unwrap(),
        serde_json::to_value(&project.scenes["main"].entities).unwrap()
    );
    let restored = history.undo(project.scenes["main"].clone()).unwrap();
    assert_eq!(restored.entities[0].id, "root");
    assert!(history.can_redo());
}

#[test]
fn rename_is_trimmed_undoable_and_keeps_identity_and_components() {
    let mut project = project();
    let original = project.scenes["main"].entities[0].clone();
    let mut history = SceneHistory::new();
    run(
        &mut project,
        EntityEdit::Rename {
            entity_id: "root".into(),
            name: "  Renamed Root  ".into(),
        },
        &mut history,
    )
    .unwrap();
    let renamed = &project.scenes["main"].entities[0];
    assert_eq!(renamed.name, "Renamed Root");
    assert_eq!(renamed.id, original.id);
    assert_eq!(
        serde_json::to_value(&renamed.components).unwrap(),
        serde_json::to_value(&original.components).unwrap()
    );
    assert_eq!(history.undo_depth(), 1);

    run(
        &mut project,
        EntityEdit::Rename {
            entity_id: "root".into(),
            name: "Renamed Root".into(),
        },
        &mut history,
    )
    .unwrap();
    assert_eq!(history.undo_depth(), 1);
    assert!(run(
        &mut project,
        EntityEdit::Rename {
            entity_id: "root".into(),
            name: "  ".into(),
        },
        &mut history,
    )
    .is_err());

    let restored = history.undo(project.scenes["main"].clone()).unwrap();
    assert_eq!(restored.entities[0].name, original.name);
    assert_eq!(restored.entities[0].id, original.id);
}

#[test]
fn reparent_rejects_cycles_and_noops_do_not_change_history() {
    let mut project = project();
    let mut history = SceneHistory::new();
    assert!(run(
        &mut project,
        EntityEdit::Reparent {
            entity_id: "root".into(),
            parent_id: Some("child".into())
        },
        &mut history
    )
    .is_err());
    assert_eq!(history.undo_depth(), 0);
    run(
        &mut project,
        EntityEdit::Reparent {
            entity_id: "child".into(),
            parent_id: Some("root".into()),
        },
        &mut history,
    )
    .unwrap();
    assert_eq!(history.undo_depth(), 0);
    run(
        &mut project,
        EntityEdit::Reparent {
            entity_id: "child".into(),
            parent_id: None,
        },
        &mut history,
    )
    .unwrap();
    assert_eq!(history.undo_depth(), 1);
    run(
        &mut project,
        EntityEdit::Reparent {
            entity_id: "child".into(),
            parent_id: Some("root".into()),
        },
        &mut history,
    )
    .unwrap();
    assert!(project.scenes["main"].entities[1]
        .components
        .iter()
        .any(|component| matches!(component, Component::Parent { parent } if parent == "root")));
    assert_eq!(history.undo_depth(), 2);
    assert!(run(
        &mut project,
        EntityEdit::Delete {
            entity_id: "absent".into()
        },
        &mut history
    )
    .is_err());
    assert_eq!(history.undo_depth(), 2);
}

#[test]
fn prefab_instances_survive_duplicate_save_reopen_runtime_load_and_history() {
    let mut project = project();
    project.prefabs.insert(
        "enemy_prefab".into(),
        Prefab {
            id: "enemy_prefab".into(),
            name: "Enemy".into(),
            root_entity: Entity {
                id: "enemy_template".into(),
                name: "Enemy".into(),
                components: vec![
                    Component::Transform {
                        x: 5.0,
                        y: 7.0,
                        rotation: 0.2,
                        scale: 1.5,
                    },
                    Component::Sprite {
                        color: [0.8, 0.2, 0.1, 1.0],
                        width: 40.0,
                        height: 28.0,
                        texture: Some("art/enemy.png".into()),
                        layer: 3.0,
                        material: None,
                        animation_clip: Some("walk".into()),
                    },
                    Component::Parent {
                        parent: "template_parent".into(),
                    },
                ],
            },
        },
    );
    let mut history = SceneHistory::new();
    let instance_id = run(
        &mut project,
        EntityEdit::InstantiatePrefab {
            prefab_id: "enemy_prefab".into(),
            position: [64.0, 96.0],
        },
        &mut history,
    )
    .unwrap()
    .unwrap();
    let instance = project.scenes["main"]
        .entities
        .iter()
        .find(|entity| entity.id == instance_id)
        .unwrap();
    assert_eq!(instance.name, "Enemy Instance");
    assert!(instance.components.iter().any(|component| matches!(
        component,
        Component::Transform {
            x: 64.0,
            y: 96.0,
            rotation: 0.2,
            scale: 1.5
        }
    )));
    assert!(!instance
        .components
        .iter()
        .any(|component| matches!(component, Component::Parent { .. })));
    assert!(
        matches!(instance.components.last(), Some(Component::PrefabInstance { prefab_id, overrides })
        if prefab_id == "enemy_prefab" && overrides.entity_id == instance_id)
    );

    let duplicate_id = run(
        &mut project,
        EntityEdit::Duplicate {
            entity_id: instance_id.clone(),
        },
        &mut history,
    )
    .unwrap()
    .unwrap();
    let duplicate = project.scenes["main"]
        .entities
        .iter()
        .find(|entity| entity.id == duplicate_id)
        .unwrap();
    assert!(
        matches!(duplicate.components.last(), Some(Component::PrefabInstance { overrides, .. })
        if overrides.entity_id == duplicate_id)
    );

    let restored = history.undo(project.scenes["main"].clone()).unwrap();
    project.scenes.insert("main".into(), restored);
    assert!(!project.scenes["main"]
        .entities
        .iter()
        .any(|entity| entity.id == duplicate_id));
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    let reopened_instance = reopened.scenes["main"]
        .entities
        .iter()
        .find(|entity| entity.id == instance_id)
        .unwrap();
    assert!(
        matches!(reopened_instance.components.last(), Some(Component::PrefabInstance { prefab_id, overrides })
        if prefab_id == "enemy_prefab" && overrides.entity_id == instance_id)
    );
    assert_eq!(
        serde_json::to_value(snapshot_for_scene(
            &spawn(&reopened),
            &Default::default(),
            &Default::default(),
            Some(&reopened),
            Some("main")
        ))
        .unwrap(),
        serde_json::to_value(reopened.scenes["main"].clone()).unwrap()
    );

    let mut world = spawn(&reopened);
    assert_eq!(
        attach_prefab_instances(&mut world, &reopened.scenes["main"]),
        1
    );
    assert!(world
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &PrefabInstance)>()
        .iter()
        .any(|(_, source, instance)| source.0 == instance_id
            && instance.prefab_id == "enemy_prefab"
            && instance.entity_id == instance_id));
    let restored = history.undo(project.scenes["main"].clone()).unwrap();
    assert!(!restored
        .entities
        .iter()
        .any(|entity| entity.id == instance_id));
    let redone = history.redo(restored).unwrap();
    assert!(redone
        .entities
        .iter()
        .any(|entity| entity.id == instance_id));
}

fn spawn(project: &Project) -> GameWorld {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        "main",
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    world
}

#[test]
fn reparent_and_parent_delete_preserve_child_world_transform() {
    let mut project = project();
    {
        let scene = project.scenes.get_mut("main").unwrap();
        scene.entities.push(Entity {
            id: "other-parent".into(),
            name: "Other Parent".into(),
            components: vec![Component::Transform {
                x: 100.0,
                y: -30.0,
                rotation: -0.4,
                scale: 3.0,
            }],
        });
    }
    let before =
        kiln_core::hierarchy_transform::world_transforms(&project.scenes["main"]).unwrap()["child"];
    let mut history = SceneHistory::new();
    run(
        &mut project,
        EntityEdit::Reparent {
            entity_id: "child".into(),
            parent_id: Some("other-parent".into()),
        },
        &mut history,
    )
    .unwrap();
    let reparented =
        kiln_core::hierarchy_transform::world_transforms(&project.scenes["main"]).unwrap()["child"];
    assert!((reparented.position - before.position).length() < 0.0001);
    assert!((reparented.rotation - before.rotation).abs() < 0.0001);
    assert!((reparented.scale - before.scale).length() < 0.0001);

    run(
        &mut project,
        EntityEdit::Delete {
            entity_id: "other-parent".into(),
        },
        &mut history,
    )
    .unwrap();
    let detached =
        kiln_core::hierarchy_transform::world_transforms(&project.scenes["main"]).unwrap()["child"];
    assert!((detached.position - before.position).length() < 0.0001);
    assert!((detached.rotation - before.rotation).abs() < 0.0001);
    assert!((detached.scale - before.scale).length() < 0.0001);
    assert!(project.scenes["main"]
        .entities
        .iter()
        .find(|entity| entity.id == "child")
        .unwrap()
        .components
        .iter()
        .any(|component| matches!(component, Component::Transform { .. })));
}
