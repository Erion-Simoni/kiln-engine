use kiln_core::{
    ui::*, Action, Binding, BindingType, Component, Entity, Project, Scene, Transform,
};
use kiln_ir::{HostContext, Value};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
use std::collections::HashMap;
fn setup(root: &str, child: &str) -> (SliceSession, GameWorld) {
    let mut p = Project::default();
    p.action_map.add_action(
        Action::new("move", "Move").with_binding(Binding::new(BindingType::Key("d".into()))),
    );
    p.ui.initial_screen = "menu".into();
    p.ui.screens.insert(
        "menu".into(),
        UiScreen {
            widgets: vec![UiWidget {
                id: "button".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    p.ui.screens.insert("other".into(), UiScreen::default());
    p.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            nested_scenes: vec![],
            script: Some("main.ks".into()),
            entities: vec![Entity {
                id: "controller".into(),
                name: "Controller".into(),
                components: vec![Component::Script {
                    path: "ui.ks".into(),
                    properties: HashMap::new(),
                }],
            }],
        },
    );
    let session = SliceSession::with_scene_sources(&p, "main", |path| {
        Ok(if path == "main.ks" {
            root.into()
        } else {
            child.into()
        })
    })
    .unwrap();
    let mut world = GameWorld::new();
    world.spawn((SourceId::new("controller"), Transform::default()));
    (session, world)
}
#[test]
fn ui_callback_errors_restore_the_root_and_entity_context() {
    for root_error in [true, false] {
        let source = r#"if script_phase() == "ui_event" { ui_show_screen("other"); array_get(array_new(), 10); }"#;
        let (mut session, mut world) = setup(
            if root_error { source } else { "return 0;" },
            if root_error { "return 0;" } else { source },
        );
        session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
        session.host_mut().queue_ui_key("Enter");
        let error = session
            .step(&mut world, (0.0, 0.0), 1.0 / 60.0)
            .unwrap_err()
            .to_string();
        assert!(error.contains(".ks:"), "{error}");
        assert_eq!(session.host_ui().active_screen(), "other");
        assert_eq!(
            session.host_mut().call_host(83, &[]).unwrap(),
            Value::Str(String::new())
        );
        session
            .host_mut()
            .call_host(80, &[Value::Str("menu".into())])
            .unwrap();
        session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap(); // The failed edge is not replayed.
        assert_eq!(
            session.host_mut().call_host(35, &[]).unwrap(),
            Value::I64(0)
        );
    }
}
#[test]
fn modal_screens_capture_gameplay_actions_in_root_and_attached_updates() {
    let source = r#"if script_phase() == "update" {state_set("input", get_action("move"));state_set("axis",get_input_x());}"#;
    let (mut session, mut world) = setup(source, source);
    // Keep simulation running while input is captured.
    session.host_mut().ui_runtime_mut().close();
    session.host_mut().set_pressed_keys(&["d".into()]);
    session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("controller", "input"),
        Some(&Value::Bool(true))
    );
    // Configure a new non-pausing modal document through the normal scene loader.
    let mut doc = session.host_ui().document().clone();
    doc.screens.get_mut("menu").unwrap().pause_game = false;
    *session.host_mut().ui_runtime_mut() = kiln_ui::authored::UiRuntime::new(&doc, "main").unwrap();
    session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("controller", "input"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        session.entity_script_state("controller", "axis"),
        Some(&Value::F64(0.0))
    );
    assert_eq!(
        session
            .host_mut()
            .call_host(57, &[Value::Str("input".into()), Value::I64(0)])
            .unwrap(),
        Value::Bool(false)
    );
    session.host_mut().ui_runtime_mut().close();
    session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("controller", "input"),
        Some(&Value::Bool(true))
    );
}
#[test]
fn queued_pointer_taps_use_viewport_mapping_and_focus_loss_cancels_them() {
    let source =
        r#"if script_phase() == "ui_event" {state_set("events",state_get("events",0)+1);}"#;
    let (mut session, mut world) = setup("return 0;", source);
    let area = [100.0, 50.0, 1280.0, 720.0];
    session.host_mut().set_ui_pointer(area, None, false);
    for down in [true, false] {
        session
            .host_mut()
            .queue_ui_pointer(area, [160.0, 110.0], down);
    }
    session.host_mut().reset_ui_input();
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(session.entity_script_state("controller", "events"), None);
    for down in [true, false] {
        session
            .host_mut()
            .queue_ui_pointer(area, [99.0, 110.0], down);
    }
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(session.entity_script_state("controller", "events"), None);
    for down in [true, false] {
        session
            .host_mut()
            .queue_ui_pointer(area, [160.0, 110.0], down);
    }
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("controller", "events"),
        Some(&Value::I64(1))
    );
}
#[test]
fn authored_labels_accept_numeric_gameplay_values_without_a_formatting_workaround() {
    let (mut session, _) = setup("return 0;", "return 0;");
    assert_eq!(
        session
            .host_mut()
            .call_host(85, &[Value::Str("button".into()), Value::I64(42)])
            .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        session.host_ui().document().screens["menu"].widgets[0].text,
        "42"
    );
    assert!(session
        .host_mut()
        .call_host(86, &[Value::Str("button".into()), Value::I64(0)])
        .is_err());
}
#[test]
fn attached_ui_callbacks_receive_current_physical_input_while_gameplay_is_paused() {
    let source = r#"if script_phase() == "ui_event" {state_set("action",get_action("move"));state_set("pointer",get_mouse_world_x());}"#;
    let (mut session, mut world) = setup("return 0;", source);
    session.host_mut().set_pressed_keys(&["d".into()]);
    session.host_mut().set_mouse_world([123.0, 456.0]);
    session.host_mut().queue_ui_key("Enter");
    session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.entity_script_state("controller", "action"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        session.entity_script_state("controller", "pointer"),
        Some(&Value::F64(123.0))
    );
    assert_eq!(session.state().elapsed, 0.0);
}

#[test]
fn rapid_cancel_navigation_and_accept_keep_their_order_across_menu_transitions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/ui2d_editor");
    let project =
        Project::from_toml(&std::fs::read_to_string(root.join("kiln.toml")).unwrap()).unwrap();
    let mut session = SliceSession::with_scene_sources(&project, "main", |path| {
        std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string())
    })
    .unwrap();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
        kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3])
    });
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    session.host_mut().queue_ui_key("ArrowDown");
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    session.host_mut().queue_ui_key("Enter");
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(session.host_ui().active_screen(), "settings");
    // All events arrive between two fixed ticks, as quick DOM key taps do.
    for key in ["Escape", "ArrowUp", "Enter"] {
        session.host_mut().queue_ui_key(key);
    }
    session.host_mut().set_pressed_keys(&["Enter".into()]);
    for _ in 0..3 {
        session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    }
    assert_eq!(session.host_ui().active_screen(), "hud");
    session.step(&mut world, (0.0, 0.0), 1.0 / 60.0).unwrap();
    assert_eq!(
        session.host_mut().game_value("health"),
        Some(&Value::I64(73))
    );
}
