//! Compact dependency DAGs must compile without copying subgraphs exponentially.
use kiln_graph::{ArithmeticOp, Connection, ConstantValue, Graph, Node, NodeType};
use kiln_ir::{HostContext, Value, Vm, VmError};
fn connect(graph: &mut Graph, source: uuid::Uuid, out: &str, target: uuid::Uuid, input: &str) {
    let from = graph.nodes[&source]
        .pins
        .iter()
        .find(|p| p.name == out)
        .unwrap()
        .id;
    let to = graph.nodes[&target]
        .pins
        .iter()
        .find(|p| p.name == input)
        .unwrap()
        .id;
    graph.add_connection(Connection::new(from, to));
}
fn doubling(levels: usize) -> Graph {
    let mut graph = Graph::new();
    let mut previous = graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(1),
        },
        (0.0, 0.0),
    ));
    let mut output = "value";
    for _ in 0..levels {
        let node = graph.add_node(Node::new(
            NodeType::Arithmetic {
                op: ArithmeticOp::Add,
            },
            (0.0, 0.0),
        ));
        connect(&mut graph, previous, output, node, "a");
        connect(&mut graph, previous, output, node, "b");
        previous = node;
        output = "result";
    }
    let result = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, previous, output, result, "in");
    graph
}
#[derive(Default)]
struct Host(usize);
impl HostContext for Host {
    fn call_host(&mut self, id: u32, _: &[Value]) -> Result<Value, VmError> {
        assert_eq!(id, 58);
        self.0 += 1;
        Ok(Value::I64(0))
    }
}
#[test]
fn repeated_shared_inputs_compile_and_execute_once_per_node() {
    let graph = doubling(20);
    let source = kiln_graph::compiler::graph_source(&graph).unwrap();
    assert!(source.len() < 32768);
    let program = kiln_graph::compile_graph(&graph).unwrap();
    assert!(program.len() < 16000);
    let mut vm = Vm::new(Host::default());
    for tick in 1..=2 {
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(1 << 20)]);
        assert_eq!(vm.host().0, tick * graph.nodes.len());
    }
}

#[test]
fn maximum_dependency_depth_executes_and_next_level_rejects() {
    // Output + 62 operators + constant reaches the documented depth of 64.
    let mut graph = doubling(62);
    for node in graph.nodes.values_mut() {
        if let NodeType::Constant { value } = &mut node.node_type {
            *value = ConstantValue::Integer(0);
        }
    }
    let program = kiln_graph::compile_graph(&graph).unwrap();
    let mut vm = Vm::new(Host::default());
    vm.execute(&program).unwrap();
    assert_eq!(vm.stack(), &[Value::I64(0)]);
    assert_eq!(vm.host().0, graph.nodes.len());
    assert!(kiln_graph::compile_graph(&doubling(63))
        .unwrap_err()
        .to_string()
        .contains("depth exceeds 64"));
}

#[test]
fn legacy_shared_dependency_draft_upgrades_to_executable_source() {
    let graph = doubling(20);
    let current = kiln_graph::document::encode(&graph).unwrap();
    let (metadata, _) = current.split_once("// kiln-graph-end\n").unwrap();
    // The old generator rejected this valid DAG; preserve its exact draft marker.
    let legacy = format!(
        "{}// kiln-graph-end\n// Graph error: Generated graph exceeds 256 KiB; split the graph\n__kiln_graph_compile_error();\n",
        metadata.replacen("// kiln-graph-v2\n", "// kiln-graph-v1\n", 1)
    );
    let loaded = kiln_graph::document::decode(&legacy).unwrap().unwrap();
    assert_eq!(kiln_graph::document::encode(&loaded).unwrap(), current);
    let mut vm = Vm::new(Host::default());
    vm.execute(&kiln_graph::compile_graph(&loaded).unwrap())
        .unwrap();
    assert_eq!(vm.stack(), &[Value::I64(1 << 20)]);
}

#[test]
fn equality_accepts_text_and_boolean_values_and_refresh_retains_pin_ids() {
    for (left, right, expected) in [
        (
            ConstantValue::String("update".into()),
            ConstantValue::String("update".into()),
            true,
        ),
        (
            ConstantValue::Boolean(true),
            ConstantValue::Boolean(false),
            false,
        ),
    ] {
        let mut graph = Graph::new();
        let a = graph.add_node(Node::new(NodeType::Constant { value: left }, (0.0, 0.0)));
        let b = graph.add_node(Node::new(NodeType::Constant { value: right }, (0.0, 0.0)));
        let mut comparison = Node::new(
            NodeType::Comparison {
                op: kiln_graph::ComparisonOp::Equal,
            },
            (0.0, 0.0),
        );
        for pin in comparison
            .pins
            .iter_mut()
            .filter(|p| p.direction == kiln_graph::PinDirection::Input)
        {
            pin.pin_type = kiln_graph::PinType::Integer;
        }
        let ids = comparison.pins.iter().map(|p| p.id).collect::<Vec<_>>();
        comparison.refresh_pins();
        assert_eq!(
            ids,
            comparison.pins.iter().map(|p| p.id).collect::<Vec<_>>()
        );
        let compare = graph.add_node(comparison);
        let output = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
        connect(&mut graph, a, "value", compare, "a");
        connect(&mut graph, b, "value", compare, "b");
        connect(&mut graph, compare, "result", output, "in");
        let mut vm = Vm::new(Host::default());
        vm.execute(&kiln_graph::compile_graph(&graph).unwrap())
            .unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(expected)]);
    }
}
