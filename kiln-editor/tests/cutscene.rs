use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_editor::cutscene::{sample, Cutscene};
use kiln_editor::script_editor::ScriptEditor;
use kiln_ir::Value;
use kiln_runtime::{GameWorld, SliceSession, SourceId, Sprite};
fn document() -> Cutscene {
    serde_json::from_str(include_str!(
        "../../samples/authored_cutscene/timeline.json"
    ))
    .unwrap()
}
fn project() -> Project {
    let mut p = Project::default();
    let mut scene = Scene {
        id: "main".into(),
        name: "Main".into(),
        script: Some("cutscene.ks".into()),
        nested_scenes: vec![],
        entities: vec![],
    };
    for id in ["actor", "ready", "camera_rig"] {
        let mut components = vec![Component::Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale: 1.0,
        }];
        if id != "camera_rig" {
            components.push(Component::Sprite {
                color: [1.0, 0.0, 0.0, 1.0],
                width: 64.0,
                height: 64.0,
                texture: None,
                layer: 0.0,
                material: None,
                animation_clip: None,
            });
        }
        scene.entities.push(Entity {
            id: id.into(),
            name: id.into(),
            components,
        });
    }
    p.scenes.insert("main".into(), scene);
    for (name, key) in [
        ("pause", "Space"),
        ("restart", "r"),
        ("seek", "ArrowRight"),
        ("cancel", "c"),
    ] {
        p.action_map
            .add_action(
                kiln_core::Action::new(name, name).with_binding(kiln_core::Binding::new(
                    kiln_core::BindingType::Key(key.into()),
                )),
            );
    }
    p
}
fn session(doc: &Cutscene) -> (Project, String, SliceSession, GameWorld) {
    let p = project();
    doc.validate_scene(&p.scenes["main"]).unwrap();
    let source = doc.source().unwrap();
    let s = SliceSession::with_scene_sources(&p, "main", |_| Ok(source.clone())).unwrap();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &p, "main", &mut |_, color, _| {
        Sprite::new(color[0], color[1], color[2], color[3])
    });
    (p, source, s, world)
}
fn actor(world: &GameWorld) -> hecs::Entity {
    world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, id)| id.0 == "actor")
        .unwrap()
        .0
}
fn verify(doc: &Cutscene, world: &GameWorld, time: f64) {
    let h = actor(world);
    let pose = world.world_ref().get::<&Transform>(h).unwrap();
    let color = world.world_ref().get::<&Sprite>(h).unwrap().color;
    let expected: Vec<_> = doc
        .tracks
        .iter()
        .filter(|t| t.entity == "actor")
        .map(|t| sample(t, time))
        .collect();
    assert_eq!(
        pose.position.to_array(),
        [expected[0][0] as f32, expected[0][1] as f32]
    );
    assert_eq!(pose.rotation, expected[1][0] as f32);
    assert_eq!(
        pose.scale.to_array(),
        [expected[2][0] as f32, expected[2][1] as f32]
    );
    assert_eq!(
        color,
        expected[3]
            .iter()
            .map(|v| *v as f32)
            .collect::<Vec<_>>()
            .as_slice()
    );
}
fn press(s: &mut SliceSession, w: &mut GameWorld, key: &str, dt: f64) {
    s.host_mut().set_pressed_keys(&[key.into()]);
    s.step(w, (0.0, 0.0), dt).unwrap();
    s.host_mut().set_pressed_keys(&[]);
}
#[test]
fn generated_sampling_matches_preview_across_keys_and_large_ticks() {
    let doc = document();
    let (_, _, mut s, mut w) = session(&doc);
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    verify(&doc, &w, 0.0);
    for dt in [0.25, 0.75, 0.5, 1.5, 10.0] {
        s.step(&mut w, (0.0, 0.0), dt).unwrap();
        let time = s
            .root_script_state("cutscene_time")
            .unwrap()
            .to_f64()
            .unwrap();
        verify(&doc, &w, time);
    }
    assert_eq!(
        s.root_script_state("cutscene_running"),
        Some(&Value::Bool(false))
    );
    // Completed timelines relinquish ownership of subsequent manual writes.
    let h = actor(&w);
    w.world_mut().get::<&mut Transform>(h).unwrap().rotation = 2.0;
    s.step(&mut w, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(
        w.world_ref().get::<&Transform>(actor(&w)).unwrap().rotation,
        2.0
    );
}
#[test]
fn pause_seek_cancel_restart_and_json_resume_preserve_the_sequence() {
    let doc = document();
    let (p, source, mut s, mut w) = session(&doc);
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    s.step(&mut w, (0.0, 0.0), 0.5).unwrap();
    press(&mut s, &mut w, "Space", 1.0);
    verify(&doc, &w, 0.5);
    s.step(&mut w, (0.0, 0.0), 1.0).unwrap();
    verify(&doc, &w, 0.5);
    press(&mut s, &mut w, "ArrowRight", 1.0);
    verify(&doc, &w, doc.duration() / 2.0);
    let saved = s.snapshot(&w).unwrap();
    let decoded = kiln_runtime::GameSave::from_json(&saved.to_json().unwrap()).unwrap();
    let (mut resumed, mut other) = decoded
        .restore(&p, |_| Ok(source.clone()), |_| None)
        .unwrap();
    resumed.step(&mut other, (0.0, 0.0), 2.0).unwrap();
    verify(&doc, &other, doc.duration() / 2.0);
    press(&mut resumed, &mut other, "Space", 0.0);
    resumed.step(&mut other, (0.0, 0.0), 0.25).unwrap();
    verify(&doc, &other, doc.duration() / 2.0 + 0.25);
    press(&mut resumed, &mut other, "c", 1.0);
    resumed.step(&mut other, (0.0, 0.0), 2.0).unwrap();
    verify(&doc, &other, doc.duration() / 2.0 + 0.25);
    press(&mut resumed, &mut other, "r", 1.0);
    verify(&doc, &other, 0.0);
}
#[test]
fn source_roundtrip_and_guarded_save_reject_manual_or_external_edits() {
    let doc = document();
    let source = doc.source().unwrap();
    assert_eq!(Cutscene::from_source(&source).unwrap(), doc);
    assert!(Cutscene::from_source(&(source.clone() + "// custom\n")).is_err());
    let dir = tempfile::tempdir().unwrap();
    let mut editor = ScriptEditor::default();
    editor
        .stage_cutscene(dir.path(), "cutscene.ks", source.clone())
        .unwrap();
    assert!(editor.is_dirty());
    editor.save().unwrap();
    let reopened = std::fs::read_to_string(dir.path().join("cutscene.ks")).unwrap();
    assert_eq!(Cutscene::from_source(&reopened).unwrap(), doc);
    editor
        .stage_cutscene(dir.path(), "cutscene.ks", source.clone())
        .unwrap();
    editor.source.push('\n');
    std::fs::write(dir.path().join("cutscene.ks"), "// external\n").unwrap();
    assert!(editor.save().is_err());
    let mut other = ScriptEditor::default();
    assert!(other
        .stage_cutscene(dir.path(), "cutscene.ks", source)
        .is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("cutscene.ks")).unwrap(),
        "// external\n"
    );
}
#[test]
fn documents_reject_invalid_bindings_keys_limits_and_controls() {
    let doc = document();
    let mut bad = doc.clone();
    bad.tracks[0].keys[1].time = 0.0;
    assert!(bad.source().is_err());
    let mut bad = doc.clone();
    bad.tracks[0].keys[1].time = f64::NAN;
    assert!(bad.source().is_err());
    let mut bad = doc.clone();
    bad.tracks[2].keys[1].value[0] = 0.0;
    assert!(bad.source().is_err());
    let mut bad = doc.clone();
    bad.actions[1] = bad.actions[0].clone();
    assert!(bad.source().is_err());
    let mut bad = doc.clone();
    bad.tracks.push(bad.tracks[0].clone());
    assert!(bad.source().is_err());
    let mut bad = doc.clone();
    bad.tracks[0].entity = "missing".into();
    assert!(bad.validate_scene(&project().scenes["main"]).is_err());
    let mut p = project();
    p.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(Component::Physics {
            body: Default::default(),
            collider: Default::default(),
        });
    assert!(doc.validate_scene(&p.scenes["main"]).is_err());
    let mut bad = doc;
    bad.tracks[0].keys = vec![bad.tracks[0].keys[0].clone(); 65];
    assert!(bad.source().is_err());
}
#[test]
fn sample_source_is_reproducible() {
    let source = document().source().unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../samples/authored_cutscene/scripts/main.ks");
    if std::env::var_os("KILN_UPDATE_CUTSCENE_SAMPLE").is_some() {
        std::fs::write(&path, &source).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}

#[test]
fn bounded_document_runs_at_default_budget_and_literals_preserve_extreme_values() {
    use kiln_editor::cutscene::{Key, Track};
    use kiln_runtime::tweens::{TweenEase, TweenProperty};
    let mut doc = Cutscene {
        tracks: vec![],
        ..Default::default()
    };
    let mut p = project();
    for i in 0..8 {
        let id = format!("target{i}");
        let mut entity = p.scenes["main"].entities[0].clone();
        entity.id = id.clone();
        p.scenes.get_mut("main").unwrap().entities.push(entity);
        for property in [TweenProperty::Position, TweenProperty::Rotation] {
            let values = if property == TweenProperty::Position {
                vec![f32::MAX as f64, f32::from_bits(1) as f64]
            } else {
                vec![-f32::MAX as f64]
            };
            doc.tracks.push(Track {
                blend: None,
                entity: id.clone(),
                property,
                keys: (0..4)
                    .map(|time| Key {
                        time: time as f64,
                        value: values.clone(),
                        ease: TweenEase::Out,
                    })
                    .collect(),
            });
        }
    }
    doc.validate_scene(&p.scenes["main"]).unwrap();
    let source = doc.source().unwrap();
    assert_eq!(Cutscene::from_source(&source).unwrap(), doc);
    let mut s = SliceSession::with_scene_sources(&p, "main", |_| Ok(source.clone())).unwrap();
    let mut w = GameWorld::new();
    kiln_runtime::spawn_scene(&mut w, &p, "main", &mut |_, c, _| {
        Sprite::new(c[0], c[1], c[2], c[3])
    });
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    s.step(&mut w, (0.0, 0.0), 0.5).unwrap();
    for (_, id, pose) in w
        .world_ref()
        .query::<(hecs::Entity, &SourceId, &Transform)>()
        .iter()
    {
        if id.0.starts_with("target") {
            assert_eq!(pose.position.x, f32::MAX);
            assert_eq!(pose.position.y, f32::from_bits(1));
            assert_eq!(pose.rotation, -f32::MAX);
        }
    }
}
