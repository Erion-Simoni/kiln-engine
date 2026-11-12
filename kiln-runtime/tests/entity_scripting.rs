use std::collections::HashMap;

use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::Value;
use kiln_runtime::{GameWorld, SliceSession, SourceId};

fn fixture() -> (Project, HashMap<String, String>, GameWorld) {
    let mut project = Project::default();
    let mut world = GameWorld::new();
    let entities: Vec<_> = [
        ("enemy-a", "enemy.ks", 1.0),
        ("enemy-b", "enemy.ks", 3.0),
        ("projectile", "projectile.ks", 5.0),
        ("trigger", "trigger.ks", 0.0),
    ]
    .into_iter()
    .map(|(id, path, speed)| {
        world.spawn((SourceId::new(id), Transform::new(0.0, 0.0)));
        Entity {
            id: id.into(),
            name: id.into(),
            components: vec![Component::Script {
                path: path.into(),
                properties: HashMap::from([("speed".into(), speed)]),
            }],
        }
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
    project.scenes.insert(
        "next".into(),
        Scene {
            id: "next".into(),
            name: "Next".into(),
            script: None,
            entities: vec![],
            nested_scenes: vec![],
        },
    );
    let sources = HashMap::from([
        ("math.ks".into(), "fn advance(x, speed) { if speed > 0 { return x + speed; } return x; }".into()),
        ("enemy.ks".into(), r#"import "math.ks";
if script_phase() == "start" { state_set("ticks", 0); state_set("samples", array_new()); }
if script_phase() == "update" {
    let ticks = state_get("ticks", 0) + 1;
    state_set("ticks", ticks);
    state_set("frame", get_frame_count());
    state_set("samples", array_push(state_get("samples", array_new()), get_property("speed")));
    let position = get_entity_pos(self_entity());
    set_entity_pos(self_entity(), advance(array_get(position, 0), get_property("speed")), 0.0);
}
if script_phase() == "stop" { state_set("stopped", 1); }
"#.into()),
        ("projectile.ks".into(), r#"if script_phase() == "update" { set_entity_pos(self_entity(), get_elapsed() * get_property("speed"), 0.0); }
if script_phase() == "collision_enter" { emit_signal("hit", event_entity()); }
"#.into()),
        ("trigger.ks".into(), r#"if script_phase() == "signal" { if signal_name() == "hit" { state_set("target", signal_value()); change_scene("next"); } }"#.into()),
    ]);
    (project, sources, world)
}

#[test]
fn shared_entity_scheduler_acceptance_and_editor_player_parity() {
    let (project, sources, mut editor_world) = fixture();
    let authored = project.to_toml().unwrap();
    let (_, _, mut player_world) = fixture();
    let read = |path: &str| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    };
    let mut editor = SliceSession::with_scene_sources(&project, "main", read).unwrap();
    let player_project = Project::from_toml(&authored).unwrap();
    let assets: HashMap<_, _> = sources
        .iter()
        .map(|(path, text)| (path.clone(), text.as_bytes().to_vec()))
        .collect();
    let mut player = SliceSession::with_scene_sources(&player_project, "main", |path| {
        let bytes = assets.get(path).ok_or_else(|| format!("Missing {path}"))?;
        String::from_utf8(bytes.clone()).map_err(|error| error.to_string())
    })
    .unwrap();
    for session in [&mut editor, &mut player] {
        session
            .queue_collision_event("projectile".into(), "trigger".into(), true)
            .unwrap();
    }
    for tick in 0..2 {
        editor
            .step(&mut editor_world, (0.0, 0.0), 1.0 / 60.0)
            .unwrap();
        player
            .step(&mut player_world, (0.0, 0.0), 1.0 / 60.0)
            .unwrap();
        if tick == 0 {
            assert_eq!(editor.take_scene_request(), None);
            assert_eq!(player.take_scene_request(), None);
        }
    }
    assert_eq!(
        editor.entity_script_state("enemy-a", "ticks"),
        Some(&Value::I64(2))
    );
    assert_eq!(
        editor.entity_script_state("enemy-b", "ticks"),
        Some(&Value::I64(2))
    );
    assert_eq!(
        editor.entity_script_state("enemy-a", "frame"),
        Some(&Value::I64(2))
    );
    assert_eq!(
        editor.entity_script_state("enemy-a", "samples"),
        Some(&Value::Array(vec![Value::F64(1.0), Value::F64(1.0)]))
    );
    assert_eq!(
        editor.entity_script_state("enemy-b", "samples"),
        Some(&Value::Array(vec![Value::F64(3.0), Value::F64(3.0)]))
    );
    assert_eq!(
        editor.entity_script_state("trigger", "target"),
        Some(&Value::Str("trigger".into()))
    );
    let positions = |world: &GameWorld| {
        let mut positions: Vec<_> = world
            .world_ref()
            .query::<(&SourceId, &Transform)>()
            .iter()
            .map(|(id, transform)| (id.0.clone(), transform.position.x))
            .collect();
        positions.sort_by(|left, right| left.0.cmp(&right.0));
        positions
    };
    assert_eq!(positions(&editor_world), positions(&player_world));
    assert_eq!(positions(&editor_world)[0], ("enemy-a".into(), 2.0));
    assert_eq!(positions(&editor_world)[1], ("enemy-b".into(), 6.0));
    assert_eq!(editor.take_scene_request(), Some("next".into()));
    assert_eq!(player.take_scene_request(), Some("next".into()));
    editor.stop(&mut editor_world).unwrap();
    assert_eq!(
        editor.entity_script_state("enemy-a", "stopped"),
        Some(&Value::I64(1))
    );
    assert_eq!(project.to_toml().unwrap(), authored);
    let next = SliceSession::with_scene_sources(&project, "next", read).unwrap();
    assert_eq!(next.state().elapsed, 0.0);
    assert!(next.entity_script_debug().is_empty());
}

#[test]
fn modules_reject_cycles_and_report_the_imported_file() {
    let (project, mut sources, _) = fixture();
    sources.insert("math.ks".into(), "import \"enemy.ks\";".into());
    let read = |path: &str| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    };
    let error = match SliceSession::with_scene_sources(&project, "main", read) {
        Ok(_) => panic!("Expected cycle rejection"),
        Err(error) => error,
    };
    assert!(error.contains("cyclic"));
    sources.insert(
        "math.ks".into(),
        "fn advance(x, speed) { return missing; }".into(),
    );
    let read = |path: &str| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    };
    let error = match SliceSession::with_scene_sources(&project, "main", read) {
        Ok(_) => panic!("Expected source diagnostic"),
        Err(error) => error,
    };
    assert!(error.contains("math.ks:1:"), "{error}");
}

#[test]
fn runtime_errors_point_into_the_imported_function_and_recursion_is_rejected() {
    let (project, mut sources, mut world) = fixture();
    sources.insert(
        "math.ks".into(),
        "fn advance(x, speed) { return 1 / 0; }".into(),
    );
    let read = |path: &str| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    };
    let mut session = SliceSession::with_scene_sources(&project, "main", read).unwrap();
    let error = session
        .step(&mut world, (0.0, 0.0), 1.0 / 60.0)
        .unwrap_err()
        .to_string();
    assert!(error.contains("math.ks:1:"), "{error}");
    sources.insert(
        "math.ks".into(),
        "fn advance(x, speed) { return advance(x, speed); }".into(),
    );
    let read = |path: &str| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    };
    let error = match SliceSession::with_scene_sources(&project, "main", read) {
        Ok(_) => panic!("Expected recursion rejection"),
        Err(error) => error,
    };
    assert!(error.contains("Recursive"), "{error}");
}

