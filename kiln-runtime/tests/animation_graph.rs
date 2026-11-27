use kiln_core::{
    animation_graph::{AnimationGraph, AnimationTransition, Comparison},
    Clip, Component, Entity, Frame, FrameEvent, Project, Scene,
};
use kiln_ir::{Value, VmError};
use kiln_render::TextureId;
use kiln_runtime::{AnimationPlayer, GameWorld, SliceSession};
use std::collections::{BTreeMap, HashMap};
fn clips() -> HashMap<String, Clip> {
    ["idle", "run", "jump"]
        .into_iter()
        .map(|name| {
            let mut clip = Clip::new(name.into(), "white.png".into());
            clip.loops = name != "jump";
            clip.add_frame(Frame {
                rect: [0, 0, 1, 1],
                pivot: [0.5; 2],
                duration: 0.125,
                events: vec![FrameEvent {
                    name: name.into(),
                    payload: None,
                }],
            });
            (name.into(), clip)
        })
        .collect()
}
fn graph() -> AnimationGraph {
    AnimationGraph {
        entry: "idle".into(),
        states: BTreeMap::from([
            ("idle".into(), "idle".into()),
            ("run".into(), "run".into()),
            ("jump".into(), "jump".into()),
        ]),
        parameters: BTreeMap::from([("moving".into(), 0.0), ("jump".into(), 0.0)]),
        transitions: vec![
            AnimationTransition {
                from: "*".into(),
                to: "jump".into(),
                parameter: "jump".into(),
                comparison: Comparison::Greater,
                threshold: 0.5,
                ..Default::default()
            },
            AnimationTransition {
                from: "idle".into(),
                to: "run".into(),
                parameter: "moving".into(),
                comparison: Comparison::Greater,
                threshold: 0.5,
                ..Default::default()
            },
            AnimationTransition {
                from: "run".into(),
                to: "idle".into(),
                parameter: "moving".into(),
                comparison: Comparison::Less,
                threshold: 0.5,
                ..Default::default()
            },
            AnimationTransition {
                from: "jump".into(),
                to: "idle".into(),
                after_finished: true,
                ..Default::default()
            },
        ],
    }
}
fn player() -> AnimationPlayer {
    let mut player = AnimationPlayer::new();
    for clip in clips().into_values() {
        player.add_bound_clip(clip, TextureId::WHITE, [1, 1]);
    }
    player.bind_graph(graph()).unwrap();
    player
}
#[test]
fn movement_conditions_do_not_restart_running_and_pause_holds_graph() {
    let mut p = player();
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.update(0.03125);
    assert_eq!(p.graph_state(), Some("run"));
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.update(0.03125);
    assert_eq!(p.elapsed(), 0.0625);
    p.set_speed(0.0);
    p.set_graph_parameter("moving", 0.0).unwrap();
    p.update(1.0);
    assert_eq!(p.graph_state(), Some("run"));
    assert_eq!(p.elapsed(), 0.0625);
    p.set_speed(1.0);
    p.update(0.03125);
    assert_eq!(p.graph_state(), Some("idle"));
    p.play("unknown");
    assert_eq!(p.graph_state(), Some("idle"));
    p.play("idle");
    assert_eq!(p.graph_state(), None);
}
#[test]
fn priority_one_shot_finish_and_same_clip_state_restart_are_explicit() {
    let mut p = player();
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.set_graph_parameter("jump", 1.0).unwrap();
    p.update(0.125);
    assert_eq!(p.graph_state(), Some("jump"));
    assert_eq!(p.take_events()[0].name, "jump");
    p.set_graph_parameter("jump", 0.0).unwrap();
    p.set_graph_parameter("moving", 0.0).unwrap();
    p.update(0.03125);
    assert_eq!(p.graph_state(), Some("idle"));
    assert_eq!(p.take_events()[0].name, "idle");
    let mut same = graph();
    same.states.insert("run".into(), "idle".into());
    p.bind_graph(same).unwrap();
    p.update(0.0625);
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.update(0.03125);
    assert_eq!(p.elapsed(), 0.03125);
}
#[test]
fn graph_snapshot_resume_and_corrupt_state_validation_are_atomic() {
    let mut p = player();
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.update(0.03125);
    p.take_events();
    assert!(p.set_graph_parameter("unknown", 1.0).is_err());
    assert!(p.set_graph_parameter("moving", f64::NAN).is_err());
    let saved = p.snapshot().unwrap();
    let mut resumed =
        AnimationPlayer::restore_snapshot(&saved, |_| Some((TextureId::WHITE, [1, 1]))).unwrap();
    p.update(0.03125);
    resumed.update(0.03125);
    assert_eq!(p.elapsed(), resumed.elapsed());
    assert_eq!(resumed.graph_state(), Some("run"));
    assert_eq!(resumed.graph_parameters().unwrap()["moving"], 1.0);
    let mut bad = saved.clone();
    bad.graph.as_mut().unwrap().state = "missing".into();
    bad.current = None;
    assert!(AnimationPlayer::restore_snapshot(&bad, |_| Some((TextureId::WHITE, [1, 1]))).is_err());
    let mut bad = saved;
    bad.graph
        .as_mut()
        .unwrap()
        .parameters
        .insert("extra".into(), 0.0);
    assert!(AnimationPlayer::restore_snapshot(&bad, |_| Some((TextureId::WHITE, [1, 1]))).is_err());
}
fn session(source: &str) -> (Project, SliceSession, GameWorld) {
    let mut project = Project {
        animation_clips: clips(),
        ..Default::default()
    };
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "actor".into(),
                name: "Actor".into(),
                components: vec![
                    Component::Transform {
                        x: 0.0,
                        y: 0.0,
                        rotation: 0.0,
                        scale: 1.0,
                    },
                    Component::Sprite {
                        color: [1.0; 4],
                        width: 16.0,
                        height: 16.0,
                        texture: None,
                        layer: 0.0,
                        material: None,
                        animation_clip: None,
                    },
                    Component::AnimationGraph { graph: graph() },
                ],
            }],
        },
    );
    let session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(source.into())).unwrap();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
        kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]).with_size(1.0, 1.0)
    });
    (project, session, world)
}
fn graph_param(world: &GameWorld) -> f64 {
    world
        .world_ref()
        .query::<&AnimationPlayer>()
        .iter()
        .next()
        .unwrap()
        .graph_parameters()
        .unwrap()["moving"]
}
#[test]
fn script_parameters_commit_only_with_callback_success_and_debug_resume() {
    let (_, mut failed, mut world) =
        session("set_animation_parameter(\"actor\",\"moving\",true);let fail=1/0;");
    assert!(failed.step(&mut world, (0.0, 0.0), 0.016).is_err());
    assert_eq!(graph_param(&world), 0.0);
    let (_, mut paused, mut world) =
        session("set_animation_parameter(\"actor\",\"moving\",true);\nstate_set(\"done\",true);");
    paused.add_source_breakpoint("main.ks", 2).unwrap();
    assert!(matches!(
        paused.step(&mut world, (0.0, 0.0), 0.016),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(graph_param(&world), 0.0);
    paused.debug_continue();
    paused.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(graph_param(&world), 1.0);
}
#[test]
fn script_facts_and_general_logical_save_preserve_active_state() {
    let source="state_set(\"queued\",set_animation_parameter(\"actor\",\"moving\",true));state_set(\"state\",get_animation_state(\"actor\"));state_set(\"missing\",set_animation_parameter(\"actor\",\"bad\",true));";
    let (project, mut s, mut world) = session(source);
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(s.root_script_state("queued"), Some(&Value::Bool(true)));
    assert_eq!(s.root_script_state("missing"), Some(&Value::Bool(false)));
    assert_eq!(
        s.root_script_state("state"),
        Some(&Value::Str("run".into()))
    );
    let saved = s.snapshot(&world).unwrap();
    let (mut restored, mut world) = saved
        .restore(
            &project,
            |_| Ok(source.into()),
            |_| Some((TextureId::WHITE, [1, 1])),
        )
        .unwrap();
    assert_eq!(graph_param(&world), 1.0);
    restored.step(&mut world, (0.0, 0.0), 0.016).unwrap();
    assert_eq!(
        restored.root_script_state("state"),
        Some(&Value::Str("run".into()))
    );
}
#[test]
fn graph_reference_limits_and_conflicting_clip_ownership_reject() {
    let mut model = graph();
    let mut resources = clips();
    model.validate(&resources).unwrap();
    model.transitions[0].to = "missing".into();
    assert!(model.validate(&resources).is_err());
    model = graph();
    model.transitions[1].after_finished = true;
    assert!(model.validate(&resources).is_err());
    model = graph();
    model.transitions[0].threshold = f64::NAN;
    assert!(model.validate(&resources).is_err());
    model = graph();
    resources.get_mut("idle").unwrap().next = Some("run".into());
    assert!(model.validate(&resources).is_err());
}

#[test]
fn graph_transitions_preserve_unconsumed_crossed_events() {
    let mut p = player();
    p.update(0.03125);
    p.set_graph_parameter("moving", 1.0).unwrap();
    p.update(0.03125);
    assert_eq!(
        p.take_events()
            .iter()
            .map(|event| event.name.as_str())
            .collect::<Vec<_>>(),
        ["idle", "run"]
    );
}
