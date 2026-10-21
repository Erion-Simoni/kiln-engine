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