#[test]
fn expanded_syntax_executes_in_attached_gameplay_scripts() {
    let (project, mut sources, mut world) = fixture();
    sources.insert(
        "enemy.ks".into(),
        r#"
if script_phase() == "update" && get_property("speed") > 0 {
    let i = 0;
    let distance = 0.0;
    while i < 4 {
        i += 1;
        if i == 2 { continue; } else if i > 3 { break; }
        distance += get_property("speed");
    }
    let position = get_entity_pos(self_entity());
    if !(distance == 0) { set_entity_pos(self_entity(), array_get(position, 0) + distance, 0.0); }
}
"#
        .into(),
    );
    let mut session = SliceSession::with_scene_sources(&project, "main", |path| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    })
    .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    let positions: HashMap<_, _> = world
        .world_ref()
        .query::<(&SourceId, &Transform)>()
        .iter()
        .map(|(id, transform)| (id.0.clone(), transform.position.x))
        .collect();
    assert_eq!(positions["enemy-a"], 2.0);
    assert_eq!(positions["enemy-b"], 6.0);
}

#[test]
fn runaway_attached_loop_reports_runtime_budget_error() {
    let (project, mut sources, mut world) = fixture();
    sources.insert("enemy.ks".into(), "while true { continue; }".into());
    let mut session = SliceSession::with_scene_sources(&project, "main", |path| {
        sources
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing {path}"))
    })
    .unwrap();
    let error = session.step(&mut world, (0.0, 0.0), 0.016).unwrap_err();
    assert!(error.to_string().contains("Instruction budget exceeded"));
}
