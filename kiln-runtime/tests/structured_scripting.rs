use kiln_core::{Project, Scene};
use kiln_ir::{HostContext, Value};
use kiln_runtime::{GameSave, GameWorld, SliceSession};

#[test]
fn dictionaries_and_counted_loops_survive_a_real_logical_save_restart() {
    let code = r#"
let actor=state_get("actor",dict_new());
for(let i=0;i<3;i+=1) { actor=dict_set(actor,"health",dict_get(actor,"health",0)+1); }
actor=dict_set(actor,"kind","enemy");
actor=dict_set(actor,"exact",9007199254740993);
state_set("actor",actor);
state_set("keys",dict_keys(actor));
state_set("has",dict_has(actor,"kind"));
"#;
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            entities: vec![],
            nested_scenes: vec![],
        },
    );
    let mut session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(code.into())).unwrap();
    let mut world = GameWorld::new();
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    let snapshot =
        GameSave::from_json(&session.snapshot(&world).unwrap().to_json().unwrap()).unwrap();
    let (mut restored, mut restored_world) = snapshot
        .restore(&project, |_| Ok(code.into()), |_| None)
        .unwrap();
    restored
        .step(&mut restored_world, (0.0, 0.0), 1.0 / 60.0)
        .unwrap();
    let actor = restored
        .host_mut()
        .call_host(57, &[Value::Str("actor".into()), Value::Bool(false)])
        .unwrap();
    let Value::Map(values) = actor else {
        panic!("dictionary lost its type")
    };
    assert_eq!(values["health"], Value::I64(6));
    assert_eq!(values["kind"], Value::Str("enemy".into()));
    assert_eq!(values["exact"], Value::I64(9_007_199_254_740_993));
    assert_eq!(
        restored
            .host_mut()
            .call_host(57, &[Value::Str("keys".into()), Value::Bool(false)])
            .unwrap(),
        Value::Array(vec![
            Value::Str("exact".into()),
            Value::Str("health".into()),
            Value::Str("kind".into())
        ])
    );
    let removed = restored
        .host_mut()
        .call_host(
            114,
            &[Value::Map(values.clone()), Value::Str("kind".into())],
        )
        .unwrap();
    assert_eq!(
        restored
            .host_mut()
            .call_host(113, &[removed, Value::Str("kind".into())])
            .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn dictionary_keys_depth_types_and_integer_precision_are_bounded_and_preserved() {
    let mut session = SliceSession::new("").unwrap();
    let host = session.host_mut();
    let dictionary = host
        .call_host(
            112,
            &[
                Value::Map(Default::default()),
                Value::Str("exact".into()),
                Value::I64(9_007_199_254_740_993),
            ],
        )
        .unwrap();
    assert_eq!(
        host.call_host(
            111,
            &[
                dictionary.clone(),
                Value::Str("exact".into()),
                Value::I64(0)
            ]
        )
        .unwrap(),
        Value::I64(9_007_199_254_740_993)
    );
    assert!(host
        .call_host(
            112,
            &[
                dictionary.clone(),
                Value::Str("x".repeat(257)),
                Value::Bool(true)
            ]
        )
        .is_err());
    assert!(host
        .call_host(
            111,
            &[Value::I64(1), Value::Str("key".into()), Value::I64(0)]
        )
        .is_err());
    let mut nested = dictionary;
    for _ in 0..16 {
        nested = Value::Map(std::collections::BTreeMap::from([("child".into(), nested)]));
    }
    assert!(host
        .call_host(
            112,
            &[
                Value::Map(Default::default()),
                Value::Str("nested".into()),
                nested
            ]
        )
        .is_err());
}

#[test]
fn nested_arrays_stop_before_clone_or_drop_can_overflow_the_native_stack() {
    let mut session=SliceSession::new("let a=array_new(); for(let i=0;i<30;i+=1) {a=array_push(array_new(),a);} state_set(\"bad\",a);").unwrap();
    let error = session
        .step(&mut GameWorld::new(), (0.0, 0.0), 1.0 / 60.0)
        .unwrap_err();
    assert!(error.to_string().contains("limit"));
}

#[test]
fn cooperative_stages_and_one_shot_timers_resume_once_after_logical_save_restart() {
    let code = r#"
    enum Stage { Warmup, Fire }
    if !state_get("started",false) {
        task_schedule("enemy",0.2,Stage.Warmup);
        timer_start("one-shot",0.2);
        state_set("started",true);
    }
    if task_ready("enemy") {
        let stage=task_take("enemy",Stage.Warmup);
        if stage==Stage.Warmup {task_schedule("enemy",0.2,Stage.Fire);}
        else {state_set("fired",state_get("fired",0)+1);}
    }
    if timer_finished("one-shot") {state_set("timer-count",state_get("timer-count",0)+1);}
    "#;
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            entities: vec![],
            nested_scenes: vec![],
        },
    );
    let mut session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(code.into())).unwrap();
    let mut world = GameWorld::new();
    session.step(&mut world, (0.0, 0.0), 0.1).unwrap();
    let saved = GameSave::from_json(&session.snapshot(&world).unwrap().to_json().unwrap()).unwrap();
    let (mut session, mut world) = saved
        .restore(&project, |_| Ok(code.into()), |_| None)
        .unwrap();
    for _ in 0..7 {
        session.step(&mut world, (0.0, 0.0), 0.1).unwrap();
    }
    for key in ["fired", "timer-count"] {
        assert_eq!(
            session
                .host_mut()
                .call_host(57, &[Value::Str(key.into()), Value::I64(0)])
                .unwrap(),
            Value::I64(1)
        );
    }
    assert_eq!(
        session
            .host_mut()
            .call_host(120, &[Value::Str("enemy".into())])
            .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn invalid_task_replacement_preserves_the_existing_deadline_and_payload() {
    let mut session = SliceSession::new("").unwrap();
    let host = session.host_mut();
    host.call_host(
        116,
        &[
            Value::Str("work".into()),
            Value::F64(2.0),
            Value::Str("original".into()),
        ],
    )
    .unwrap();
    for delay in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(host
            .call_host(
                116,
                &[
                    Value::Str("work".into()),
                    Value::F64(delay),
                    Value::Bool(false)
                ]
            )
            .is_err());
    }
    assert_eq!(
        host.call_host(121, &[Value::Str("work".into())]).unwrap(),
        Value::F64(2.0)
    );
    assert_eq!(
        host.call_host(
            118,
            &[Value::Str("work".into()), Value::Str("not yet".into())]
        )
        .unwrap(),
        Value::Str("not yet".into())
    );
    assert_eq!(
        host.call_host(119, &[Value::Str("work".into())]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        host.call_host(119, &[Value::Str("work".into())]).unwrap(),
        Value::Bool(false)
    );
    for index in 0..256 {
        host.call_host(122, &[Value::Str(format!("t{index}")), Value::F64(1.0)])
            .unwrap();
    }
    assert!(host
        .call_host(122, &[Value::Str("overflow".into()), Value::F64(1.0)])
        .is_err());
}
