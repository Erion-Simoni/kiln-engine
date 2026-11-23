//! Watches read actual VM data without compiling/evaluating user expressions.
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
}
