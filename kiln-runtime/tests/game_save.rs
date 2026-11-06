use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{HostContext, Value};
use kiln_runtime::{GameSave, GameWorld, ParentLink, SliceSession, SourceId, Sprite, Velocity};
use std::{collections::HashMap, path::PathBuf};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kiln-game-save-{}-{}",
            std::process::id(),
            kiln_core::next_id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (Project, HashMap<String, String>) {
    let mut p = Project {
        id: "save-game".into(),
        ..Default::default()
    };
    let actor = Entity {
        id: "actor".into(),
        name: "Actor".into(),
        components: vec![
            Component::Transform {
                x: 20.0,
                y: 30.0,
                rotation: 0.25,
                scale: 2.0,
            },
            Component::Sprite {
                color: [0.2, 0.4, 0.6, 1.0],
                width: 42.0,
                height: 51.0,
                texture: None,
                layer: 7.0,
                material: None,
                animation_clip: None,
            },
            Component::Script {
                path: "actor.ks".into(),
                properties: HashMap::from([("health".into(), 123.0)]),
            },
        ],
    };
    let child = Entity {
        id: "child".into(),
        name: "Child".into(),
        components: vec![
            Component::Transform {
                x: 3.0,
                y: 4.0,
                rotation: 0.0,
                scale: 1.0,
            },
            Component::Parent {
                parent: "actor".into(),
            },
        ],
    };
    let victim = Entity {
        id: "victim".into(),
        name: "Victim".into(),
        components: vec![],
    };
    for id in ["a", "b"] {
        p.scenes.insert(
            id.into(),
            Scene {
                id: id.into(),
                name: id.into(),
                script: Some("root.ks".into()),
                entities: vec![actor.clone(), child.clone(), victim.clone()],
                nested_scenes: vec![],
            },
        );
    }
    let scripts=HashMap::from([
        ("root.ks".into(),"state_set(\"ticks\", state_get(\"ticks\", 0)+1); if game_get(\"spawn\", false) { spawn_entity(\"first\",1.0,2.0,\"\"); spawn_entity(\"second\",3.0,4.0,\"\"); despawn_entity(\"victim\"); game_set(\"spawn\",false); } if game_get(\"save\",false) { game_set(\"save\",false); save_game(2); } if game_get(\"load\",false) {game_set(\"load\",false);load_game(2);}".into()),
        ("actor.ks".into(),"if script_phase()==\"start\" {state_set(\"starts\",state_get(\"starts\",0)+1);state_set(\"health\",get_property(\"health\"));} if script_phase()==\"update\" {state_set(\"ticks\",state_get(\"ticks\",0)+1);}".into()),
    ]);
    (p, scripts)
}
fn launch(
    p: &Project,
    scripts: &HashMap<String, String>,
    scene: &str,
    temp: &Temp,
) -> (SliceSession, GameWorld) {
    let mut session = SliceSession::with_scene_sources(p, scene, |path| {
        scripts.get(path).cloned().ok_or(path.into())
    })
    .unwrap();
    session.host_mut().set_save_dir(&temp.0);
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, p, scene, &mut |_, color, _| {
        Sprite::new(color[0], color[1], color[2], color[3])
    });
    (session, world)
}
fn entity(world: &GameWorld, id: &str) -> hecs::Entity {
    world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, s)| s.0 == id)
        .unwrap()
        .0
}
fn ids(world: &GameWorld) -> Vec<String> {
    let mut ids: Vec<_> = world
        .world_ref()
        .query::<&SourceId>()
        .iter()
        .map(|id| id.0.clone())
        .collect();
    ids.sort();
    ids
}
fn load(
    session: &mut SliceSession,
    world: &mut GameWorld,
    temp: &Temp,
    p: &Project,
    scripts: &HashMap<String, String>,
    slot: u32,
) -> Result<(), String> {
    let _ = temp;
    session.load_game(
        world,
        slot,
        p,
        |path| scripts.get(path).cloned().ok_or(path.into()),
        |_| None,
    )
}

