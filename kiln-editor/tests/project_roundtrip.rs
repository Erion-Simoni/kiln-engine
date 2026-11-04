//! Disk-backed acceptance tests for the first project-lifecycle checkpoint.
use kiln_core::{Component, Entity, Project, Scene};
use kiln_editor::{project_files, scene_from_world, SceneHistory};
use kiln_runtime::{GameWorld, SourceId, Sprite};
use std::collections::HashMap;

fn scene(id: &str, entities: Vec<Entity>) -> Scene {
    Scene {
        id: id.into(),
        name: format!("Authored {id}"),
        entities,
        nested_scenes: Vec::new(),
        script: None,
    }
}
fn entity(id: &str, components: Vec<Component>) -> Entity {
    Entity {
        id: id.into(),
        name: format!("Custom name {id}"),
        components,
    }
}
fn transform(x: f32) -> Component {
    Component::Transform {
        x,
        y: 20.0,
        rotation: 0.3,
        scale: 1.5,
    }
}
fn sprite() -> Component {
    Component::Sprite {
        color: [0.2, 0.4, 0.8, 1.0],
        width: 48.0,
        height: 24.0,
        texture: Some("art/hero.png".into()),
        layer: 7.0,
        material: Some("blue".into()),
        animation_clip: Some("idle".into()),
    }
}
fn sample() -> Project {
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        scene(
            "main",
            vec![
                entity("parent", vec![transform(10.0)]),
                entity(
                    "hero",
                    vec![
                        transform(40.0),
                        sprite(),
                        Component::Parent {
                            parent: "parent".into(),
                        },
                        Component::Role {
                            role: "player".into(),
                        },
                    ],
                ),
                entity(
                    "marker",
                    vec![Component::Role {
                        role: "spawn".into(),
                    }],
                ),
                entity(
                    "unresolved-map",
                    vec![
                        transform(50.0),
                        Component::Tilemap {
                            map: "missing".into(),
                        },
                    ],
                ),
            ],
        ),
    );
    project.scenes.insert(
        "overlay".into(),
        scene(
            "overlay",
            vec![entity("nested-hero", vec![transform(90.0), sprite()])],
        ),
    );
    project
        .scenes
        .get_mut("main")
        .unwrap()
        .nested_scenes
        .push(kiln_core::NestedScene {
            scene_id: "overlay".into(),
            additive: true,
            offset: kiln_core::SceneOffset { x: 100.0, y: 50.0 },
        });
    project
}
fn spawn(project: &Project) -> GameWorld {
    let mut world = GameWorld::new();
    let mut factory = |_entity: &Entity, color: [f32; 4], _texture: Option<&str>| {
        Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
    };
    kiln_runtime::spawn_scene(&mut world, project, "main", &mut factory);
    world
}
fn snapshot(world: &GameWorld, project: &Project) -> Scene {
    scene_from_world(world, &HashMap::new(), &HashMap::new(), Some(project))
}
fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn create_opens_from_either_directory_or_manifest_and_keeps_relative_paths() {
    let parent = tempfile::tempdir().unwrap();
    let destination = parent.path().join("New game");
    let created = project_files::create("New game", &destination, "empty_2d").unwrap();
    let by_dir = project_files::load(&destination).unwrap();
    let by_file = project_files::load(&created.path).unwrap();
    assert_eq!(json(&by_dir.project), json(&by_file.project));
    assert_eq!(
        by_dir.path,
        destination.join("kiln.toml").canonicalize().unwrap()
    );
    assert_eq!(by_dir.project.name, "New game");
    let script = by_dir.project.scenes["main"].script.as_ref().unwrap();
    assert_eq!(script, "scripts/main.ks");
    assert!(destination.join("art").is_dir());
    kiln_runtime::SliceSession::new(&std::fs::read_to_string(destination.join(script)).unwrap())
        .unwrap();
}

#[test]
fn creation_does_not_overwrite_existing_directories_or_files() {
    let parent = tempfile::tempdir().unwrap();
    let destination = parent.path().join("existing");
    std::fs::create_dir(&destination).unwrap();
    assert!(project_files::create("Game", &destination, "empty_2d").is_err());
    let sentinel = destination.join("kiln.toml");
    std::fs::write(&sentinel, "important existing content").unwrap();
    assert!(project_files::create("Game", &destination, "empty_2d").is_err());
    assert_eq!(
        std::fs::read_to_string(sentinel).unwrap(),
        "important existing content"
    );
}

#[test]
fn invalid_creation_input_does_not_leave_a_partial_project() {
    let parent = tempfile::tempdir().unwrap();
    let destination = parent.path().join("new");
    for (name, template) in [
        ("", "empty_2d"),
        ("../escaped", "empty_2d"),
        ("Game", "unknown"),
    ] {
        assert!(project_files::create(name, &destination, template).is_err());
        assert!(!destination.exists());
    }
    assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
}

