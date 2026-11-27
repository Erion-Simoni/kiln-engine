use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{Value, VmError};
use kiln_runtime::tweens::{TweenEase, TweenProperty as Property, TweenTrack};
use kiln_runtime::{GameWorld, SliceSession, SourceId, Sprite};
fn session(
    source: &str,
    actors: usize,
    attachment: Option<&str>,
) -> (Project, SliceSession, GameWorld) {
    let mut project = Project::default();
    let entities = (0..actors)
        .map(|i| {
            let mut components = vec![
                Component::Transform {
                    x: 80.0,
                    y: 120.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Sprite {
                    color: [1.0, 0.0, 0.0, 1.0],
                    width: 32.0,
                    height: 32.0,
                    texture: None,
                    layer: 0.0,
                    material: None,
                    animation_clip: None,
                },
            ];
            if attachment.is_some() {
                components.push(Component::Script {
                    path: "actor.ks".into(),
                    properties: Default::default(),
                });
            }
            Entity {
                id: format!("a{i}"),
                name: "Actor".into(),
                components,
            }
        })
        .collect();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            nested_scenes: vec![],
            entities,
        },
    );
    let s = SliceSession::with_scene_sources(&project, "main", |path| {
        Ok(if path == "main.ks" {
            source
        } else {
            attachment.unwrap()
        }
        .into())
    })
    .unwrap();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
        Sprite::new(color[0], color[1], color[2], color[3])
    });
    (project, s, world)
}
fn handle(world: &GameWorld) -> hecs::Entity {
    world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, id)| id.0 == "a0")
        .unwrap()
        .0
}
fn x(world: &GameWorld) -> f32 {
    world
        .world_ref()
        .get::<&Transform>(handle(world))
        .unwrap()
        .position
        .x
}
fn control(s: &mut SliceSession, world: &mut GameWorld, command: &str) {
    s.host_mut()
        .set_game_value("command", Value::Str(command.into()))
        .unwrap();
    s.step(world, (0.0, 0.0), 0.0).unwrap();
}
const CONTROL: &str = r#"
let command=game_get("command","");game_set("command","");
if command=="start" {tween_position("a0",256.0,120.0,1.0,"smooth");tween_color("a0",0.0,1.0,0.0,1.0,1.0,"linear");}
if command=="pause" {tween_pause("a0","position",true);tween_pause("a0","color",true);}
if command=="resume" {tween_pause("a0","position",false);tween_pause("a0","color",false);}
if command=="seek" {tween_seek("a0","position",0.25);}
if command=="replace" {tween_position("a0",0.0,120.0,0.5,"linear");}
if command=="manual" {set_entity_pos("a0",50.0,120.0);set_entity_component("a0","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),0.0),1.0),1.0));}
if command=="cut" {tween_position("a0",16.0,120.0,0.0,"linear");}
if command=="cancel" {tween_cancel("a0","position");}
if command=="despawn" {despawn_entity("a0");}
"#;
#[test]
fn pause_seek_completion_and_manual_writes_are_consistent() {
    let (_, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    s.step(&mut world, (0.0, 0.0), 0.5).unwrap();
    assert_eq!(x(&world), 168.0);
    assert_eq!(
        world
            .world_ref()
            .get::<&Sprite>(handle(&world))
            .unwrap()
            .color,
        [0.5, 0.5, 0.0, 1.0]
    );
    control(&mut s, &mut world, "pause");
    s.step(&mut world, (0.0, 0.0), 0.5).unwrap();
    assert_eq!(x(&world), 168.0);
    control(&mut s, &mut world, "seek");
    assert_eq!(x(&world), 107.5);
    s.step(&mut world, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(x(&world), 107.5);
    control(&mut s, &mut world, "resume");
    s.step(&mut world, (0.0, 0.0), 0.75).unwrap();
    assert_eq!(x(&world), 256.0);
    assert!(s.snapshot(&world).unwrap().tweens.tracks.is_empty());
    control(&mut s, &mut world, "start");
    control(&mut s, &mut world, "manual");
    s.step(&mut world, (0.0, 0.0), 2.0).unwrap();
    assert_eq!(x(&world), 50.0);
    assert_eq!(
        world
            .world_ref()
            .get::<&Sprite>(handle(&world))
            .unwrap()
            .color,
        [0.0, 0.0, 1.0, 1.0]
    );
    control(&mut s, &mut world, "start");
    control(&mut s, &mut world, "cut");
    assert_eq!(x(&world), 16.0);
    assert!(!s
        .snapshot(&world)
        .unwrap()
        .tweens
        .tracks
        .iter()
        .any(|t| t.property == Property::Position));
}
#[test]
fn replacement_cancel_despawn_and_stop_release_tracks() {
    let (_, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    s.step(&mut world, (0.0, 0.0), 0.5).unwrap();
    control(&mut s, &mut world, "replace");
    s.step(&mut world, (0.0, 0.0), 0.25).unwrap();
    assert_eq!(x(&world), 84.0);
    control(&mut s, &mut world, "cancel");
    s.step(&mut world, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(x(&world), 84.0);
    control(&mut s, &mut world, "start");
    control(&mut s, &mut world, "despawn");
    assert!(s.snapshot(&world).unwrap().tweens.tracks.is_empty());
    let (_, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    s.stop(&mut world).unwrap();
    assert!(s.snapshot(&world).unwrap().tweens.tracks.is_empty());
}
#[test]
fn pending_and_attachment_starts_share_the_scene_capacity() {
    let root = (0..80)
        .map(|i| {
            format!("if tween_position(\"a{i}\",200.0,120.0,1.0,\"linear\") {{accepted+=1;}}\n")
        })
        .collect::<String>();
    let source = format!("let accepted=0;{root}state_set(\"accepted\",accepted);");
    let (_, mut s, mut world) = session(&source, 80, None);
    s.step(&mut world, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(s.root_script_state("accepted"), Some(&Value::I64(64)));
    assert_eq!(s.snapshot(&world).unwrap().tweens.tracks.len(), 64);
    let root = (0..40)
        .map(|i| format!("tween_position(\"a{i}\",200.0,120.0,1.0,\"linear\");"))
        .collect::<String>();
    let child="if script_phase()==\"update\" {state_set(\"accepted\",tween_color(self_entity(),0.0,1.0,0.0,1.0,1.0,\"linear\"));}";
    let (_, mut s, mut world) = session(&root, 40, Some(child));
    s.step(&mut world, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(s.snapshot(&world).unwrap().tweens.tracks.len(), 64);
    assert_eq!(
        (0..40)
            .filter(
                |i| s.entity_script_state(&format!("a{i}"), "accepted") == Some(&Value::Bool(true))
            )
            .count(),
        24
    );
}
#[test]
fn failed_or_suspended_callbacks_cannot_start_uncommitted_tweens() {
    let (_, mut s, mut world) = session(
        "tween_position(\"a0\",256.0,120.0,1.0,\"linear\");let fail=1/0;",
        1,
        None,
    );
    assert!(s.step(&mut world, (0.0, 0.0), 0.0).is_err());
    assert!(s.snapshot(&world).unwrap().tweens.tracks.is_empty());
    let (_, mut s, mut world) = session(
        "tween_position(\"a0\",256.0,120.0,1.0,\"linear\");\nstate_set(\"done\",true);",
        1,
        None,
    );
    s.add_source_breakpoint("main.ks", 2).unwrap();
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 0.0),
        Err(VmError::DebugSuspended)
    ));
    assert_eq!(x(&world), 80.0);
    assert!(s.snapshot(&world).is_err());
    s.debug_continue();
    s.step(&mut world, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(s.snapshot(&world).unwrap().tweens.tracks.len(), 1);
    assert!(matches!(
        s.step(&mut world, (0.0, 0.0), 0.5),
        Err(VmError::DebugSuspended)
    ));
    let position = x(&world);
    assert_eq!(position, 168.0);
    for _ in 0..3 {
        assert!(matches!(
            s.step(&mut world, (0.0, 0.0), 1.0),
            Err(VmError::DebugSuspended)
        ));
        assert_eq!(x(&world), position);
    }
}
#[test]
fn saves_resume_mid_tween_and_reject_corrupt_bindings() {
    let (project, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    s.step(&mut world, (0.0, 0.0), 0.5).unwrap();
    control(&mut s, &mut world, "pause");
    let saved = s.snapshot(&world).unwrap();
    let decoded = kiln_runtime::GameSave::from_json(&saved.to_json().unwrap()).unwrap();
    let (mut resumed, mut other) = decoded
        .restore(&project, |_| Ok(CONTROL.into()), |_| None)
        .unwrap();
    assert_eq!(x(&other), 168.0);
    resumed.step(&mut other, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(x(&other), 168.0);
    control(&mut resumed, &mut other, "resume");
    resumed.step(&mut other, (0.0, 0.0), 0.5).unwrap();
    assert_eq!(x(&other), 256.0);
    let mut bad = saved;
    bad.tweens.tracks[0].entity = "missing".into();
    assert!(bad
        .restore(&project, |_| Ok(CONTROL.into()), |_| None)
        .is_err());
    assert_eq!(x(&world), 168.0);
}
#[test]
fn ui_pause_holds_time_and_physics_position_ownership_is_rejected() {
    let (_, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    s.step(&mut world, (0.0, 0.0), 0.25).unwrap();
    let position = x(&world);
    let mut doc = kiln_core::ui::UiDocument {
        initial_screen: "pause".into(),
        ..Default::default()
    };
    doc.screens.insert(
        "pause".into(),
        kiln_core::ui::UiScreen {
            pause_game: true,
            ..Default::default()
        },
    );
    *s.host_mut().ui_runtime_mut() = kiln_ui::authored::UiRuntime::new(&doc, "main").unwrap();
    s.step(&mut world, (0.0, 0.0), 1.0).unwrap();
    assert_eq!(x(&world), position);
    let (_,mut s,mut world)=session("state_set(\"position\",tween_position(\"a0\",200.0,120.0,1.0,\"linear\"));state_set(\"color\",tween_color(\"a0\",0.0,1.0,0.0,1.0,1.0,\"linear\"));",1,None);
    let actor = handle(&world);
    world
        .world_mut()
        .insert_one(actor, kiln_core::physics_data::RigidBody2D::default())
        .unwrap();
    s.step(&mut world, (0.0, 0.0), 0.0).unwrap();
    assert_eq!(s.root_script_state("position"), Some(&Value::Bool(false)));
    assert_eq!(s.root_script_state("color"), Some(&Value::Bool(true)));
}
#[test]
fn finite_extreme_interpolation_and_ease_curves_are_safe() {
    let track = TweenTrack {
        entity: "a0".into(),
        property: Property::Position,
        from: vec![-f32::MAX as f64, 0.0],
        to: vec![f32::MAX as f64, 0.0],
        duration: 1.0,
        elapsed: 0.5,
        ease: TweenEase::Linear,
        paused: false,
    };
    track.validate().unwrap();
    for bad in [f64::NAN, -1.0, 61.0] {
        let mut invalid = track.clone();
        invalid.duration = bad;
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn invalid_requests_leave_no_queued_state_and_old_snapshots_remain_readable() {
    for source in [
        "tween_position(\"a0\",1.0,2.0,-1.0,\"linear\");",
        "tween_position(\"a0\",1.0,2.0,1.0,\"unknown\");",
        "tween_color(\"a0\",2.0,0.0,0.0,1.0,1.0,\"linear\");",
        "tween_seek(\"a0\",\"position\",-1.0);",
    ] {
        let (_, mut s, mut world) = session(source, 1, None);
        assert!(s.step(&mut world, (0.0, 0.0), 0.0).is_err());
        assert!(s.snapshot(&world).unwrap().tweens.tracks.is_empty());
    }
    let (project, mut s, mut world) = session(CONTROL, 1, None);
    control(&mut s, &mut world, "start");
    let saved = s.snapshot(&world).unwrap();
    let mut json = serde_json::to_value(&saved).unwrap();
    json.as_object_mut().unwrap().remove("tweens");
    let old = kiln_runtime::GameSave::from_json(&serde_json::to_string(&json).unwrap()).unwrap();
    assert!(old.tweens.tracks.is_empty());
    assert!(old
        .restore(&project, |_| Ok(CONTROL.into()), |_| None)
        .is_ok());
}
