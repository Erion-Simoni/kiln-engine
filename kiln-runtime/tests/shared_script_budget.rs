//! Aggregate gameplay work cannot multiply the per-invocation allowance.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
fn fixture(count: usize) -> (Project, GameWorld) {
    let mut project = Project::default();
    let mut world = GameWorld::new();
    let entities = (0..count)
        .map(|n| {
            let id = format!("actor-{n}");
            world.spawn((SourceId::new(id.clone()), Transform::new(0.0, 0.0)));
            Entity {
                id: id.clone(),
                name: id,
                components: vec![Component::Script {
                    path: "actor.ks".into(),
                    properties: Default::default(),
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
    (project, world)
}
#[test]
fn ordinary_entity_invocations_share_one_tick_allowance() {
    let (project, mut world) = fixture(4);
    let source="if script_phase() == \"update\" { for (let i = 0; i < 30000; i += 1) { let scratch = i + 1; } state_set(\"done\", true); }";
    let mut session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(source.into())).unwrap();
    assert!(
        session.step(&mut world, (0.0, 0.0), 0.016).is_err(),
        "Four individually bounded scripts must not multiply the tick allowance"
    );
}

const HEAVY: &str = "for (let i = 0; i < 60; i += 1) { let scratch = i + 1; }";
fn create(project: &Project, actor: &str, root: &str) -> SliceSession {
    SliceSession::with_scene_sources(project, "main", |path| {
        Ok(if path == "root.ks" {
            root.into()
        } else {
            actor.into()
        })
    })
    .unwrap()
}
#[test]
fn configurable_budget_caps_root_and_attachments_and_reports_context() {
    for limit in [2000, 10000] {
        let (mut project, mut world) = fixture(4);
        project.script_instruction_budget = limit;
        project.scenes.get_mut("main").unwrap().script = Some("root.ks".into());
        let actor =
            format!("if script_phase() == \"update\" {{ {HEAVY} state_set(\"done\", true); }}");
        let mut session = create(&project, &actor, HEAVY);
        let result = session.step(&mut world, (0.0, 0.0), 0.016);
        let (used, configured) = session.script_instruction_usage();
        assert_eq!(configured, limit);
        assert!(used <= limit);
        if limit == 2000 {
            let message = result.unwrap_err().to_string();
            assert!(
                message.contains("shared gameplay tick limit 2000")
                    && message.contains("actor.ks")
                    && message.contains("entity actor-"),
                "{message}"
            );
            assert_eq!(used, limit);
        } else {
            result.unwrap();
            for n in 0..4 {
                assert_eq!(
                    session.entity_script_state(&format!("actor-{n}"), "done"),
                    Some(&kiln_ir::Value::Bool(true))
                );
            }
        }
    }
}
#[test]
fn tick_allowance_resets_once_per_tick_and_per_call_limit_is_retained() {
    let (mut project, mut world) = fixture(1);
    project.script_instruction_budget = 100;
    let mut session = create(
        &project,
        "if script_phase() == \"update\" { state_set(\"ticks\", state_get(\"ticks\", 0) + 1); }",
        "",
    );
    let mut cumulative = 0;
    for _ in 0..20 {
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        let (used, limit) = session.script_instruction_usage();
        assert!(used <= limit);
        cumulative += used;
    }
    assert!(cumulative > 100);
    assert_eq!(
        session.entity_script_state("actor-0", "ticks"),
        Some(&kiln_ir::Value::I64(20))
    );
    project.script_instruction_budget = 2_000_000;
    project.scenes.get_mut("main").unwrap().script = Some("root.ks".into());
    let mut session = create(&project, "return 0;", "while true { continue; }");
    let message = session
        .step(&mut world, (0.0, 0.0), 0.016)
        .unwrap_err()
        .to_string();
    assert!(message.contains("Instruction budget exceeded"));
    assert!(!message.contains("shared gameplay"));
    assert_eq!(session.script_instruction_usage(), (1_000_000, 2_000_000));
}
#[test]
fn signal_fanout_and_collision_callbacks_cannot_multiply_allowance() {
    for phase in ["signal", "collision_enter"] {
        let (mut project, mut world) = fixture(4);
        project.script_instruction_budget = 2000;
        project.scenes.get_mut("main").unwrap().script = Some("root.ks".into());
        let actor = format!("if script_phase() == \"{phase}\" {{ {HEAVY} }}");
        let root = if phase == "signal" {
            "if get_frame_count() == 1 { emit_signal(\"one\",0); emit_signal(\"two\",0); emit_signal(\"three\",0); }"
        } else {
            "return 0;"
        };
        let mut session = create(&project, &actor, root);
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        if phase == "collision_enter" {
            for n in 1..4 {
                session
                    .queue_collision_event("actor-0".into(), format!("actor-{n}"), true)
                    .unwrap();
            }
        }
        let message = session
            .step(&mut world, (0.0, 0.0), 0.016)
            .unwrap_err()
            .to_string();
        assert!(
            message.contains(phase) && message.contains("shared gameplay tick limit"),
            "{message}"
        );
        assert_eq!(session.script_instruction_usage(), (2000, 2000));
        // Failed event delivery does not disable future ticks or replenish within one tick.
        session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    }
}
#[test]
fn paused_ui_callbacks_share_budget_and_cleanup_is_fresh_but_bounded() {
    let (mut project, mut world) = fixture(4);
    project.script_instruction_budget = 2000;
    project.scenes.get_mut("main").unwrap().script = Some("root.ks".into());
    project.ui.initial_screen = "menu".into();
    project.ui.screens.insert(
        "menu".into(),
        kiln_core::ui::UiScreen {
            widgets: vec![kiln_core::ui::UiWidget {
                id: "accept".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    let actor=format!("if script_phase() == \"ui_event\" {{ {HEAVY} }} if script_phase() == \"stop\" {{ {HEAVY} }}");
    let mut session = create(
        &project,
        &actor,
        &format!("if script_phase() == \"ui_event\" {{ {HEAVY} }}"),
    );
    session.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert!(session.host_ui().pauses_game());
    session.host_mut().queue_ui_key("Enter");
    let message = session
        .step(&mut world, (0.0, 0.0), 0.016)
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("ui_event") && message.contains("shared gameplay tick limit"),
        "{message}"
    );
    assert_eq!(session.script_instruction_usage(), (2000, 2000));
    let message = session.stop(&mut world).unwrap_err().to_string();
    assert!(
        message.contains("stop") && message.contains("shared gameplay tick limit"),
        "{message}"
    );
    assert_eq!(session.script_instruction_usage(), (2000, 2000));
}
#[test]
fn project_budget_persists_and_invalid_values_reject_before_source_reads() {
    let (mut project, _) = fixture(0);
    let default = project.to_toml().unwrap();
    assert!(!default.contains("script_instruction_budget"));
    assert_eq!(
        Project::from_toml(&default)
            .unwrap()
            .script_instruction_budget,
        1_000_000
    );
    project.script_instruction_budget = 1234;
    assert_eq!(
        Project::from_toml(&project.to_toml().unwrap())
            .unwrap()
            .script_instruction_budget,
        1234
    );
    assert_eq!(
        serde_json::from_value::<Project>(serde_json::to_value(&project).unwrap())
            .unwrap()
            .script_instruction_budget,
        1234
    );
    for invalid in [0, 10_000_001, u64::MAX] {
        project.script_instruction_budget = invalid;
        let reads = std::cell::Cell::new(0);
        let result = SliceSession::with_scene_sources(&project, "main", |_| {
            reads.set(reads.get() + 1);
            Ok("return 0;".into())
        });
        assert!(result.is_err());
        assert_eq!(reads.get(), 0);
    }
}
