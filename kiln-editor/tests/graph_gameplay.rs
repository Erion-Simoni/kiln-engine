//! Graph programs execute through real session loading, gameplay and save restoration.
use kiln_core::{Entity, Project, Transform};
use kiln_graph::{ArithmeticOp, Connection, ConstantValue, Graph, Node, NodeType};
use kiln_runtime::{GameSave, GameWorld, SliceSession, SourceId, Sprite};
use uuid::Uuid;
fn node(graph: &mut Graph, kind: NodeType) -> Uuid {
    graph.add_node(Node::new(kind, (0.0, 0.0)))
}
fn call(graph: &mut Graph, name: &str) -> Uuid {
    node(graph, NodeType::HostCall { name: name.into() })
}
fn wire(graph: &mut Graph, a: Uuid, output: &str, b: Uuid, input: &str) {
    let from = graph.nodes[&a]
        .pins
        .iter()
        .find(|p| p.name == output)
        .unwrap()
        .id;
    let to = graph.nodes[&b]
        .pins
        .iter()
        .find(|p| p.name == input)
        .unwrap()
        .id;
    graph.add_connection(Connection::new(from, to));
}
fn graph() -> (Graph, Uuid) {
    let mut graph = Graph::new();
    let speed = node(
        &mut graph,
        NodeType::Constant {
            value: ConstantValue::Float(120.0),
        },
    );
    let dt = call(&mut graph, "get_delta_time");
    let move_player = call(&mut graph, "move_player");
    let event = node(
        &mut graph,
        NodeType::Event {
            name: "On Update".into(),
        },
    );
    for (axis, arg) in [("get_input_x", "arg1"), ("get_input_y", "arg2")] {
        let input = call(&mut graph, axis);
        let velocity = node(
            &mut graph,
            NodeType::Arithmetic {
                op: ArithmeticOp::Mul,
            },
        );
        let delta = node(
            &mut graph,
            NodeType::Arithmetic {
                op: ArithmeticOp::Mul,
            },
        );
        wire(&mut graph, input, "result", velocity, "a");
        wire(&mut graph, speed, "value", velocity, "b");
        wire(&mut graph, velocity, "result", delta, "a");
        wire(&mut graph, dt, "result", delta, "b");
        wire(&mut graph, delta, "result", move_player, arg);
    }
    wire(&mut graph, event, "trigger", move_player, "trigger");
    let out = node(&mut graph, NodeType::Output);
    wire(&mut graph, move_player, "done", out, "in");
    (graph, speed)
}
fn spawn(project: &Project) -> GameWorld {
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        project,
        "main",
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    world
}
fn position(world: &GameWorld) -> [f32; 2] {
    let mut query = world.world_ref().query::<(&SourceId, &Transform)>();
    let (_, t) = query.iter().find(|(id, _)| id.0 == "player").unwrap();
    [t.position.x, t.position.y]
}
#[test]
fn generated_graph_matches_text_gameplay_and_preserves_inspection_across_save_restore() {
    let (graph, speed) = graph();
    let source = kiln_graph::document::encode(&graph).unwrap();
    let mut project =
        Project::from_toml(include_str!("../../samples/vertical_slice/kiln.toml")).unwrap();
    project.scenes.get_mut("main").unwrap().script = Some("scripts/gameplay_graph.ks".into());
    let mut graph_session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(source.clone())).unwrap();
    let manual="if script_phase()==\"update\" {move_player(get_input_x()*120.0*get_delta_time(),get_input_y()*120.0*get_delta_time());}";
    let mut text_session =
        SliceSession::with_scene_sources(&project, "main", |_| Ok(manual.into())).unwrap();
    let mut graph_world = spawn(&project);
    let mut text_world = spawn(&project);
    for axis in [(1.0, 0.0), (0.0, -0.5), (-0.4, 0.3)] {
        graph_session.step(&mut graph_world, axis, 0.1).unwrap();
        text_session.step(&mut text_world, axis, 0.1).unwrap();
        assert_eq!(position(&graph_world), position(&text_world));
    }
    let key = format!("__kiln_graph:{speed}");
    assert_eq!(
        graph_session.root_script_state(&key),
        Some(&kiln_ir::Value::F64(120.0))
    );
    let saved = GameSave::from_json(
        &graph_session
            .snapshot(&graph_world)
            .unwrap()
            .to_json()
            .unwrap(),
    )
    .unwrap();
    let (mut restored, mut world) = saved
        .restore(
            &project,
            |_| Ok(source.clone()),
            |_| Some((kiln_render::TextureId(0), [32, 32])),
        )
        .unwrap();
    assert_eq!(
        restored.root_script_state(&key),
        graph_session.root_script_state(&key)
    );
    restored.step(&mut world, (0.5, 0.2), 0.1).unwrap();
    text_session.step(&mut text_world, (0.5, 0.2), 0.1).unwrap();
    assert_eq!(position(&world), position(&text_world));
}
