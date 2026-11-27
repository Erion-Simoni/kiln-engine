use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{HostContext, Value, VmError};
use kiln_runtime::camera::{CameraController, CameraEffect, CameraShake};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
use std::collections::HashMap;
fn session(root: &str, actors: usize) -> (SliceSession, GameWorld) {
    let mut p = Project::default();
    let mut world = GameWorld::new();
    let entities = (0..actors)
        .map(|i| {
            let id = format!("actor-{i}");
            world.spawn((SourceId::new(&id), Transform::new(0.0, 0.0)));
            Entity {
                id: id.clone(),
                name: id,
                components: vec![Component::Script {
                    path: "actor.ks".into(),
                    properties: HashMap::new(),
                }],
            }
        })
        .collect();
    p.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            entities,
            nested_scenes: vec![],
        },
    );
    let s = SliceSession::with_scene_sources(&p, "main", |path| {
        Ok(if path == "main.ks" {
            root
        } else {
            "if script_phase()==\"update\" {state_set(\"accepted\",camera_shake(4,3,1,8,17));}"
        }
        .into())
    })
    .unwrap();
    (s, world)
}
#[test]
fn callback_requests_share_one_bound_across_scene_and_attachments() {
    let (mut s, mut world) = session("camera_shake(4,3,1,8,17);", 40);
    s.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    let effects = s.take_camera_effects();
    assert_eq!(effects.len(), 32);
    let accepted = (0..40)
        .filter(|i| {
            s.entity_script_state(&format!("actor-{i}"), "accepted") == Some(&Value::Bool(true))
        })
        .count();
    assert_eq!(accepted, 31);
    assert!(s.take_camera_effects().is_empty());
    s.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(s.take_camera_effects().len(), 32);
}
#[test]
fn pending_callback_commands_are_bounded_and_invalid_requests_do_not_queue() {
    let (mut s,mut world)=session("let accepted=0;for(let i=0;i<40;i+=1){if camera_shake(4,3,1,8,i){accepted+=1;}}state_set(\"accepted\",accepted);",0);
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(s.root_script_state("accepted"), Some(&Value::I64(32)));
    assert_eq!(s.take_camera_effects().len(), 32);
    for args in [
        vec![],
        vec![
            Value::F64(-1.0),
            Value::I64(1),
            Value::I64(1),
            Value::I64(8),
            Value::I64(17),
        ],
        vec![
            Value::I64(1),
            Value::I64(1),
            Value::I64(0),
            Value::I64(8),
            Value::I64(17),
        ],
        vec![
            Value::I64(1),
            Value::I64(1),
            Value::I64(1),
            Value::I64(8),
            Value::F64(17.0),
        ],
    ] {
        assert!(s.host_mut().call_host(124, &args).is_err());
        assert!(s.host_mut().commands().is_empty());
    }
}
#[test]
fn suspended_or_failed_callbacks_do_not_deliver_uncommitted_effects() {
    let (mut s, mut world) = session("camera_shake(4,3,1,8,17);\nstate_set(\"finished\",1);", 0);
    s.add_source_breakpoint("main.ks", 2).unwrap();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 0.016),
        Err(VmError::DebugSuspended)
    ));
    assert!(s.take_camera_effects().is_empty());
    let mut controller = CameraController::default();
    assert!(s
        .update_camera(
            &mut controller,
            &Default::default(),
            &world,
            [320.0, 240.0],
            0.016
        )
        .is_err());
    s.debug_continue();
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(s.take_camera_effects().len(), 1);
    let (mut failed, mut world) = session("camera_shake(4,3,1,8,17);let fail=1/0;", 0);
    assert!(failed.step(&mut world, (0.0, 0.0), 0.016).is_err());
    assert!(failed.take_camera_effects().is_empty());
}
#[test]
fn shared_camera_adapter_applies_and_cancels_script_effects() {
    let (mut s, mut world) = session("camera_shake(12,8,1,7,17);", 0);
    let mut c = CameraController::default();
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    let moved = s
        .update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.016)
        .unwrap();
    assert_ne!(moved.position, [0.0, 0.0]);
    assert!(s.take_camera_effects().is_empty());
    let mut doc = kiln_core::ui::UiDocument {
        initial_screen: "pause".into(),
        ..Default::default()
    };
    doc.screens.insert(
        "pause".into(),
        kiln_core::ui::UiScreen {
            pause_game: true,
            ..Default::default()
        },
    );
    *s.host_mut().ui_runtime_mut() = kiln_ui::authored::UiRuntime::new(&doc, "main").unwrap();
    assert_eq!(
        s.update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.5)
            .unwrap()
            .position,
        moved.position
    );
    let (mut stop, mut stop_world) = session("camera_shake_stop();", 0);
    stop.step(&mut stop_world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(stop.take_camera_effects(), vec![CameraEffect::StopShake]);
    stop.step(&mut stop_world, (0.0, 0.0), 0.016).unwrap();
    stop.update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.016)
        .unwrap();
    assert_eq!(
        c.update(&Default::default(), &world, [320.0, 240.0], 0.016)
            .unwrap()
            .position,
        [0.0, 0.0]
    );
    assert!(c
        .apply_effect(CameraEffect::Shake(CameraShake {
            amplitude: [f32::NAN, 1.0],
            duration: 1.0,
            frequency: 1.0,
            seed: 0
        }))
        .is_err());
}

