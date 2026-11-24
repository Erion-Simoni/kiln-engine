//! Selectors and bounded isolated expressions inspect actual VM data.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
use std::collections::HashMap;
fn session(source: &str) -> (SliceSession, GameWorld) {
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
        SliceSession::with_scene_sources(&project, "main", |_| Ok(source.into())).unwrap();
    let mut world = GameWorld::new();
    session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    (session, world)
}
#[test]
fn watches_read_nested_locals_state_and_slots_beyond_snapshot_limit() {
    let (session,_)=session("let bag = dict_set(dict_new(), \"a/b~\", array_push(array_new(), 7)); state_set(\"bag\", bag);");
    assert_eq!(
        session.live_watch("scene", "local:bag/a~1b~0/0").unwrap(),
        "I64(7)"
    );
    assert_eq!(
        session.live_watch("scene", "state:bag/a~1b~0/0").unwrap(),
        "I64(7)"
    );
    assert_eq!(
        session.live_watch("scene", "slot:0").unwrap(),
        "Dictionary (1 entries)"
    );
    let source = (0..160)
        .map(|n| format!("let value{n} = {n};\n"))
        .collect::<String>();
    let (large, _) = self::session(&source);
    assert_eq!(large.live_vm_debug()[0].locals.len(), 128);
    assert_eq!(large.live_watch("scene", "slot:159").unwrap(), "I64(159)");
}
#[test]
fn invalid_or_ambiguous_watches_report_errors_without_mutating_vm() {
    let (session, _) =
        session("fn first(repeated) { return repeated; } fn second(repeated) { return repeated; } first(1); second(2); state_set(\"kept\", 9);");
    assert!(session
        .live_watch("scene", "local:repeated")
        .unwrap_err()
        .contains("ambiguous"));
    assert_eq!(session.live_watch("scene", "slot:0").unwrap(), "I64(1)");
    assert_eq!(session.live_watch("scene", "slot:1").unwrap(), "I64(2)");
    let before = session.live_vm_debug()[0].locals.clone();
    for selector in [
        "state_set(\"kept\",0)",
        "state:missing",
        "slot:-1",
        "slot:999999",
        "slot:0/key",
        "state:kept~2",
        "state:kept/0",
    ] {
        assert!(session.live_watch("scene", selector).is_err(), "{selector}");
    }
    assert!(session.live_watch("missing-owner", "state:kept").is_err());
    assert!(session.live_watch("entity:missing", "state:kept").is_err());
    assert!(session.live_watch("scene", &"x".repeat(513)).is_err());
    assert!(session
        .live_watch("scene", &format!("slot:0{}", "/x".repeat(16)))
        .is_err());
    assert_eq!(session.live_watch("scene", "state:kept").unwrap(), "I64(9)");
    assert_eq!(session.live_vm_debug()[0].locals, before);
}
#[test]
fn watches_select_distinct_entity_vms() {
    let mut project = Project::default();
    let entities = [("first", 1.0), ("second", 3.0)]
        .into_iter()
        .map(|(id, speed)| Entity {
            id: id.into(),
            name: id.into(),
            components: vec![Component::Script {
                path: "actor.ks".into(),
                properties: HashMap::from([("speed".into(), speed)]),
            }],
        })
        .collect();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            entities,
            nested_scenes: vec![],
        },
    );
    let mut session = SliceSession::with_scene_sources(&project, "main", |_| {
        Ok("let speed = get_property(\"speed\"); state_set(\"speed\", speed);".into())
    })
    .unwrap();
    let mut world = GameWorld::new();
    for id in ["first", "second"] {
        world.spawn((SourceId::new(id), Transform::new(0.0, 0.0)));
    }
    session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(
        session.live_watch("entity:first", "local:speed").unwrap(),
        "F64(1.0)"
    );
    assert_eq!(
        session.live_watch("entity:second", "state:speed").unwrap(),
        "F64(3.0)"
    );
    session
        .host_mut()
        .set_game_value("bonus", kiln_ir::Value::I64(2))
        .unwrap();
    assert_eq!(
        session
            .live_watch("entity:first", "expr:speed+game(\"bonus\")")
            .unwrap(),
        "F64(3.0)"
    );
    assert_eq!(
        session
            .live_watch("entity:second", "expr:state(\"speed\")*2")
            .unwrap(),
        "F64(6.0)"
    );
}

