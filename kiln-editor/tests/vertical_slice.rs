//! End-to-end test of the vertical slice sample in the editor.
//!
//! The editor runs the same game loop as the standalone player, so the
//! sample has to work in both.

use std::path::PathBuf;

use kiln_core::Project;

/// The directory of the vertical slice sample.
fn sample_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("samples")
        .join("vertical_slice")
}

#[test]
fn the_editor_can_play_the_sample_project() {
    let dir = sample_dir();
    let manifest = dir.join("kiln.toml");
    let text = std::fs::read_to_string(&manifest).expect("the sample manifest is readable");
    let project = Project::from_toml(&text).expect("the sample manifest parses");

    let scene_id = project.default_scene_id().expect("the sample has a scene");
    assert_eq!(scene_id, "main");

    // The editor reads the script from the project directory.
    let script_path = project
        .scenes
        .get(&scene_id)
        .and_then(|s| s.script.clone())
        .expect("the sample scene has a script");
    let source = std::fs::read_to_string(dir.join(&script_path)).expect("the script is readable");
    let mut session = kiln_runtime::SliceSession::new(&source).expect("the script compiles");

    // The editor spawns the scene the same way the player does.
    let mut world = kiln_runtime::GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    kiln_runtime::spawn_scene(&mut world, &project, &scene_id, &mut make_sprite);

    // Playing the sample moves the hero and collects a coin.
    for _ in 0..300 {
        session
            .step(&mut world, (1.0, 0.0), 1.0 / 60.0)
            .expect("the script runs");
    }
    assert!(session.state().score > 0, "the sample scores in the editor");
}

#[test]
fn an_inspector_edit_is_written_to_the_scene_and_can_be_undone() {
    let dir = sample_dir();
    let manifest = dir.join("kiln.toml");
    let project = Project::from_toml(
        &std::fs::read_to_string(&manifest).expect("the sample manifest is readable"),
    )
    .expect("the sample manifest parses");
    let scene_id = project.default_scene_id().expect("the sample has a scene");
    let original = project.scenes.get(&scene_id).expect("the scene exists");

    // The editor spawns the scene the way the player does.
    let mut world = kiln_runtime::GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    kiln_runtime::spawn_scene(&mut world, &project, &scene_id, &mut make_sprite);

    let entity_textures = std::collections::HashMap::new();
    let entity_roles = std::collections::HashMap::new();

    // The scene built from the world holds the same entities as the project.
    let built =
        kiln_editor::scene_from_world(&world, &entity_textures, &entity_roles, Some(&project));
    assert_eq!(built.id, scene_id);
    assert_eq!(built.entities.len(), original.entities.len());
    assert_eq!(built.entities[0].id, original.entities[0].id);
    assert_eq!(built.script, original.script);

    // Move an entity, the way the inspector does, and record the scene before
    // the edit so it can be undone.
    let mut history = kiln_editor::SceneHistory::new();
    history.record(built.clone());

    let moved = {
        let world = world.world_mut();
        let mut query = world.query::<(&mut kiln_core::Transform, &mut kiln_runtime::Sprite)>();
        let mut position = None;
        for (transform, _) in query.iter() {
            transform.position.x += 25.0;
            position = Some(transform.position.x);
            break;
        }
        position.expect("the scene has an entity")
    };

    // The edit shows up in the scene that is written to disk.
    let edited =
        kiln_editor::scene_from_world(&world, &entity_textures, &entity_roles, Some(&project));
    let transform = edited.entities[0]
        .components
        .iter()
        .find_map(|c| match c {
            kiln_core::project::Component::Transform { x, .. } => Some(*x),
            _ => None,
        })
        .expect("the entity has a transform");
    assert_eq!(transform, moved);

    // Undo restores the scene as it was before the edit.
    let restored = history.undo(edited).expect("undo");
    let original_transform = restored.entities[0]
        .components
        .iter()
        .find_map(|c| match c {
            kiln_core::project::Component::Transform { x, .. } => Some(*x),
            _ => None,
        })
        .expect("the entity has a transform");
    assert_ne!(
        original_transform, transform,
        "the undo restored the old value"
    );
}