#[test]
fn full_general_world_script_data_and_cross_scene_restore_survive_restart() {
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    session
        .host_mut()
        .set_game_value("spawn", Value::Bool(true))
        .unwrap();
    session
        .host_mut()
        .set_game_value(
            "inventory",
            Value::Array(vec![
                Value::I64(i64::MAX),
                Value::F64(2.0),
                Value::Str("key".into()),
                Value::Bool(true),
            ]),
        )
        .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.05).unwrap();
    session.step(&mut world, (0.0, 0.0), 0.05).unwrap();
    let actor = entity(&world, "actor");
    world
        .world_mut()
        .get::<&mut Transform>(actor)
        .unwrap()
        .scale = kiln_core::Vec2::new(2.0, 3.0);
    world
        .world_mut()
        .insert_one(actor, Velocity([11.0, -4.0]))
        .unwrap();
    session.save_game(&world, 2).unwrap();
    let before = ids(&world);
    assert!(before.contains(&"entity_1".into()) && before.contains(&"entity_2".into()));
    assert!(!before.contains(&"victim".into()));
    let (mut fresh, mut fresh_world) = launch(&p, &scripts, "b", &temp);
    load(&mut fresh, &mut fresh_world, &temp, &p, &scripts, 2).unwrap();
    assert_eq!(fresh.scene_id(), Some("a"));
    assert_eq!(ids(&fresh_world), before);
    let actor = entity(&fresh_world, "actor");
    let child = entity(&fresh_world, "child");
    assert_eq!(
        fresh_world
            .world_ref()
            .get::<&Transform>(actor)
            .unwrap()
            .scale
            .to_array(),
        [2.0, 3.0]
    );
    assert_eq!(
        fresh_world.world_ref().get::<&Velocity>(actor).unwrap().0,
        [11.0, -4.0]
    );
    assert_eq!(
        fresh_world.world_ref().get::<&ParentLink>(child).unwrap().0,
        actor
    );
    assert_eq!(
        fresh_world.world_ref().get::<&Sprite>(actor).unwrap().size,
        [42.0, 51.0]
    );
    assert_eq!(
        fresh.host_mut().game_value("inventory"),
        session.host_mut().game_value("inventory")
    );
    assert_eq!(
        fresh.entity_script_state("actor", "starts"),
        Some(&Value::I64(1))
    );
    assert_eq!(
        fresh.entity_script_state("actor", "ticks"),
        Some(&Value::I64(2))
    );
    fresh.step(&mut fresh_world, (0.0, 0.0), 0.05).unwrap();
    assert_eq!(
        fresh.entity_script_state("actor", "starts"),
        Some(&Value::I64(1))
    );
    assert_eq!(
        fresh.entity_script_state("actor", "ticks"),
        Some(&Value::I64(3))
    );
    assert_eq!(
        fresh
            .host_mut()
            .call_host(57, &[Value::Str("ticks".into()), Value::I64(0)])
            .unwrap(),
        Value::I64(3)
    );
}

#[test]
fn script_save_and_load_are_serviced_after_commands_and_resume_at_tick_boundaries() {
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    session
        .host_mut()
        .set_game_value("save", Value::Bool(true))
        .unwrap();
    session
        .host_mut()
        .set_game_value("spawn", Value::Bool(true))
        .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.05).unwrap();
    assert_eq!(session.host_mut().save_status(), "queued");
    assert!(!session
        .service_saves(
            &mut world,
            &p,
            |path| scripts.get(path).cloned().ok_or(path.into()),
            |_| None
        )
        .unwrap());
    assert_eq!(session.host_mut().save_status(), "saved");
    let spawned = entity(&world, "entity_1");
    world.world_mut().despawn(spawned).unwrap();
    session
        .host_mut()
        .set_game_value("load", Value::Bool(true))
        .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.05).unwrap();
    assert!(session
        .service_saves(
            &mut world,
            &p,
            |path| scripts.get(path).cloned().ok_or(path.into()),
            |_| None
        )
        .unwrap());
    assert_eq!(session.host_mut().save_status(), "loaded");
    assert!(ids(&world).contains(&"entity_1".into()));
    assert_eq!(
        session.entity_script_state("actor", "ticks"),
        Some(&Value::I64(1))
    );
}

#[test]
fn invalid_versions_corruption_scripts_and_missing_resources_do_not_replace_live_state() {
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    session.step(&mut world, (0.0, 0.0), 0.01).unwrap();
    session.save_game(&world, 0).unwrap();
    let before = ids(&world);
    let original = std::fs::read(temp.0.join("slot_0.json")).unwrap();
    let mut changed = scripts.clone();
    changed
        .get_mut("actor.ks")
        .unwrap()
        .push_str("state_set(\"new\",1);");
    assert!(load(&mut session, &mut world, &temp, &p, &changed, 0).is_err());
    assert_eq!(ids(&world), before);
    let mut saved = session.snapshot(&world).unwrap();
    saved.version += 1;
    let json = serde_json::to_string(&saved).unwrap();
    kiln_save::SaveSlotManager::with_directory(temp.0.clone())
        .save(0, &json, "future")
        .unwrap();
    let future = std::fs::read(temp.0.join("slot_0.json")).unwrap();
    assert!(load(&mut session, &mut world, &temp, &p, &scripts, 0).is_err());
    assert!(session.save_game(&world, 0).is_err());
    assert_eq!(std::fs::read(temp.0.join("slot_0.json")).unwrap(), future);
    assert_eq!(ids(&world), before);
    std::fs::write(temp.0.join("slot_0.json"), b"broken").unwrap();
    assert!(session.save_game(&world, 0).is_err());
    assert_eq!(
        std::fs::read(temp.0.join("slot_0.json")).unwrap(),
        b"broken"
    );
    std::fs::write(temp.0.join("slot_0.json"), original).unwrap();
    let mut wrong = p.clone();
    wrong.id = "other-game".into();
    assert!(load(&mut session, &mut world, &temp, &wrong, &scripts, 0).is_err());
    assert_eq!(ids(&world), before);
    assert_eq!(
        session.host_mut().call_host(53, &[Value::I64(-1)]).unwrap(),
        Value::Bool(false)
    );
    assert!(session
        .host_mut()
        .set_game_value("nan", Value::F64(f64::NAN))
        .is_err());
    assert!(GameSave::from_json("{}").is_err());
}

