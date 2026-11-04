//! Multi-scene authoring acceptance using the same workspace and snapshots as UI.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_editor::{
    project_files, scene_document::snapshot_for_scene, scene_workspace::SceneWorkspace,
};
use kiln_runtime::{GameWorld, Sprite};
use std::collections::HashMap;

fn scene(id: &str, x: f32) -> Scene {
    Scene {
        id: id.into(),
        name: format!("Authored {id}"),
        script: Some(format!("scripts/{id}.ks")),
        nested_scenes: vec![],
        entities: vec![Entity {
            id: "shared-id".into(),
            name: format!("Entity in {id}"),
            components: vec![
                Component::Transform {
                    x,
                    y: 2.0,
                    rotation: 0.4,
                    scale: 1.5,
                },
                Component::Sprite {
                    color: [0.2, 0.4, 0.8, 1.0],
                    width: 32.0,
                    height: 32.0,
                    texture: Some(format!("art/{id}.png")),
                    layer: 3.0,
                    material: Some("ink".into()),
                    animation_clip: None,
                },
                Component::Role {
                    role: format!("role-{id}"),
                },
            ],
        }],
    }
}
fn project() -> Project {
    let mut project = Project::default();
    for (id, x) in [("a", 10.0), ("b", 80.0)] {
        project.scenes.insert(id.into(), scene(id, x));
    }
    project
}
fn spawn(project: &Project, tabs: &SceneWorkspace) -> GameWorld {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        tabs.active_id().unwrap(),
        &mut |_: &Entity, c: [f32; 4], _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    world
}
fn snapshot(project: &Project, tabs: &SceneWorkspace, world: &GameWorld) -> Scene {
    snapshot_for_scene(
        world,
        &HashMap::new(),
        &HashMap::new(),
        Some(project),
        tabs.active_id(),
    )
}
fn move_to(world: &mut GameWorld, x: f32) {
    for transform in world.world().query::<&mut Transform>().iter() {
        transform.position.x = x;
    }
}
fn x(scene: &Scene) -> f32 {
    scene.entities[0]
        .components
        .iter()
        .find_map(|c| match c {
            Component::Transform { x, .. } => Some(*x),
            _ => None,
        })
        .unwrap()
}
fn json(value: &(impl serde::Serialize + ?Sized)) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn switching_preserves_both_live_scenes_and_reopens_from_disk() {
    let mut project = project();
    let original = project.clone();
    let mut tabs = SceneWorkspace::open(&project);
    let mut world = spawn(&project, &tabs);
    move_to(&mut world, 25.0);
    tabs.switch(&mut project, snapshot(&original, &tabs, &world), 1)
        .unwrap();
    let mut world = spawn(&project, &tabs);
    let before_b = snapshot(&project, &tabs, &world);
    assert_eq!(before_b.id, "b");
    assert_eq!(x(&before_b), 80.0);
    assert_eq!(before_b.entities[0].name, "Entity in b");
    move_to(&mut world, 130.0);
    let edited_b = snapshot(&project, &tabs, &world);
    tabs.switch(&mut project, edited_b, 0).unwrap();
    let world = spawn(&project, &tabs);
    let edited_a = snapshot(&project, &tabs, &world);
    assert_eq!(x(&edited_a), 25.0);
    tabs.capture(&mut project, edited_a).unwrap();
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(x(&reopened.scenes["a"]), 25.0);
    assert_eq!(x(&reopened.scenes["b"]), 130.0);
    for id in ["a", "b"] {
        assert_eq!(reopened.scenes[id].name, original.scenes[id].name);
        assert_eq!(reopened.scenes[id].script, original.scenes[id].script);
        assert_eq!(
            json(&reopened.scenes[id].entities[0].components[1..]),
            json(&original.scenes[id].entities[0].components[1..])
        );
    }
    assert_eq!(reopened.default_scene_id(), original.default_scene_id());
    let reopened_tabs = SceneWorkspace::open(&reopened);
    assert_eq!(reopened_tabs.active_id(), Some("a"));
    assert!(reopened_tabs.tabs.iter().all(|t| !t.dirty));
}

