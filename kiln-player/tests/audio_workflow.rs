//! Real import/save/reopen/KAR loading and scene/entity script audio acceptance.
#![cfg(feature = "audio")]

use kiln_core::{audio::AudioBus, Component, Entity, Project, Scene};
use kiln_editor::{audio_editor::import_audio, project_files};
use kiln_ir::Value;
use kiln_runtime::{preferences::PlayerPreferences, GameWorld, SliceSession, SourceId};
use std::{collections::HashMap, path::Path};

fn tone(frequency: f64, seconds: f64) -> Vec<u8> {
    let data: Vec<u8> = (0..(48_000.0 * seconds) as usize)
        .flat_map(|index| {
            (((index as f64 * frequency * std::f64::consts::TAU / 48_000.0).sin() * 8192.0) as i16)
                .to_le_bytes()
        })
        .collect();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend(96_000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes
}

fn author(root: &Path, source: &Path) -> Project {
    std::fs::write(source.join("hit.wav"), tone(880.0, 0.15)).unwrap();
    std::fs::write(source.join("theme.wav"), tone(220.0, 0.5)).unwrap();
    let mut project = Project {
        id: "audio-authoring-2d".into(),
        name: "Audio authoring".into(),
        ..Default::default()
    };
    import_audio(&mut project, root, &source.join("hit.wav"), "hit", false).unwrap();
    import_audio(&mut project, root, &source.join("theme.wav"), "theme", true).unwrap();
    project.audio_assets[0].bus = "Combat".into();
    project.audio_assets[0].volume = 0.8;
    project.audio_mixer.buses.insert(
        "Combat".into(),
        AudioBus {
            volume: 0.7,
            ..Default::default()
        },
    );
    assert!(import_audio(&mut project, root, &source.join("hit.wav"), "hit", false).is_err());
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(
        root.join("scripts/main.ks"),
        r#"
if get_frame_count() == 1 { play_music("theme"); }
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("scripts/emitter.ks"),
        r#"
if script_phase() == "start" {
    set_volume(0.5);
    set_bus_volume("Combat", 0.5);
    setting_set("language", "sq");
    play_sound("hit"); play_sound("hit");
}
if script_phase() == "update" {
    state_set("music", music_playing());
    state_set("hit", sound_playing("hit"));
    state_set("language", setting_get("language", "en"));
    state_set("master", get_bus_volume("Master"));
}
"#,
    )
    .unwrap();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("scripts/main.ks".into()),
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "emitter".into(),
                name: "Emitter".into(),
                components: vec![Component::Script {
                    path: "scripts/emitter.ks".into(),
                    properties: HashMap::new(),
                }],
            }],
        },
    );
    project_files::save(&project, &root.join("kiln.toml")).unwrap();
    project
}

fn session(game: &kiln_player::loader::LoadedGame, dir: &Path) -> SliceSession {
    let mut session = SliceSession::with_scene_sources(&game.project, "main", |path| {
        String::from_utf8(game.assets[path].clone()).map_err(|error| error.to_string())
    })
    .unwrap();
    assert!(session
        .host_mut()
        .configure_audio(&game.project, |path| Ok(game.assets[path].clone()))
        .is_empty());
    session.host_mut().set_save_dir(dir);
    session
}

