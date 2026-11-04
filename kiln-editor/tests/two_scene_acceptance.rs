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
