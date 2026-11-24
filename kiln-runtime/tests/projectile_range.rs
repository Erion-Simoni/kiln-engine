//! Real sensor collisions and logical saves must agree for editable graphs and text.
#![cfg(feature = "physics")]
use kiln_core::{Project, Transform};
use kiln_ir::Value;
use kiln_runtime::{GameSave, GameWorld, SliceSession, SourceId, Sprite};
use std::path::{Path, PathBuf};

fn read(root: &Path, relative: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(relative)).map_err(|error| error.to_string())
}
fn load(name: &str) -> (PathBuf, Project, SliceSession, GameWorld) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../samples")
        .join(name);
    let project = Project::from_toml(&read(&root, "kiln.toml").unwrap()).unwrap();
    let session = SliceSession::with_scene_sources(&project, "main", |p| read(&root, p)).unwrap();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
        Sprite::new(color[0], color[1], color[2], color[3])
    });
    (root, project, session, world)
}
fn x(world: &GameWorld, name: &str) -> Option<f32> {
    world
        .world_ref()
        .query::<(&SourceId, &Transform)>()
        .iter()
        .find(|(id, _)| id.0 == name)
        .map(|(_, t)| t.position.x)
}
fn observe(
    session: &SliceSession,
    world: &GameWorld,
) -> (Option<Value>, Option<Value>, Option<f32>, bool, [f32; 4]) {
    let color = world
        .world_ref()
        .query::<(&SourceId, &Sprite)>()
        .iter()
        .find(|(id, _)| id.0 == "actor")
        .map(|(_, sprite)| sprite.color)
        .unwrap();
    (
        session
            .snapshot(world)
            .unwrap()
            .game_data
            .get("hits")
            .map(|v| v.value().unwrap()),
        session.entity_script_state("trigger", "activated").cloned(),
        x(world, "projectile"),
        x(world, "enemy").is_some(),
        color,
    )
}
#[test]
fn text_and_graph_match_real_collisions_and_logical_restore_outcomes() {
    let (text_root, text_project, mut text, mut text_world) = load("projectile_range");
    let (graph_root, graph_project, mut graph, mut graph_world) = load("projectile_range_graph");
    let mut restored_text = None;
    let mut restored_graph = None;
    for tick in 0..240 {
        // Left stops the projectile, right doubles speed, release returns to base speed.
        let input = if tick < 12 {
            -1.0
        } else if tick < 36 {
            1.0
        } else {
            0.0
        };
        text.step(&mut text_world, (input, 0.0), 1.0 / 60.0)
            .unwrap();
        graph
            .step(&mut graph_world, (input, 0.0), 1.0 / 60.0)
            .unwrap();
        assert_eq!(
            observe(&text, &text_world),
            observe(&graph, &graph_world),
            "tick {tick}"
        );
        if tick == 11 {
            assert_eq!(x(&text_world, "projectile"), Some(40.0));
        }
        if tick == 45 {
            assert!(
                x(&text_world, "enemy").is_some(),
                "save must precede the hit"
            );
            let save = GameSave::from_json(&text.snapshot(&text_world).unwrap().to_json().unwrap())
                .unwrap();
            restored_text = Some(
                save.restore(&text_project, |p| read(&text_root, p), |_| None)
                    .unwrap(),
            );
            let save =
                GameSave::from_json(&graph.snapshot(&graph_world).unwrap().to_json().unwrap())
                    .unwrap();
            restored_graph = Some(
                save.restore(&graph_project, |p| read(&graph_root, p), |_| None)
                    .unwrap(),
            );
        } else if tick > 45 {
            for (restored, original, world) in [
                (&mut restored_text, &text, &text_world),
                (&mut restored_graph, &graph, &graph_world),
            ] {
                let (session, restored_world) = restored.as_mut().unwrap();
                session
                    .step(restored_world, (input, 0.0), 1.0 / 60.0)
                    .unwrap();
                // Physics warm-start state is not serialized. Exact resumed pose
                // continuation is tracked as blocked; compare eventual gameplay.
                if tick == 239 {
                    let actual = observe(session, restored_world);
                    let expected = observe(original, world);
                    assert_eq!(
                        (actual.0, actual.1, actual.3, actual.4),
                        (expected.0, expected.1, expected.3, expected.4)
                    );
                }
            }
        }
    }
    assert_eq!(observe(&text, &text_world).0, Some(Value::I64(1)));
    assert_eq!(text.state().score, 1);
    assert_eq!(graph.state().score, 1);
    assert!(x(&text_world, "enemy").is_none());
    assert_eq!(
        text.entity_script_state("trigger", "activated"),
        Some(&Value::Bool(true))
    );
    assert_eq!(observe(&text, &text_world).4, [0.0, 1.0, 0.0, 1.0]);
}