#[test]
fn transition_commands_share_limits_and_callback_atomicity() {
    let (mut s, mut world) = session("for(let i=0;i<40;i+=1){camera_transition(i,0,2,1);}", 0);
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(s.take_camera_effects().len(), 32);
    for source in [
        "camera_transition(80,40,2,1);\nstate_set(\"done\",1);",
        "camera_transition_stop();\nstate_set(\"done\",1);",
    ] {
        let (mut paused, mut w) = session(source, 0);
        paused.add_source_breakpoint("main.ks", 2).unwrap();
        assert!(matches!(
            paused.step(&mut w, (0.0, 0.0), 0.016),
            Err(VmError::DebugSuspended)
        ));
        assert!(paused.take_camera_effects().is_empty());
        paused.debug_continue();
        paused.step(&mut w, (0.0, 0.0), 0.016).unwrap();
        assert_eq!(paused.take_camera_effects().len(), 1);
    }
    let (mut failed, mut w) = session("camera_transition(80,40,2,1);let fail=1/0;", 0);
    assert!(failed.step(&mut w, (0.0, 0.0), 0.016).is_err());
    assert!(failed.take_camera_effects().is_empty());
    for args in [
        vec![],
        vec![Value::I64(0), Value::I64(0), Value::I64(0), Value::I64(1)],
        vec![Value::I64(0), Value::I64(0), Value::I64(2), Value::I64(61)],
    ] {
        assert!(s.host_mut().call_host(126, &args).is_err());
        assert!(s.host_mut().commands().is_empty());
    }
}
#[test]
fn transition_adapter_holds_pause_and_script_stop_restores_authored_camera() {
    let (mut s, mut world) = session("if get_frame_count()==1 {camera_transition(80,40,2,1);}", 0);
    let mut c = CameraController::default();
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    s.update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.016)
        .unwrap();
    let half = s
        .update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.5)
        .unwrap();
    assert_eq!(half.position, [40.0, 20.0]);
    assert_eq!(half.zoom, 1.5);
    let mut doc = kiln_core::ui::UiDocument {
        initial_screen: "pause".into(),
        ..Default::default()
    };
    doc.screens.insert(
        "pause".into(),
        kiln_core::ui::UiScreen {
            pause_game: true,
            ..Default::default()
        },
    );
    *s.host_mut().ui_runtime_mut() = kiln_ui::authored::UiRuntime::new(&doc, "main").unwrap();
    assert_eq!(
        s.update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 1.0)
            .unwrap()
            .position,
        half.position
    );
    let (mut stop, mut w) = session("camera_transition_stop();", 0);
    stop.step(&mut w, (0.0, 0.0), 0.016).unwrap();
    let restored = stop
        .update_camera(&mut c, &Default::default(), &world, [320.0, 240.0], 0.016)
        .unwrap();
    assert_eq!(restored.position, [0.0; 2]);
    assert_eq!(restored.zoom, 1.0);
}
