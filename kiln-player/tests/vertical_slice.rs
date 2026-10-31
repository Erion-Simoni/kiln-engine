//! End-to-end test of the vertical slice sample.
//!
//! The sample is a real project on disk: a manifest, PNG art and a
//! KilnScript that holds the game rules. This test loads it the same way the
//! standalone player does and plays a few frames, so the sample cannot rot
//! without the tests noticing.

use std::path::PathBuf;

use kiln_player::loader;
use kiln_runtime::{spawn_scene, GameWorld, SliceSession, Sprite};

/// The directory of the vertical slice sample.
fn sample_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("samples")
        .join("vertical_slice")
}

#[test]
fn the_sample_project_loads_with_its_art_and_script() {
    let game = loader::load(&sample_dir(), None).expect("the sample loads");

    assert_eq!(game.project.name, "Kiln Vertical Slice");
    assert_eq!(game.scene_id, "main");

    // The art is real files on disk, not generated colours.
    for art in ["art/hero.png", "art/coin.png", "art/ground.png"] {
        let bytes = game.assets.get(art).expect("the art is loaded");
        let image = kiln_assets::decode_png(bytes).expect("the art decodes");
        assert!(image.width > 0 && image.height > 0);
    }

    // The scene names a script, which is loaded as an asset too.
    let scene = game.project.scenes.get(&game.scene_id).unwrap();
    let script_path = scene.script.as_ref().expect("the scene has a script");
    assert!(game.assets.contains_key(script_path));
}

#[test]
fn playing_the_sample_moves_the_player_and_collects_a_coin() {
    let game = loader::load(&sample_dir(), None).expect("the sample loads");
    let scene = game.project.scenes.get(&game.scene_id).unwrap();
    let source = String::from_utf8(
        game.assets
            .get(scene.script.as_ref().unwrap())
            .expect("the script is loaded")
            .clone(),
    )
    .expect("the script is text");

    let mut session = SliceSession::with_action_map(SliceSession::new(&source)source, game.project.action_map.clone()).expect("the script compiles");

    // Spawn the scene the way the player does, but without GPU textures.
    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    spawn_scene(&mut world, &game.project, &game.scene_id, &mut make_sprite);

    // The player starts at the position of the manifest.
    let start = player_position(&mut world).expect("the scene has a player");

    // Moving right for a few seconds brings the player to the first coin.
    for _ in 0..300 {
        session
            .step(&mut world, (1.0, 0.0), 1.0 / 60.0)
            .expect("the script runs");
    }

    let moved = player_position(&mut world).expect("the player still exists");
    assert!(
        moved.0 > start.0,
        "the player moved right: {start:?} -> {moved:?}"
    );
    assert!(
        session.state().score > 0,
        "the player collected a coin: {:?}",
        session.state()
    );
}

/// The position of the player entity in the world.
fn player_position(world: &mut GameWorld) -> Option<(f32, f32)> {
    use kiln_runtime::{Role, SourceId};
    for (role, transform) in world
        .world_mut()
        .query_mut::<(&Role, &kiln_core::Transform)>()
    {
        if role.as_str() == kiln_runtime::ROLE_PLAYER {
            let _ = &SourceId::new("unused");
            return Some((transform.position.x, transform.position.y));
        }
    }
    None
}

#[test]
fn the_sample_state_survives_a_save_and_load() {
    let game = loader::load(&sample_dir(), None).expect("the sample loads");
    let scene = game.project.scenes.get(&game.scene_id).unwrap();
    let source = String::from_utf8(
        game.assets
            .get(scene.script.as_ref().unwrap())
            .expect("the script is loaded")
            .clone(),
    )
    .expect("the script is text");

    let mut session = SliceSession::with_action_map(SliceSession::new(&source)source, game.project.action_map.clone()).expect("the script compiles");
    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    spawn_scene(&mut world, &game.project, &game.scene_id, &mut make_sprite);

    for _ in 0..300 {
        session
            .step(&mut world, (1.0, 0.0), 1.0 / 60.0)
            .expect("the script runs");
    }
    let score = session.state().score;
    assert!(score > 0, "the player collected a coin");

    // Write the state to a slot and read it back.
    let dir = std::env::temp_dir().join("kiln-vertical-slice-save");
    let _ = std::fs::remove_dir_all(&dir);
    kiln_runtime::save_state(&dir, 0, session.state()).expect("the state saves");

    let loaded = kiln_runtime::load_state(&dir, 0).expect("the state loads");
    assert_eq!(loaded.score, score);
    assert_eq!(loaded.collected, session.state().collected);
}