#[test]
fn player_preferences_are_independent_of_a_loaded_game_slot() {
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    session
        .host_mut()
        .set_player_setting("language", Value::Str("en".into()))
        .unwrap();
    session.save_game(&world, 0).unwrap();
    session
        .host_mut()
        .set_player_setting("language", Value::Str("sq".into()))
        .unwrap();
    load(&mut session, &mut world, &temp, &p, &scripts, 0).unwrap();
    assert_eq!(
        session.host_mut().player_setting("language"),
        Some(Value::Str("sq".into()))
    );
}

#[test]
fn animation_rebinds_resources_and_missing_textures_leave_live_world_untouched() {
    use kiln_core::{Clip, Frame};
    use kiln_render::TextureId;
    use kiln_runtime::AnimationPlayer;
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    let actor = entity(&world, "actor");
    let mut clip = Clip::new("walk".into(), "sheet.png".into());
    clip.frames.push(Frame {
        rect: [0, 0, 16, 16],
        pivot: [0.5; 2],
        duration: 1.0,
        events: vec![],
    });
    let mut animation = AnimationPlayer::new();
    animation.add_bound_clip(clip, TextureId::WHITE, [32, 16]);
    animation.play("walk");
    animation.seek(0.4);
    world.world_mut().insert_one(actor, animation).unwrap();
    session.save_game(&world, 0).unwrap();
    world
        .world_mut()
        .get::<&mut Transform>(actor)
        .unwrap()
        .position
        .x = 999.0;
    assert!(load(&mut session, &mut world, &temp, &p, &scripts, 0).is_err());
    assert_eq!(
        world
            .world_ref()
            .get::<&Transform>(actor)
            .unwrap()
            .position
            .x,
        999.0
    );
    session
        .load_game(
            &mut world,
            0,
            &p,
            |path| scripts.get(path).cloned().ok_or(path.into()),
            |path| (path == "sheet.png").then_some((TextureId::WHITE, [32, 16])),
        )
        .unwrap();
    let actor = entity(&world, "actor");
    let animation = world.world_ref().get::<&AnimationPlayer>(actor).unwrap();
    assert_eq!(animation.current_clip(), Some("walk"));
    assert!((animation.elapsed() - 0.4).abs() < 0.0001);
    drop(animation);
    assert_eq!(
        world
            .world_ref()
            .get::<&Transform>(actor)
            .unwrap()
            .position
            .x,
        20.0
    );
}

#[test]
fn deleted_spawn_ids_are_not_reused_after_save_and_restore() {
    let temp = Temp::new();
    let (p, scripts) = fixture();
    let (mut session, mut world) = launch(&p, &scripts, "a", &temp);
    session
        .host_mut()
        .set_game_value("spawn", Value::Bool(true))
        .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.01).unwrap();
    let deleted = entity(&world, "entity_2");
    world.world_mut().despawn(deleted).unwrap();
    session.save_game(&world, 0).unwrap();
    load(&mut session, &mut world, &temp, &p, &scripts, 0).unwrap();
    session
        .host_mut()
        .set_game_value("spawn", Value::Bool(true))
        .unwrap();
    session.step(&mut world, (0.0, 0.0), 0.01).unwrap();
    assert!(!ids(&world).contains(&"entity_2".into()));
    assert!(ids(&world).contains(&"entity_3".into()));
    assert!(ids(&world).contains(&"entity_4".into()));
}

#[test]
fn normal_scene_changes_carry_typed_progress_written_by_stop_callbacks() {
    let temp = Temp::new();
    let (p, mut scripts) = fixture();
    scripts
        .get_mut("actor.ks")
        .unwrap()
        .push_str(" if script_phase()==\"stop\" { game_set(\"quest\", array_push(array_push(array_new(), true), 9)); }");
    let (mut previous, mut world) = launch(&p, &scripts, "a", &temp);
    previous.step(&mut world, (0.0, 0.0), 0.01).unwrap();
    previous.stop(&mut world).unwrap();
    let (mut next, mut next_world) = launch(&p, &scripts, "b", &temp);
    next.set_game_progress(previous.game_progress()).unwrap();
    next.step(&mut next_world, (0.0, 0.0), 0.01).unwrap();
    assert_eq!(
        next.host_mut().game_value("quest"),
        Some(&Value::Array(vec![Value::Bool(true), Value::I64(9)]))
    );
    next.save_game(&next_world, 0).unwrap();
    let (mut restart, mut restart_world) = launch(&p, &scripts, "a", &temp);
    load(&mut restart, &mut restart_world, &temp, &p, &scripts, 0).unwrap();
    assert_eq!(restart.scene_id(), Some("b"));
    assert_eq!(
        restart.host_mut().game_value("quest"),
        next.host_mut().game_value("quest")
    );
}
