use kiln_core::{Component, Entity, Project, Scene};
use kiln_editor::{
    entity_command::{EntityCommand, EntityEdit},
    project_files, SceneHistory,
};

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