fn execute(game: &kiln_player::loader::LoadedGame, dir: &Path) -> Vec<f32> {
    let mut session = session(game, dir);
    let mut world = GameWorld::new();
    world.spawn((SourceId::new("emitter"), kiln_core::Transform::default()));
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("emitter", "music"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        session.entity_script_state("emitter", "hit"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        session.entity_script_state("emitter", "language"),
        Some(&Value::Str("sq".into()))
    );
    assert_eq!(
        session.entity_script_state("emitter", "master"),
        Some(&Value::F64(0.5))
    );
    let audio = session.host_mut().audio_mut().unwrap();
    assert_eq!(audio.active_source_count(), 3);
    let samples = audio.render_samples(4800).unwrap();
    assert!(samples.iter().any(|sample| sample.abs() > 0.1));
    assert_eq!(
        PlayerPreferences::load(dir)
            .unwrap()
            .audio
            .unwrap()
            .master_volume,
        0.5
    );
    let mut restarted = self::session(game, dir);
    assert_eq!(
        restarted.host_mut().audio_mut().unwrap().master_volume(),
        0.5
    );
    assert_eq!(
        restarted.host_mut().player_setting("language"),
        Some(Value::Str("sq".into()))
    );
    samples
}

#[test]
fn audio_authoring_roundtrip_and_packed_player_produce_identical_pcm() {
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let project = author(root.path(), source.path());
    let reopened = project_files::load(root.path()).unwrap().project;
    assert_eq!(reopened.audio_assets, project.audio_assets);
    assert_eq!(reopened.audio_mixer, project.audio_mixer);
    assert_eq!(project.audio_assets[0].path, "assets/audio/hit.wav");
    let direct = kiln_player::loader::load(root.path(), None).unwrap();
    let output = tempfile::tempdir().unwrap();
    let path = output.path().join("audio.kar");
    let files = [
        "kiln.toml",
        "assets/audio/hit.wav",
        "assets/audio/theme.wav",
        "scripts/main.ks",
        "scripts/emitter.ks",
    ];
    kiln_assets::Archive::new(path.clone())
        .pack(&files.map(|path| root.path().join(path)))
        .unwrap();
    let packed = kiln_player::loader::load(&path, None).unwrap();
    let prefs = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    assert_eq!(
        execute(&direct, prefs.path()),
        execute(&packed, other.path())
    );
    // Optional reproducible sample output, never required to run the test.
    if let Some(destination) = std::env::var_os("KILN_TASK7_SAMPLE_OUT") {
        let destination = Path::new(&destination);
        let destination = if destination.is_absolute() {
            destination.to_owned()
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(destination)
        };
        for file in files {
            let target = destination.join(file);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(root.path().join(file), target).unwrap();
        }
    }
}

#[test]
fn invalid_imports_and_filename_collisions_leave_the_project_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let mut project = Project::default();
    std::fs::write(external.path().join("broken.wav"), "not audio").unwrap();
    assert!(import_audio(
        &mut project,
        root.path(),
        &external.path().join("broken.wav"),
        "broken",
        false
    )
    .is_err());
    assert!(project.audio_assets.is_empty());
    assert!(!root.path().join("assets/audio/broken.wav").exists());
    std::fs::write(external.path().join("tone.wav"), tone(440.0, 0.1)).unwrap();
    import_audio(
        &mut project,
        root.path(),
        &external.path().join("tone.wav"),
        "first",
        false,
    )
    .unwrap();
    let original = std::fs::read(root.path().join("assets/audio/tone.wav")).unwrap();
    std::fs::write(external.path().join("tone.wav"), tone(880.0, 0.1)).unwrap();
    assert!(import_audio(
        &mut project,
        root.path(),
        &external.path().join("tone.wav"),
        "other",
        false
    )
    .is_err());
    assert_eq!(project.audio_assets.len(), 1);
    assert_eq!(
        std::fs::read(root.path().join("assets/audio/tone.wav")).unwrap(),
        original
    );
    let id = project.audio_assets[0].id.clone();
    let mut browser = kiln_editor::AssetBrowser::new();
    browser.scan(root.path());
    browser
        .rename_asset("assets/audio/tone.wav", "assets/audio/moved.wav")
        .unwrap();
    assert_eq!(
        kiln_editor::editing::rewrite_asset_references(
            &mut project,
            "assets/audio/tone.wav",
            "assets/audio/moved.wav"
        ),
        1
    );
    assert_eq!(project.audio_assets[0].id, id);
    assert_eq!(project.audio_assets[0].name, "first");
    let mut host = kiln_runtime::SliceHost::default();
    assert!(host
        .configure_audio(&project, |path| std::fs::read(root.path().join(path))
            .map_err(|error| error.to_string()))
        .is_empty());
}
