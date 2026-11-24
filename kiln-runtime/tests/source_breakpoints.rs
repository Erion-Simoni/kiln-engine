use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{Value, VmError};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
use std::collections::HashMap;
fn fixture(
    root: &str,
    entities: Vec<Entity>,
    files: HashMap<&str, &str>,
    budget: u64,
) -> (SliceSession, GameWorld) {
    let mut p = Project {
        script_instruction_budget: budget,
        ..Default::default()
    };
    p.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some(root.into()),
            entities,
            nested_scenes: vec![],
        },
    );
    let mut world = GameWorld::new();
    for e in &p.scenes["main"].entities {
        world.spawn((SourceId::new(&e.id), Transform::new(0.0, 0.0)));
    }
    (
        SliceSession::with_scene_sources(&p, "main", |path| {
            files
                .get(path)
                .map(|s| s.to_string())
                .ok_or(path.to_owned())
        })
        .unwrap(),
        world,
    )
}
#[test]
fn imported_breakpoint_steps_real_vm_and_commits_the_callback_once() {
    let (mut s,mut world)=fixture("main.ks",vec![],HashMap::from([
        ("main.ks","import \"logic.ks\";\nlet result=compute(3);\nstate_set(\"result\",result);\nadd_score(1);"),
        ("logic.ks","fn compute(value){\nlet increment=value+1;\nreturn increment*2;\n}"),
    ]),1000);
    s.add_source_breakpoint("logic.ks", 3).unwrap();
    assert!(s.add_source_breakpoint("logic.ks", 4).is_err());
    assert!(matches!(
        s.step(&mut world, (1.0, 0.0), 0.016),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(s.source_stop().unwrap().source.path, "logic.ks");
    assert_eq!(s.live_watch("scene", "active:increment").unwrap(), "I64(4)");
    assert!(s.snapshot(&world).unwrap_err().contains("suspended"));
    let usage = s.script_instruction_usage();
    assert!(matches!(
        s.step(&mut world, (9.0, 9.0), 99.0),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(usage, s.script_instruction_usage());
    s.debug_step_statement();
    assert!(matches!(
        s.step(&mut world, (9.0, 9.0), 99.0),
        Err(VmError::DebugSuspended)
    ));
    let stop = s.source_stop().unwrap();
    assert_eq!(
        (stop.source.path.as_str(), stop.source.line),
        ("main.ks", 3)
    );
    assert_eq!(s.live_watch("scene", "active:result").unwrap(), "I64(8)");
    s.debug_continue();
    s.step(&mut world, (9.0, 9.0), 99.0).unwrap();
    assert_eq!(s.root_script_state("result"), Some(&Value::I64(8)));
    assert_eq!(s.state().score, 1);
    assert_eq!(s.state().elapsed, 0.016);
    s.snapshot(&world).unwrap();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 0.016),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(s.state().score, 1);
}
fn actor(id: &str) -> Entity {
    Entity {
        id: id.into(),
        name: id.into(),
        components: vec![Component::Script {
            path: "actor.ks".into(),
            properties: Default::default(),
        }],
    }
}
#[test]
fn attachment_resume_preserves_callback_order_events_and_frozen_input() {
    let (mut s,mut world)=fixture("main.ks",vec![actor("a"),actor("b")],HashMap::from([
        ("main.ks","state_set(\"ticks\",state_get(\"ticks\",0)+1);"),
        ("actor.ks","import \"update.ks\";\nif script_phase()==\"start\" {state_set(\"starts\",state_get(\"starts\",0)+1);}\nif script_phase()==\"collision_enter\" {state_set(\"hits\",state_get(\"hits\",0)+1);}\nif script_phase()==\"update\" {update();}"),
        ("update.ks","fn update(){\nstate_set(\"input\",get_input_x());\nstate_set(\"updates\",state_get(\"updates\",0)+1);\nstate_set(\"frame\",get_frame_count());\n}"),
    ]),10000);
    s.queue_collision_event("a".into(), "b".into(), true)
        .unwrap();
    s.add_source_breakpoint("update.ks", 3).unwrap();
    let mut stopped = Vec::new();
    for _ in 0..4 {
        match s.step(
            &mut world,
            if stopped.is_empty() {
                (0.5, 0.0)
            } else {
                (99.0, 0.0)
            },
            if stopped.is_empty() { 0.016 } else { 99.0 },
        ) {
            Err(VmError::DebugSuspended) => {
                stopped.push(s.source_stop().unwrap().owner);
                s.debug_continue();
            }
            Ok(()) => break,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(stopped, vec!["a", "b"]);
    assert_eq!(s.root_script_state("ticks"), Some(&Value::I64(1)));
    for id in ["a", "b"] {
        assert_eq!(s.entity_script_state(id, "starts"), Some(&Value::I64(1)));
        assert_eq!(s.entity_script_state(id, "updates"), Some(&Value::I64(1)));
        assert_eq!(s.entity_script_state(id, "hits"), Some(&Value::I64(1)));
        assert_eq!(s.entity_script_state(id, "input"), Some(&Value::F64(0.5)));
    }
    for id in ["a", "b"] {
        assert_eq!(s.entity_script_state(id, "frame"), Some(&Value::I64(1)));
    }
    assert_eq!(s.state().elapsed, 0.016);
}
#[test]
fn repeated_source_stops_cannot_refresh_the_tick_instruction_budget() {
    let (mut s, mut world) = fixture(
        "main.ks",
        vec![],
        HashMap::from([(
            "main.ks",
            "while true {\nstate_set(\"n\",state_get(\"n\",0)+1);\n}",
        )]),
        60,
    );
    s.add_source_breakpoint("main.ks", 2).unwrap();
    let mut used = 0;
    let mut exhausted = false;
    for _ in 0..100 {
        let result = s.step(&mut world, (0.0, 0.0), 0.016);
        let now = s.script_instruction_usage().0;
        assert!(now >= used);
        used = now;
        match result {
            Err(VmError::DebugSuspended) => s.debug_continue(),
            Err(error) => {
                assert!(
                    error.to_string().contains("shared gameplay tick limit 60"),
                    "{error}"
                );
                exhausted = true;
                break;
            }
            Ok(()) => panic!("Infinite loop completed"),
        }
    }
    assert!(exhausted);
    assert_eq!(used, 60);
}

#[test]
fn stop_abandons_suspended_commands_and_cleanup_does_not_hit_breakpoints() {
    let (mut s,mut world)=fixture("main.ks",vec![actor("a")],HashMap::from([
        ("main.ks","return 0;"),
        ("actor.ks","if script_phase()==\"update\" {\nset_entity_pos(self_entity(),9.0,0.0);\nstate_set(\"later\",1);\n}\nif script_phase()==\"stop\" {game_set(\"stops\",game_get(\"stops\",0)+1);}"),
    ]),1000);
    s.add_source_breakpoint("actor.ks", 3).unwrap();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 0.016),
        Err(VmError::DebugSuspended)
    ));
    s.add_source_breakpoint("actor.ks", 5).unwrap();
    s.stop(&mut world).unwrap();
    assert!(s.source_stop().is_none());
    assert_eq!(s.host_mut().game_value("stops"), Some(&Value::I64(1)));
    let x = world
        .world_ref()
        .query::<(&SourceId, &Transform)>()
        .iter()
        .find(|(id, _)| id.0 == "a")
        .unwrap()
        .1
        .position
        .x;
    assert_eq!(x, 0.0);
}

#[cfg(feature = "physics")]
#[test]
fn resumed_source_stop_does_not_advance_physics_again() {
    use kiln_runtime::{Collider2D, RigidBody2D, RigidBodyType};
    let (mut s, mut world) = fixture(
        "main.ks",
        vec![],
        HashMap::from([(
            "main.ks",
            "let a=1;\nstate_set(\"a\",a);\nstate_set(\"b\",2);",
        )]),
        1000,
    );
    world.spawn((
        SourceId::new("body"),
        Transform::new(0.0, 0.0),
        RigidBody2D {
            body_type: RigidBodyType::Kinematic,
            linear_velocity: [60.0, 0.0],
            ..Default::default()
        },
        Collider2D::default(),
    ));
    s.add_source_breakpoint("main.ks", 2).unwrap();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 1.0 / 60.0),
        Err(VmError::DebugSuspended)
    ));
    let position = |world: &GameWorld| {
        world
            .world_ref()
            .query::<(&SourceId, &Transform)>()
            .iter()
            .find(|(id, _)| id.0 == "body")
            .unwrap()
            .1
            .position
            .x
    };
    let stopped = position(&world);
    assert!(stopped > 0.0);
    s.debug_step_statement();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 1.0),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(position(&world), stopped);
    s.debug_continue();
    s.step(&mut world, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(position(&world), stopped);
}