#[test]
fn expression_watches_share_arithmetic_and_read_nested_values_without_mutation() {
    let (session, world) = session(
        r#"
let health=9;
let exact=9007199254740993;
let bag=dict_set(dict_new(),"items",array_push(array_new(),7));
state_set("bag",bag);
game_set("bonus",3);
"#,
    );
    let before = session.snapshot(&world).unwrap().to_json().unwrap();
    let position = session.live_vm_debug()[0].instruction;
    let usage = session.script_instruction_usage();
    for (expression, expected) in [
        ("health * 2 + game(\"bonus\")", "I64(21)"),
        ("exact == 9007199254740993", "Bool(true)"),
        (
            "array_get(dict_get(state(\"bag\"),\"items\",false),0) / 2",
            "I64(3)",
        ),
        (
            "array_length(dict_get(bag,\"items\",false)) == 1 && dict_has(bag,\"items\")",
            "Bool(true)",
        ),
        ("false && missing > 0", "Bool(false)"),
        ("true || state(\"missing\")", "Bool(true)"),
        ("local(\"health\") + slot(0)", "I64(18)"),
        ("\"alpha\" < \"beta\"", "Bool(true)"),
    ] {
        assert_eq!(
            session
                .live_watch("scene", &format!("expr:{expression}"))
                .unwrap(),
            expected,
            "{expression}"
        );
    }
    assert_eq!(session.snapshot(&world).unwrap().to_json().unwrap(), before);
    assert_eq!(session.live_vm_debug()[0].instruction, position);
    assert_eq!(session.script_instruction_usage(), usage);
}

#[test]
fn expressions_reject_mutations_statements_ambiguity_and_excessive_work() {
    let (session,world)=session("fn first(repeated) {return repeated;} fn second(repeated) {return repeated;} first(1);second(2);state_set(\"kept\",9);");
    let before = session.snapshot(&world).unwrap().to_json().unwrap();
    for expression in [
        "state_set(\"kept\",0)",
        "false && game_set(\"bad\",1)",
        "print(1)",
        "task_schedule(\"bad\",0,1)",
        "get_score()",
        "first(2)",
        "1; state_set(\"kept\",0)",
        "1; return 2",
        "repeated",
        "slot(-1)",
        "slot(0.0)",
        "state(1)",
        "state(\"missing\")",
        "game(\"missing\")",
        "1/0",
        "9223372036854775807+1",
        "array_get(1,0)",
        "dict_get(1,\"key\",0)",
        "array_get(1)",
        "local(\"missing\")",
    ] {
        assert!(
            session
                .live_watch("scene", &format!("expr:{expression}"))
                .is_err(),
            "{expression}"
        );
    }
    assert!(session
        .live_watch("scene", &format!("expr:{}true", "!".repeat(40)))
        .unwrap_err()
        .contains("depth"));
    assert!(session
        .live_watch("scene", &format!("expr:{}1", "1+".repeat(70)))
        .is_err());
    assert_eq!(
        session.live_watch("scene", "expr:slot(0)+slot(1)").unwrap(),
        "I64(3)"
    );
    assert_eq!(session.snapshot(&world).unwrap().to_json().unwrap(), before);
}

#[test]
fn expression_value_copy_budget_is_enforced_without_affecting_direct_selectors() {
    let (session, _) = session(&format!(
        "let large=\"{}\"; state_set(\"large\",large);",
        "x".repeat(65536)
    ));
    assert!(session
        .live_watch("scene", "expr:state(\"large\")")
        .unwrap_err()
        .contains("byte limit"));
    assert!(session.live_watch("scene", "state:large").unwrap().len() < 200);
}