#[test]
fn save_as_keeps_assets_and_rejects_unimplemented_project_relocation() {
    let parent = tempfile::tempdir().unwrap();
    let created = project_files::create("Game", &parent.path().join("game"), "empty_2d").unwrap();
    let root = created.path.parent().unwrap();
    let copy =
        project_files::save_as(&created.project, &created.path, &root.join("copy.toml")).unwrap();
    assert_eq!(
        json(&project_files::load(&copy).unwrap().project),
        json(&created.project)
    );
    assert!(root.join("scripts/main.ks").exists());
    assert!(project_files::save_as(
        &created.project,
        &created.path,
        &parent.path().join("bad-copy.toml")
    )
    .is_err());
    assert!(!parent.path().join("bad-copy.toml").exists());
}

#[test]
fn failed_save_does_not_truncate_the_original_manifest() {
    let parent = tempfile::tempdir().unwrap();
    let created = project_files::create("Game", &parent.path().join("game"), "empty_2d").unwrap();
    let before = std::fs::read(&created.path).unwrap();
    // Replacing a directory is a real persist failure, independent of root permissions.
    assert!(project_files::save(&created.project, created.path.parent().unwrap()).is_err());
    assert_eq!(std::fs::read(&created.path).unwrap(), before);
    assert_eq!(
        json(&project_files::load(&created.path).unwrap().project),
        json(&created.project)
    );
}

#[test]
fn snapshot_preserves_names_nonvisual_entities_parent_links_and_nested_scenes() {
    let project = sample();
    let world = spawn(&project);
    let saved = snapshot(&world, &project);
    assert_eq!(json(&saved), json(&project.scenes["main"]));
    assert!(!saved.entities.iter().any(|e| e.id == "nested-hero"));
}

#[test]
fn edits_undo_redo_and_disk_reopen_preserve_other_scene_and_component_metadata() {
    let mut project = sample();
    let mut world = spawn(&project);
    let original = snapshot(&world, &project);
    let mut history = SceneHistory::new();
    history.record(original.clone());
    for (id, transform) in world
        .world_mut()
        .query_mut::<(&SourceId, &mut kiln_core::Transform)>()
    {
        if id.0 == "hero" {
            transform.position.x = 123.0;
        }
    }
    let edited = snapshot(&world, &project);
    let mut expected = project.scenes["main"].clone();
    if let Component::Transform { x, .. } = &mut expected.entities[1].components[0] {
        *x = 123.0;
    }
    assert_eq!(json(&edited), json(&expected));
    assert_eq!(
        json(&history.undo(edited.clone()).unwrap()),
        json(&original)
    );
    assert_eq!(json(&history.redo(original).unwrap()), json(&edited));
    project.scenes.insert("main".into(), edited);
    let parent = tempfile::tempdir().unwrap();
    let path = project_files::save(&project, &parent.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(&path).unwrap();
    assert_eq!(json(&reopened.project), json(&project));
}

#[test]
fn root_deletion_and_new_nonvisual_entities_are_reflected_in_the_snapshot() {
    let project = sample();
    let mut world = spawn(&project);
    let marker = world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, id)| id.0 == "marker")
        .map(|(entity, _)| entity)
        .unwrap();
    world.world_mut().despawn(marker).unwrap();
    world.spawn((
        SourceId::new("new-marker"),
        kiln_core::Transform::new(5.0, 6.0),
    ));
    let saved = snapshot(&world, &project);
    assert!(!saved
        .entities
        .iter()
        .any(|e| e.id == "marker" || e.id == "nested-hero"));
    assert!(saved.entities.iter().any(|e| e.id == "new-marker"));
}

#[test]
fn tilemap_loading_augments_existing_identity_instead_of_duplicating_it() {
    let mut project = sample();
    project.tilemaps.insert(
        "terrain".into(),
        kiln_core::TileMap {
            id: "terrain".into(),
            ..Default::default()
        },
    );
    project
        .scenes
        .get_mut("main")
        .unwrap()
        .entities
        .push(entity(
            "map",
            vec![
                transform(70.0),
                Component::Tilemap {
                    map: "terrain".into(),
                },
                Component::Role {
                    role: "terrain".into(),
                },
            ],
        ));
    let mut world = spawn(&project);
    let before = world.world_ref().len();
    let mut resolve = |_id: &str| None;
    kiln_runtime::spawn_tilemaps(
        &mut world,
        &project,
        &project.scenes["main"],
        Default::default(),
        &mut resolve,
    );
    assert_eq!(world.world_ref().len(), before);
    let maps = world
        .world_ref()
        .query::<(&SourceId, &kiln_runtime::Tilemap, &kiln_runtime::Role)>()
        .iter()
        .filter(|(id, _, role)| id.0 == "map" && role.0 == "terrain")
        .count();
    assert_eq!(maps, 1);
    assert_eq!(
        json(&snapshot(&world, &project)),
        json(&project.scenes["main"])
    );
}