#[test]
fn the_sample_art_is_found_by_the_paths_the_manifest_uses() {
    let game = loader::load(&sample_dir(), None).expect("the sample loads");

    // The player uploads every texture the scenes reference, so the paths in
    // the manifest have to match the files on disk.
    let paths = kiln_player::textures::texture_paths(&game.project);
    assert!(paths.contains(&"art/hero.png".to_string()));
    assert!(paths.contains(&"art/coin.png".to_string()));
    assert!(paths.contains(&"art/ground.png".to_string()));

    for path in paths {
        let bytes = game
            .assets
            .get(&path)
            .expect("the texture is in the assets");
        kiln_assets::decode_png(bytes).expect("the texture decodes");
    }
}

#[test]
fn the_script_decides_the_score_not_the_engine() {
    let game = loader::load(&sample_dir(), None).expect("the sample loads");
    let scene = game.project.scenes.get(&game.scene_id).unwrap();
    let source = String::from_utf8(
        game.assets
            .get(scene.script.as_ref().unwrap())
            .expect("the script is loaded")
            .clone(),
    )
    .expect("the script is text");

    // A script that scores nothing leaves the score alone.
    let mut quiet = SliceSession::new("move_player(get_input_x(), 0.0);").unwrap();
    let mut world = sample_world(&game);
    for _ in 0..300 {
        quiet.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    }
    assert_eq!(quiet.state().score, 0, "the script decides the score");

    // The sample's script awards a coin, so the score follows the rules.
    let mut session = SliceSession::with_action_map(SliceSession::new(&source)source, game.project.action_map.clone()).expect("the script compiles");
    let mut world = sample_world(&game);
    for _ in 0..300 {
        session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    }
    assert!(session.state().score > 0, "the sample script scores");
}

/// A fresh world holding the sample scene.
fn sample_world(game: &loader::LoadedGame) -> GameWorld {
    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    spawn_scene(&mut world, &game.project, &game.scene_id, &mut make_sprite);
    world
}

#[test]
fn the_sample_runs_from_a_packed_archive() {
    // The player is usually started with `--archive`, so the sample has to
    // survive being packed into an archive and loaded back.
    let dir = std::env::temp_dir().join("kiln-vertical-slice-archive");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the directory is created");

    let sample = sample_dir();
    let mut archive = kiln_assets::Archive::new(dir.join("sample.kar"));
    let files: Vec<PathBuf> = [
        "kiln.toml",
        "scripts/main.ks",
        "art/hero.png",
        "art/coin.png",
        "art/ground.png",
    ]
    .iter()
    .map(|name| sample.join(name))
    .collect();
    archive.pack(&files).expect("the sample packs");

    // The archive holds the manifest and the assets, so the game loads and
    // plays the same way it does from a directory.
    let game = loader::load(&dir.join("sample.kar"), None).expect("the archive loads");
    assert_eq!(game.project.name, "Kiln Vertical Slice");
    assert_eq!(game.scene_id, "main");

    let scene = game.project.scenes.get(&game.scene_id).unwrap();
    let script_path = scene.script.as_ref().expect("the scene has a script");
    let source = String::from_utf8(
        game.assets
            .get(script_path)
            .expect("the script is in the archive")
            .clone(),
    )
    .expect("the script is text");
    let mut session = SliceSession::with_action_map(SliceSession::new(&source)source, game.project.action_map.clone()).expect("the script compiles");

    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], _texture: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3]).with_size(16.0, 16.0)
        };
    spawn_scene(&mut world, &game.project, &game.scene_id, &mut make_sprite);

    for _ in 0..300 {
        session
            .step(&mut world, (1.0, 0.0), 1.0 / 60.0)
            .expect("the script runs");
    }
    assert!(
        session.state().score > 0,
        "the player collected a coin from the archive"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
