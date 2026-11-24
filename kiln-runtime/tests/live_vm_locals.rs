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

#[test]
fn imported_nested_functions_expose_real_frames_and_owned_locals() {
    let sources=HashMap::from([
        ("main.ks","import \"outer.ks\";\nouter(3);"),
        ("outer.ks","import \"inner.ks\";\nfn outer(value) { let adjusted=value+1; return inner(adjusted); }"),
        ("inner.ks","fn inner(dividend) {\nlet zero=0;\nreturn dividend / zero;\n}"),
    ]);
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |path| {
            sources.get(path).map(|s| s.to_string()).ok_or(path.into())
        })
        .unwrap();
    assert!(session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.016)
        .is_err());
    let snapshots = session.live_vm_debug();
    let root = &snapshots[0];
    assert_eq!(
        root.frames
            .iter()
            .map(|f| f.function.as_str())
            .collect::<Vec<_>>(),
        vec!["<entry>", "outer", "inner"]
    );
    assert!(root.frames[0]
        .location
        .as_ref()
        .unwrap()
        .contains("main.ks:2:"));
    assert!(root.frames[1]
        .location
        .as_ref()
        .unwrap()
        .contains("outer.ks:2:"));
    assert!(root.frames[2]
        .location
        .as_ref()
        .unwrap()
        .contains("inner.ks:3:"));
    assert!(root.locals.iter().any(|v| v.name == "adjusted"
        && v.function.as_deref() == Some("outer")
        && v.value == "I64(4)"));
    assert!(root.locals.iter().any(|v| v.name == "dividend"
        && v.function.as_deref() == Some("inner")
        && v.value == "I64(4)"));
}

fn halted(source: &str) -> SliceSession {
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |_| {
            Ok(source.into())
        })
        .unwrap();
    session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.016)
        .unwrap_err();
    session
}
#[test]
fn active_current_frame_excludes_returned_blocks_callers_and_uninitialized_bindings() {
    let session=halted("fn inner(value){if true {let expired=99;} let current=value+2; let not_initialized=current/0;} fn outer(value){let caller=1; return inner(value);} let entry=5; outer(entry);");
    let snapshot = session.live_vm_debug_scoped(true);
    let names = snapshot[0]
        .locals
        .iter()
        .map(|v| v.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["value", "current"]);
    assert!(snapshot[0]
        .locals
        .iter()
        .all(|v| v.in_scope && v.function.as_deref() == Some("inner")));
    assert_eq!(
        session.live_watch("scene", "active:value").unwrap(),
        "I64(5)"
    );
    assert_eq!(
        session
            .live_watch("scene", "expr:scope(\"current\")+1")
            .unwrap(),
        "I64(8)"
    );
    assert!(session
        .live_watch("scene", "local:value")
        .unwrap_err()
        .contains("ambiguous"));
    for name in ["expired", "caller", "entry", "not_initialized"] {
        assert!(
            session
                .live_watch("scene", &format!("active:{name}"))
                .is_err(),
            "{name}"
        );
    }
    assert_eq!(
        session.live_watch("scene", "local:expired").unwrap(),
        "I64(99)"
    );
}
#[test]
fn loop_shadowing_resolves_innermost_and_continue_closes_body_bindings() {
    let session = halted("let i=99; for(let i=0;i<2;i+=1){let fail=1/0;}");
    assert_eq!(session.live_watch("scene", "active:i").unwrap(), "I64(0)");
    assert!(session.live_watch("scene", "active:fail").is_err());
    let session = halted("for(let i=0;i<1;i+=1/0){let body=4;continue;}");
    assert_eq!(session.live_watch("scene", "active:i").unwrap(), "I64(0)");
    assert!(session.live_watch("scene", "active:body").is_err());
}
#[test]
fn active_preview_filters_before_the_allocated_slot_bound() {
    let unused = (0..160)
        .map(|n| format!("let expired{n}={n};"))
        .collect::<String>();
    let session = halted(&format!(
        "if true {{{unused}}} fn fail(value){{let current=value+1;return current/0;}} fail(7);"
    ));
    assert_eq!(session.live_vm_debug()[0].locals.len(), 128);
    let active = session.live_vm_debug_scoped(true);
    assert_eq!(active[0].locals.len(), 2);
    assert!(active[0].locals.iter().all(|v| v.slot >= 160));
    assert_eq!(active[0].omitted_locals, 0);
    assert_eq!(
        session.live_watch("scene", "active:current").unwrap(),
        "I64(8)"
    );
}
#[test]
fn completed_scripts_have_no_active_bindings_but_allocated_values_remain() {
    let mut session =
        SliceSession::with_scene_sources(&project(Some("main.ks"), vec![]), "main", |_| {
            Ok("for(let i=0;i<3;i+=1){let value=i;break;} let retained=7;".into())
        })
        .unwrap();
    session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.016)
        .unwrap();
    assert!(session.live_vm_debug_scoped(true)[0].locals.is_empty());
    assert_eq!(
        session.live_watch("scene", "local:retained").unwrap(),
        "I64(7)"
    );
    assert!(session
        .live_watch("scene", "expr:scope(\"retained\")")
        .is_err());
}
