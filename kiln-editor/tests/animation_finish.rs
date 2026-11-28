use kiln_core::{Project, Transform};
use kiln_editor::{
    cutscene::{Cue, CueKind, Cutscene},
    entity_command::{EntityCommand, EntityEdit},
    history::SceneHistory,
};
use kiln_runtime::{GameWorld, GlobalTransform, SliceSession, Sprite};
use std::sync::{Arc, Mutex};
fn doc() -> Cutscene {
    serde_json::from_str(include_str!("../../samples/rigged_cutscene/timeline.json")).unwrap()
}
fn project() -> Project {
    Project::from_toml(include_str!("../../samples/rigged_cutscene/kiln.toml")).unwrap()
}
fn handle(w: &GameWorld, id: &str) -> hecs::Entity {
    kiln_runtime::tweens::handle(w, id).unwrap()
}
fn pose(w: &GameWorld, id: &str) -> Transform {
    *w.world_ref().get::<&Transform>(handle(w, id)).unwrap()
}
fn point(w: &mut GameWorld, id: &str) -> [f32; 2] {
    kiln_runtime::update_global_transforms(w).unwrap();
    w.world_ref()
        .get::<&GlobalTransform>(handle(w, id))
        .unwrap()
        .position
        .to_array()
}
fn session(doc: &Cutscene) -> (Project, SliceSession, GameWorld) {
    let p = project();
    doc.validate_scene(&p.scenes["main"]).unwrap();
    doc.validate_assets(&p).unwrap();
    let source = doc.source().unwrap();
    let s = SliceSession::with_scene_sources(&p, "main", |path| {
        Ok(if path == "scripts/director.ks" {
            source.clone()
        } else if path == "scripts/listener.ks" {
            include_str!("../../samples/rigged_cutscene/scripts/listener.ks").into()
        } else {
            "// root".into()
        })
    })
    .unwrap();
    let mut w = GameWorld::new();
    kiln_runtime::spawn_scene(&mut w, &p, "main", &mut |_, c, _| {
        Sprite::new(c[0], c[1], c[2], c[3])
    });
    (p, s, w)
}
fn press(s: &mut SliceSession, w: &mut GameWorld, key: &str) {
    s.host_mut().set_pressed_keys(&[key.into()]);
    s.step(w, (0.0, 0.0), 0.0).unwrap();
    s.host_mut().set_pressed_keys(&[]);
    s.step(w, (0.0, 0.0), 0.0).unwrap();
}
#[test]
fn cues_blend_ik_and_saved_pause_resume_work_together() {
    let doc = doc();
    let (p, mut s, mut w) = session(&doc);
    let sounds = Arc::new(Mutex::new(vec![]));
    let sink = sounds.clone();
    s.host_mut().set_audio_player(Box::new(move |name| {
        sink.lock().unwrap().push(name.to_owned())
    }));
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(
        s.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(1))
    );
    assert_eq!(point(&mut w, "target"), [160.0, 120.0]);
    s.step(&mut w, (0.0, 0.0), 0.75).unwrap();
    assert_eq!(sounds.lock().unwrap().as_slice(), ["chime"]);
    let target = point(&mut w, "target");
    let tip = point(&mut w, "tip");
    assert!((tip[0] - target[0]).abs() < 0.001 && (tip[1] - target[1]).abs() < 0.001);
    press(&mut s, &mut w, "Space");
    let paused = pose(&w, "upper");
    s.step(&mut w, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(pose(&w, "upper"), paused);
    press(&mut s, &mut w, "ArrowRight");
    assert_eq!(
        s.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(1))
    );
    assert_eq!(sounds.lock().unwrap().len(), 1);
    let saved = s.snapshot(&w).unwrap();
    let decoded = kiln_runtime::GameSave::from_json(&saved.to_json().unwrap()).unwrap();
    let source = doc.source().unwrap();
    let (mut resumed, mut other) = decoded
        .restore(
            &p,
            |path| {
                Ok(if path == "scripts/director.ks" {
                    source.clone()
                } else if path == "scripts/listener.ks" {
                    include_str!("../../samples/rigged_cutscene/scripts/listener.ks").into()
                } else {
                    "// root".into()
                })
            },
            |_| None,
        )
        .unwrap();
    resumed.step(&mut other, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(
        resumed.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(1))
    );
    press(&mut resumed, &mut other, "Space");
    resumed.step(&mut other, (0.0, 0.0), 10.0).unwrap();
    resumed.step(&mut other, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(
        resumed.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(1))
    ); // skipped cue stays silent
    let tip = point(&mut other, "tip");
    assert!((tip[0] - 240.0).abs() < 0.001 && (tip[1] - 120.0).abs() < 0.001);
    press(&mut resumed, &mut other, "r");
    resumed.step(&mut other, (0.0, 0.0), 10.0).unwrap();
    resumed.step(&mut other, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(
        resumed.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(3))
    );
}
#[test]
fn continuous_play_delivers_each_cue_once_and_live_blend_weights_clamp() {
    let doc = doc();
    let (_, mut s, mut w) = session(&doc);
    let sounds = Arc::new(Mutex::new(0));
    let sink = sounds.clone();
    s.host_mut()
        .set_audio_player(Box::new(move |_| *sink.lock().unwrap() += 1));
    s.host_mut()
        .set_game_value("reach_blend", kiln_ir::Value::F64(0.0))
        .unwrap();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(point(&mut w, "target"), [140.0, 80.0]);
    s.host_mut()
        .set_game_value("reach_blend", kiln_ir::Value::F64(10.0))
        .unwrap();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(point(&mut w, "target"), [180.0, 160.0]);
    s.step(&mut w, (0.0, 0.0), 10.0).unwrap();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(
        s.entity_script_state("ready", "markers"),
        Some(&kiln_ir::Value::I64(2))
    );
    assert_eq!(*sounds.lock().unwrap(), 1);
    for _ in 0..3 {
        s.step(&mut w, (0.0, 0.0), 1.0).unwrap();
    }
    assert_eq!(*sounds.lock().unwrap(), 1);
    let c = w
        .world_ref()
        .get::<&Sprite>(handle(&w, "ready"))
        .unwrap()
        .color;
    assert_eq!(c, [0.0, 0.0, 1.0, 1.0]);
}
#[test]
fn scene_preview_changes_draws_and_restores_local_global_and_saved_state() {
    let doc = doc();
    let (_, s, mut w) = session(&doc);
    let before = s.snapshot(&w).unwrap().to_json().unwrap();
    let root = pose(&w, "upper");
    let draws = kiln_editor::cutscene_preview::collect(&mut w, Some((&doc, 2.0)), |world| {
        kiln_runtime::visual::collect_scene_draws(world, [0.0, 0.0, 320.0, 240.0])
    });
    let expected = kiln_editor::cutscene::sample(&doc.tracks[0], 2.0);
    let tip = draws
        .iter()
        .find(|(_, i)| i.color == [0.0, 1.0, 0.0, 1.0])
        .unwrap()
        .1;
    assert!((tip.position[0] - expected[0] as f32).abs() < 0.001);
    assert!((tip.position[1] - expected[1] as f32).abs() < 0.001);
    assert_eq!(pose(&w, "upper"), root);
    assert_eq!(s.snapshot(&w).unwrap().to_json().unwrap(), before);
    // Restoring guard also runs while unwinding a failed renderer.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        kiln_editor::cutscene_preview::collect(&mut w, Some((&doc, 1.0)), |_| {
            panic!("draw failure")
        })
    }));
    assert_eq!(pose(&w, "upper"), root);
}
#[test]
fn rig_creation_is_one_undoable_persistent_transaction() {
    let p = project();
    let mut scene = p.scenes["main"].clone();
    let before = serde_json::to_string(&scene).unwrap();
    let count = scene.entities.len();
    let mut history = SceneHistory::new();
    EntityCommand {
        scene_id: "main".into(),
        edit: EntityEdit::CreateCutoutRig {
            position: [32.0, 64.0],
            length: 40.0,
        },
    }
    .execute(&mut scene, &p, &mut history)
    .unwrap();
    assert_eq!(scene.entities.len(), count + 6);
    assert_eq!(history.undo_depth(), 1);
    let restored: kiln_core::Scene =
        serde_json::from_str(&serde_json::to_string(&scene).unwrap()).unwrap();
    assert_eq!(restored.entities.len(), count + 6);
    kiln_core::hierarchy_transform::world_transforms(&restored).unwrap();
    let undone = history.undo(scene).unwrap();
    assert_eq!(serde_json::to_string(&undone).unwrap(), before);
}
#[test]
fn rigged_sample_source_is_reproducible() {
    let source = doc().source().unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../samples/rigged_cutscene/scripts/director.ks");
    if std::env::var_os("KILN_UPDATE_RIG_SAMPLE").is_some() {
        std::fs::write(&path, &source).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}
#[test]
fn cue_and_blend_validation_is_bounded() {
    let mut d = doc();
    d.cues = vec![
        Cue {
            time: 0.0,
            kind: CueKind::Signal,
            name: "x".into(),
            value: 0.0
        };
        33
    ];
    assert!(d.source().is_err());
    let mut d = doc();
    d.tracks[0].blend.as_mut().unwrap().weight = f64::NAN;
    assert!(d.source().is_err());
    let mut d = doc();
    d.ik[0].weight = 2.0;
    assert!(d.source().is_err());
    let mut d = doc();
    d.cues[1].name = "missing".into();
    assert!(d.validate_assets(&project()).is_err());
}