#[test]
fn undo_redo_stacks_survive_switching_and_cannot_cross_scenes() {
    let mut project = project();
    let mut tabs = SceneWorkspace::open(&project);
    let mut a = project.scenes["a"].clone();
    tabs.history.record(a.clone());
    if let Component::Transform { x, .. } = &mut a.entities[0].components[0] {
        *x = 33.0;
    }
    tabs.switch(&mut project, a.clone(), 1).unwrap();
    assert!(!tabs.history.can_undo());
    let mut b = project.scenes["b"].clone();
    tabs.history.record(b.clone());
    if let Component::Transform { x, .. } = &mut b.entities[0].components[0] {
        *x = 95.0;
    }
    let undone_b = tabs.history.undo(b.clone()).unwrap();
    assert_eq!(undone_b.id, "b");
    assert_eq!(x(&undone_b), 80.0);
    tabs.switch(&mut project, undone_b, 0).unwrap();
    assert!(!tabs.history.can_redo());
    let undone_a = tabs.history.undo(a).unwrap();
    assert_eq!(undone_a.id, "a");
    assert_eq!(x(&undone_a), 10.0);
    tabs.switch(&mut project, undone_a, 1).unwrap();
    let current = project.scenes["b"].clone();
    let redone = tabs.history.redo(current).unwrap();
    assert_eq!(redone.id, "b");
    assert_eq!(x(&redone), 95.0);
    assert_eq!(x(&project.scenes["a"]), 10.0);
}