#[test]
fn source_stops_preserve_operand_stack_across_repeated_function_calls() {
    let (mut s, mut world) = fixture(
        "main.ks",
        vec![],
        HashMap::from([
            (
                "main.ks",
                "import \"logic.ks\";\nstate_set(\"result\",2+compute(3)*compute(4));",
            ),
            ("logic.ks", "fn compute(value){\nreturn (value+1)*2;\n}"),
        ]),
        1000,
    );
    s.add_source_breakpoint("logic.ks", 2).unwrap();
    let mut stops = 0;
    loop {
        match s.step(&mut world, (0.0, 0.0), 0.016) {
            Err(VmError::DebugSuspended) => {
                stops += 1;
                assert!(stops <= 2);
                s.debug_continue();
            }
            Ok(()) => break,
            Err(error) => panic!("{error}"),
        }
    }
    assert_eq!(stops, 2);
    assert_eq!(s.root_script_state("result"), Some(&Value::I64(82)));
}
#[test]
fn queued_signal_payloads_are_delivered_once_across_attachment_source_stops() {
    let (mut s,mut world)=fixture("main.ks",vec![actor("a"),actor("b")],HashMap::from([
        ("main.ks","emit_signal(\"tick\",get_frame_count());"),
        ("actor.ks","if script_phase()==\"signal\" {\nlet value=signal_value();\nstate_set(\"seen\",array_push(state_get(\"seen\",array_new()),value));\n}"),
    ]),10000);
    s.add_source_breakpoint("actor.ks", 3).unwrap();
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    for tick in 1..=2 {
        let mut owners = Vec::new();
        loop {
            match s.step(&mut world, (0.0, 0.0), 0.016) {
                Err(VmError::DebugSuspended) => {
                    owners.push(s.source_stop().unwrap().owner);
                    assert!(owners.len() <= 2);
                    s.debug_continue();
                }
                Ok(()) => break,
                Err(error) => panic!("{error}"),
            }
        }
        assert_eq!(owners, vec!["a", "b"]);
        let expected = Value::Array((1..=tick).map(Value::I64).collect());
        for id in ["a", "b"] {
            assert_eq!(s.entity_script_state(id, "seen"), Some(&expected));
        }
    }
}
