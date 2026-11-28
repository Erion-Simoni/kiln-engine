use kiln_core::{Project, Transform, Vec2};
use kiln_runtime::{
    rig::{solve, IkRequest},
    GameWorld, GlobalTransform, ParentLink, SliceSession, Sprite,
};
fn world() -> GameWorld {
    let p = Project::from_toml(include_str!("../../samples/rigged_cutscene/kiln.toml")).unwrap();
    let mut w = GameWorld::new();
    kiln_runtime::spawn_scene(&mut w, &p, "main", &mut |_, c, _| {
        Sprite::new(c[0], c[1], c[2], c[3])
    });
    w
}
fn handle(w: &GameWorld, id: &str) -> hecs::Entity {
    kiln_runtime::tweens::handle(w, id).unwrap()
}
fn request(target: [f64; 2], bend: f64, weight: f64) -> IkRequest {
    IkRequest {
        bones: ["upper".into(), "lower".into(), "tip".into()],
        target,
        bend,
        weight,
    }
}
fn tip(w: &GameWorld) -> [f32; 2] {
    w.world_ref()
        .get::<&GlobalTransform>(handle(w, "tip"))
        .unwrap()
        .position
        .to_array()
}
fn angles(w: &GameWorld) -> [f32; 2] {
    [
        w.world_ref()
            .get::<&Transform>(handle(w, "upper"))
            .unwrap()
            .rotation,
        w.world_ref()
            .get::<&Transform>(handle(w, "lower"))
            .unwrap()
            .rotation,
    ]
}
#[test]
fn ik_reaches_both_bends_and_clamps_unreachable_targets() {
    for bend in [-1.0, 1.0] {
        let mut w = world();
        solve(&mut w, &request([180.0, 160.0], bend, 1.0)).unwrap();
        let p = tip(&w);
        assert!((p[0] - 180.0).abs() < 0.001 && (p[1] - 160.0).abs() < 0.001);
        assert_eq!(angles(&w)[1].signum(), bend as f32);
    }
    let mut w = world();
    solve(&mut w, &request([1000.0, 120.0], 1.0, 1.0)).unwrap();
    assert_eq!(tip(&w), [240.0, 120.0]);
    solve(&mut w, &request([80.0, 120.0], 1.0, 1.0)).unwrap();
    let p = tip(&w);
    assert!((p[0] - 80.0).abs() < 0.001 && (p[1] - 120.0).abs() < 0.001);
}
#[test]
fn ik_blends_fk_and_handles_rotated_scaled_ancestors_without_partial_failures() {
    let mut w = world();
    let original = angles(&w);
    solve(&mut w, &request([180.0, 160.0], 1.0, 0.0)).unwrap();
    assert_eq!(angles(&w), original);
    solve(&mut w, &request([180.0, 160.0], 1.0, 0.5)).unwrap();
    let half = angles(&w);
    let mut full = world();
    solve(&mut full, &request([180.0, 160.0], 1.0, 1.0)).unwrap();
    let all = angles(&full);
    assert!((half[0] - all[0] * 0.5).abs() < 0.0001 && (half[1] - all[1] * 0.5).abs() < 0.0001);
    let root = handle(&w, "upper");
    let ancestor = w.world_mut().spawn((Transform::new(32.0, 48.0)
        .with_rotation(std::f32::consts::FRAC_PI_2)
        .with_scale(Vec2::splat(2.0)),));
    w.world_mut()
        .insert_one(root, ParentLink(ancestor))
        .unwrap();
    kiln_runtime::update_global_transforms(&mut w).unwrap();
    let origin = w
        .world_ref()
        .get::<&GlobalTransform>(root)
        .unwrap()
        .position;
    let target = [origin.x as f64, origin.y as f64 + 200.0];
    solve(&mut w, &request(target, -1.0, 1.0)).unwrap();
    let p = tip(&w);
    assert!((p[0] as f64 - target[0]).abs() < 0.001 && (p[1] as f64 - target[1]).abs() < 0.001);
    let before = angles(&w);
    w.world_mut().get::<&mut Transform>(ancestor).unwrap().scale = Vec2::new(2.0, 1.0);
    assert!(solve(&mut w, &request(target, 1.0, 1.0)).is_err());
    assert_eq!(angles(&w), before);
}
#[test]
fn queued_sound_and_ik_do_not_escape_failed_or_suspended_callbacks() {
    use std::sync::{Arc, Mutex};
    let sounds = Arc::new(Mutex::new(0));
    let sink = sounds.clone();
    let mut s=SliceSession::new("queue_sound(\"chime\");solve_ik_2d(\"upper\",\"lower\",\"tip\",180.0,160.0,1.0,1.0);let fail=1/0;").unwrap();
    s.host_mut()
        .set_audio_player(Box::new(move |_| *sink.lock().unwrap() += 1));
    let mut w = world();
    let before = angles(&w);
    assert!(s.step(&mut w, (0.0, 0.0), 0.0).is_err());
    assert_eq!(*sounds.lock().unwrap(), 0);
    assert_eq!(angles(&w), before);
    let p = Project::from_toml(include_str!("../../samples/rigged_cutscene/kiln.toml")).unwrap();
    let mut p = p;
    let scene = p.scenes.get_mut("main").unwrap();
    for e in &mut scene.entities {
        e.components
            .retain(|c| !matches!(c, kiln_core::Component::Script { .. }));
    }
    scene.script = Some("main.ks".into());
    let source="queue_sound(\"chime\");solve_ik_2d(\"upper\",\"lower\",\"tip\",180.0,160.0,1.0,1.0);\nstate_set(\"done\",true);";
    let mut s = SliceSession::with_scene_sources(&p, "main", |_| Ok(source.into())).unwrap();
    let sink = sounds.clone();
    s.host_mut()
        .set_audio_player(Box::new(move |_| *sink.lock().unwrap() += 1));
    s.add_source_breakpoint("main.ks", 2).unwrap();
    assert!(matches!(
        s.step(&mut w, (0.0, 0.0), 0.0),
        Err(kiln_ir::VmError::DebugSuspended)
    ));
    assert_eq!(*sounds.lock().unwrap(), 0);
    assert_eq!(angles(&w), before);
    s.debug_continue();
    s.step(&mut w, (0.0, 0.0), 9.0).unwrap();
    assert_eq!(*sounds.lock().unwrap(), 1);
    assert_ne!(angles(&w), before);
}
#[test]
fn invalid_ik_and_physics_ownership_leave_fk_pose_untouched() {
    let mut w = world();
    let h = handle(&w, "lower");
    let before = angles(&w);
    w.world_mut()
        .insert_one(h, kiln_core::physics_data::RigidBody2D::default())
        .unwrap();
    assert!(solve(&mut w, &request([180.0, 160.0], 1.0, 1.0)).is_err());
    assert_eq!(angles(&w), before);
    for r in [
        request([f64::NAN, 0.0], 1.0, 1.0),
        request([0.0, 0.0], 0.0, 1.0),
        request([0.0, 0.0], 1.0, 2.0),
    ] {
        assert!(r.validate().is_err());
    }
}

