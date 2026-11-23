//! Inspect real gameplay VMs after failure/completion without stepping a shadow VM.
use kiln_core::{Component, Entity, Project, Scene};
use kiln_runtime::{GameWorld, SliceSession};
use std::collections::HashMap;

fn project(script: Option<&str>, entities: Vec<Entity>) -> Project {
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: script.map(str::to_owned),
            entities,
            nested_scenes: vec![],
        },
    );
    project
}
#[test]
fn imported_function_failure_exposes_real_parameter_and_local_values() {
    let sources = HashMap::from([
        ("main.ks", "import \"logic.ks\";\nlet input = 3;\nfail(input);"),
        ("logic.ks", "fn fail(amount) {\nlet inspected = amount + 4;\nlet denominator = 0;\nreturn inspected / denominator;\n}"),
    ]);
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |path| {
            sources
                .get(path)
                .map(|s| s.to_string())
                .ok_or(path.to_owned())
        })
        .unwrap();
    let mut world = GameWorld::new();
    let error = session.step(&mut world, (0.0, 0.0), 0.016).unwrap_err();
    assert!(error.to_string().contains("logic.ks"), "{error}");
    let snapshots = session.live_vm_debug();
    let root = &snapshots[0];
    assert!(root.location.as_ref().unwrap().contains("logic.ks:4:"));
    assert!(root
        .locals
        .iter()
        .any(|v| v.name == "amount" && v.value == "I64(3)"));
    assert!(root
        .locals
        .iter()
        .any(|v| v.name == "inspected" && v.value == "I64(7)"));
    assert!(root
        .locals
        .iter()
        .any(|v| v.name == "denominator" && v.value == "I64(0)"));
    assert_eq!(root.locals, session.live_vm_debug()[0].locals);
}
#[test]
fn budget_failure_keeps_inspectable_locals_and_snapshot_is_bounded() {
    let source = "let counter = 9;\nwhile true { continue; }";
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |_| {
            Ok(source.into())
        })
        .unwrap();
    let error = session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.016)
        .unwrap_err();
    assert!(error.to_string().contains("Instruction budget exceeded"));
    assert!(session.live_vm_debug()[0]
        .locals
        .iter()
        .any(|v| v.name == "counter" && v.value == "I64(9)"));
    let source = (0..160)
        .map(|n| format!("let value{n} = {n};\n"))
        .collect::<String>();
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |_| {
            Ok(source.clone())
        })
        .unwrap();
    session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.016)
        .unwrap();
    let snapshot = session.live_vm_debug();
    assert_eq!(snapshot[0].locals.len(), 128);
    assert_eq!(snapshot[0].omitted_locals, 32);
}
#[test]
fn attachment_snapshots_read_distinct_executing_instances() {
    let entities = [("first", 1.0), ("second", 3.0)]
        .into_iter()
        .map(|(id, speed)| Entity {
            id: id.into(),
            name: id.into(),
            components: vec![Component::Script {
                path: "entity.ks".into(),
                properties: HashMap::from([("speed".into(), speed)]),
            }],
        })
        .collect();
    let mut session = SliceSession::with_scene_sources(&project(None, entities), "main", |_| {
        Ok("let speed = get_property(\"speed\");".into())
    })
    .unwrap();
    let mut world = GameWorld::new();
    for id in ["first", "second"] {
        world.spawn((
            kiln_runtime::SourceId::new(id),
            kiln_core::Transform::new(0.0, 0.0),
        ));
    }
    session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    let snapshots = session.live_vm_debug();
    for (owner, value) in [("first", "F64(1.0)"), ("second", "F64(3.0)")] {
        let snapshot = snapshots.iter().find(|v| v.owner == owner).unwrap();
        assert!(
            snapshot
                .locals
                .iter()
                .any(|v| v.name == "speed" && v.value == value),
            "{snapshot:?}"
        );
    }
}