#[test]
fn dirty_flags_follow_live_edits_undo_and_successful_save() {
    let mut project = project();
    let mut tabs = SceneWorkspace::open(&project);
    let mut world = spawn(&project, &tabs);
    let before = snapshot(&project, &tabs, &world);
    tabs.history.record(before.clone());
    move_to(&mut world, 50.0);
    let edited = snapshot(&project, &tabs, &world);
    tabs.refresh_dirty(&project, Some(&edited));
    assert!(tabs.tabs[0].dirty);
    assert!(!tabs.tabs[1].dirty);
    let undone = tabs.history.undo(edited.clone()).unwrap();
    tabs.refresh_dirty(&project, Some(&undone));
    assert!(!tabs.tabs[0].dirty);
    let redone = tabs.history.redo(undone).unwrap();
    tabs.capture(&mut project, redone.clone()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    assert!(project_files::save(&project, &directory.path().join("missing/kiln.toml")).is_err());
    tabs.refresh_dirty(&project, Some(&redone));
    assert!(tabs.tabs[0].dirty);
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    tabs.mark_saved(&project);
    assert!(tabs.tabs.iter().all(|t| !t.dirty));
    assert!(tabs.history.can_undo());
}

#[test]
fn dirty_tab_close_is_blocked_without_dropping_document_or_history() {
    let mut project = project();
    let mut tabs = SceneWorkspace::open(&project);
    let mut edited = project.scenes["a"].clone();
    tabs.history.record(edited.clone());
    edited.name = "Unsaved name".into();
    assert!(tabs
        .close(&mut project, edited.clone(), 0)
        .unwrap_err()
        .contains("Save"));
    assert_eq!(tabs.tabs.len(), 2);
    assert_eq!(tabs.active_id(), Some("a"));
    assert!(tabs.history.can_undo());
    tabs.capture(&mut project, edited.clone()).unwrap();
    tabs.mark_saved(&project);
    tabs.close(&mut project, edited.clone(), 0).unwrap();
    assert_eq!(tabs.active_id(), Some("b"));
    assert_eq!(json(&project.scenes["a"]), json(&edited));
    let current = project.scenes["b"].clone();
    assert!(tabs.close(&mut project, current, 0).is_err());
}

#[test]
fn creating_scene_preserves_edits_and_avoids_closed_scene_id_collisions() {
    let mut project = project();
    project
        .scenes
        .insert("scene_1".into(), scene("scene_1", 7.0));
    let mut tabs = SceneWorkspace::open(&project);
    let current = project.scenes["a"].clone();
    tabs.close(&mut project, current.clone(), 2).unwrap();
    let mut current = current;
    current.name = "Pending edit".into();
    tabs.create(&mut project, Some(current)).unwrap();
    assert_eq!(tabs.active_id(), Some("scene_2"));
    assert_eq!(project.scenes["a"].name, "Pending edit");
    assert_eq!(x(&project.scenes["scene_1"]), 7.0);
    assert!(project.scenes["scene_2"].entities.is_empty());
    assert!(!tabs.history.can_undo());
    tabs.refresh_dirty(&project, None);
    assert!(
        tabs.tabs
            .iter()
            .find(|t| t.scene_id == "scene_2")
            .unwrap()
            .dirty
    );
}

#[test]
fn invalid_or_mismatched_switch_does_not_mutate_the_project() {
    let mut project = project();
    let original = json(&project);
    let mut tabs = SceneWorkspace::open(&project);
    let current = project.scenes["a"].clone();
    tabs.history.record(current.clone());
    assert!(tabs.switch(&mut project, current, 20).is_err());
    let wrong = project.scenes["b"].clone();
    assert!(tabs.switch(&mut project, wrong, 1).is_err());
    assert_eq!(json(&project), original);
    assert_eq!(tabs.active_id(), Some("a"));
    assert_eq!(tabs.history.undo_depth(), 1);
}

#[test]
fn empty_project_can_create_and_persist_its_first_scene() {
    let mut project = Project::default();
    let mut tabs = SceneWorkspace::open(&project);
    assert_eq!(tabs.active_id(), None);
    tabs.create(&mut project, None).unwrap();
    assert_eq!(tabs.active_id(), Some("scene_1"));
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(json(&project), json(&reopened));
}

#[test]
fn closing_and_reopening_a_clean_scene_restores_its_history() {
    let mut project = project();
    let mut tabs = SceneWorkspace::open(&project);
    let current = project.scenes["a"].clone();
    tabs.history.record(current.clone());
    tabs.close(&mut project, current, 0).unwrap();
    assert_eq!(tabs.active_id(), Some("b"));
    assert!(!tabs.history.can_undo());
    let current = project.scenes["b"].clone();
    tabs.open_scene(&mut project, current, "a").unwrap();
    assert_eq!(tabs.active_id(), Some("a"));
    assert_eq!(tabs.history.undo_depth(), 1);
    let current = project.scenes["a"].clone();
    tabs.open_scene(&mut project, current, "a").unwrap();
    assert_eq!(tabs.tabs.len(), 2);
    assert_eq!(tabs.history.undo_depth(), 1);
}

#[test]
fn repeated_untextured_entity_creation_retains_unique_ids_after_switch_and_save() {
    let mut project = project();
    let mut tabs = SceneWorkspace::open(&project);
    let mut world = spawn(&project, &tabs);
    let (first, _) =
        kiln_editor::editing::add_entity(&mut world, &[], [5.0, 6.0], Default::default());
    let (second, _) =
        kiln_editor::editing::add_entity(&mut world, &[], [7.0, 8.0], Default::default());
    assert_ne!(first, second);
    let current = snapshot(&project, &tabs, &world);
    assert_eq!(current.entities.len(), 3);
    tabs.switch(&mut project, current, 1).unwrap();
    let current = project.scenes["b"].clone();
    tabs.switch(&mut project, current, 0).unwrap();
    let mut world = spawn(&project, &tabs);
    let (third, _) =
        kiln_editor::editing::add_entity(&mut world, &[], [9.0, 10.0], Default::default());
    assert_ne!(third, first);
    assert_ne!(third, second);
    let current = snapshot(&project, &tabs, &world);
    assert_eq!(current.entities.len(), 4);
    tabs.capture(&mut project, current).unwrap();
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(reopened.scenes["a"].entities.len(), 4);
    assert_eq!(json(&reopened.scenes["b"]), json(&scene("b", 80.0)));
}