#[test]
#[cfg(feature = "audio")]
fn queued_sound_reaches_native_decoded_audio_engine() {
    let p = Project::from_toml(include_str!("../../samples/rigged_cutscene/kiln.toml")).unwrap();
    let mut s = SliceSession::new("queue_sound(\"chime\");").unwrap();
    let errors = s.host_mut().configure_audio(&p, |path| {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../samples/rigged_cutscene")
                .join(path),
        )
        .map_err(|e| e.to_string())
    });
    assert!(errors.is_empty(), "{errors:?}");
    let mut w = world();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert!(s.host_mut().audio_mut().unwrap().is_playing("chime"));
}

#[test]
fn queued_attachment_sound_uses_platform_adapter_after_commit() {
    use std::sync::{Arc, Mutex};
    struct Backend(Arc<Mutex<Vec<String>>>);
    impl kiln_runtime::audio_backend::PlatformAudio for Backend {
        fn call(&mut self, id: u32, args: &[kiln_ir::Value]) -> Result<kiln_ir::Value, String> {
            assert_eq!(id, 12);
            self.0
                .lock()
                .unwrap()
                .push(args[0].as_str().unwrap().into());
            Ok(kiln_ir::Value::I64(0))
        }
        fn settings(&self) -> kiln_core::audio::AudioMixerSettings {
            Default::default()
        }
        fn set_settings(&mut self, _: kiln_core::audio::AudioMixerSettings) {}
    }
    let p = Project::from_toml(include_str!("../../samples/rigged_cutscene/kiln.toml")).unwrap();
    let mut s = SliceSession::with_scene_sources(&p, "main", |path| {
        Ok(if path == "scripts/director.ks" {
            "if script_phase()==\"update\" {queue_sound(\"chime\");}"
        } else {
            "// listener"
        }
        .into())
    })
    .unwrap();
    let calls = Arc::new(Mutex::new(vec![]));
    s.host_mut()
        .set_platform_audio(Box::new(Backend(calls.clone())));
    let mut w = world();
    s.step(&mut w, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(calls.lock().unwrap().as_slice(), ["chime"]);
    let mut s = SliceSession::new("queue_sound(\"chime\");let fail=1/0;").unwrap();
    s.host_mut()
        .set_platform_audio(Box::new(Backend(calls.clone())));
    assert!(s.step(&mut w, (0.0, 0.0), 0.0).is_err());
    assert_eq!(calls.lock().unwrap().len(), 1);
}
