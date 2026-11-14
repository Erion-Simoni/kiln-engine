//! Tests for graph compilation.

use crate::*;

#[test]
fn test_constant_node() {
    let mut graph = Graph::new();
    let const_node = Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(42),
        },
        (0.0, 0.0),
    );
    graph.add_node(const_node);

    // Add output node (required by compiler)
    let output_node = Node::new(NodeType::Output, (100.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Add event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnStart".to_string(),
        },
        (-100.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Connect event to output
    let node_event = graph.get_node(id_event).unwrap();
    let node_output = graph.get_node(id_output).unwrap();
    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
}

#[test]
fn test_arithmetic_graph() {
    let mut graph = Graph::new();

    // Event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnStart".to_string(),
        },
        (0.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Constant 10
    let const_a = Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(10),
        },
        (100.0, 0.0),
    );
    let _id_a = graph.add_node(const_a);

    // Constant 5
    let const_b = Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(5),
        },
        (200.0, 0.0),
    );
    let _id_b = graph.add_node(const_b);

    // Add node
    let add_node = Node::new(
        NodeType::Arithmetic {
            op: ArithmeticOp::Add,
        },
        (300.0, 0.0),
    );
    let _id_add = graph.add_node(add_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (400.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect event -> output
    let node_event = graph.get_node(id_event).unwrap();
    let node_output = graph.get_node(id_output).unwrap();
    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
}

#[test]
fn test_branch_graph() {
    let mut graph = Graph::new();

    // Event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnStart".to_string(),
        },
        (0.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Constant true
    let const_node = Node::new(
        NodeType::Constant {
            value: ConstantValue::Boolean(true),
        },
        (100.0, 0.0),
    );
    let _id_const = graph.add_node(const_node);

    // Branch node
    let branch_node = Node::new(NodeType::Branch, (200.0, 0.0));
    let _id_branch = graph.add_node(branch_node);

    // Host call (print)
    let host_node = Node::new(
        NodeType::HostCall {
            name: "print".to_string(),
        },
        (300.0, 0.0),
    );
    let id_host = graph.add_node(host_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (400.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect: event -> host -> output
    let node_event = graph.get_node(id_event).unwrap();
    let node_host = graph.get_node(id_host).unwrap();
    let node_output = graph.get_node(id_output).unwrap();

    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_host_trigger_id = node_host
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_host_done_id = node_host.pins.iter().find(|p| p.name == "done").unwrap().id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;

    graph.add_connection(Connection::new(pin_event_trigger_id, pin_host_trigger_id));
    graph.add_connection(Connection::new(pin_host_done_id, pin_output_in_id));

    let argument = graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::String("branch".into()),
        },
        (0.0, 0.0),
    ));
    connect(&mut graph, argument, "value", id_host, "arg1");
    let program = compile_graph(&graph).unwrap();
    // Should compile without error
    assert!(!program.instructions().is_empty());
}

#[test]
fn test_graph_matches_script_arithmetic() {
    use kiln_ir::{Instruction, Program, Value};

    let mut graph = Graph::new();

    // Constant 10
    let const_a = Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(10),
        },
        (0.0, 0.0),
    );
    let id_a = graph.add_node(const_a);

    // Constant 5
    let const_b = Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(5),
        },
        (100.0, 0.0),
    );
    let id_b = graph.add_node(const_b);

    // Add node
    let add_node = Node::new(
        NodeType::Arithmetic {
            op: ArithmeticOp::Add,
        },
        (200.0, 0.0),
    );
    let id_add = graph.add_node(add_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (300.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect: const_a -> add.a, const_b -> add.b, add.result -> output.in
    let pin_a_out = graph
        .get_node(id_a)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "value")
        .unwrap()
        .id;
    let pin_b_out = graph
        .get_node(id_b)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "value")
        .unwrap()
        .id;
    let pin_add_a = graph
        .get_node(id_add)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "a")
        .unwrap()
        .id;
    let pin_add_b = graph
        .get_node(id_add)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "b")
        .unwrap()
        .id;
    let pin_add_out = graph
        .get_node(id_add)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "result")
        .unwrap()
        .id;
    let pin_output_in = graph
        .get_node(id_output)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "in")
        .unwrap()
        .id;

    graph.add_connection(Connection::new(pin_a_out, pin_add_a));
    graph.add_connection(Connection::new(pin_b_out, pin_add_b));
    graph.add_connection(Connection::new(pin_add_out, pin_output_in));

    let graph_program = compile_graph(&graph).unwrap();

    // Create equivalent program manually (like script would produce)
    let mut manual_program = Program::new();
    manual_program.add_instruction(Instruction::PushConst(Value::I64(10)));
    manual_program.add_instruction(Instruction::PushConst(Value::I64(5)));
    manual_program.add_instruction(Instruction::Add);

    // Graph should produce at least the same arithmetic instructions
    // (may have extra labels, but should have push 10, push 5, add)
    let graph_instrs = graph_program.instructions();

    // Check that we have at least 3 instructions
    assert!(graph_instrs.len() >= 3);

    // Check that we have the key arithmetic instructions
    let has_push_10 = graph_instrs
        .iter()
        .any(|i| matches!(i, Instruction::PushConst(Value::I64(10))));
    let has_push_5 = graph_instrs
        .iter()
        .any(|i| matches!(i, Instruction::PushConst(Value::I64(5))));
    let has_add = graph_instrs.iter().any(|i| matches!(i, Instruction::Add));

    assert!(has_push_10, "Graph should push 10");
    assert!(has_push_5, "Graph should push 5");
    assert!(has_add, "Graph should add");
}

#[test]
fn test_unregistered_dialogue_event_is_rejected() {
    let mut graph = Graph::new();

    // Event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnInteract".to_string(),
        },
        (0.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Dialogue node
    let dialogue_node = Node::new(
        NodeType::Dialogue {
            dialogue_id: "npc_intro".to_string(),
        },
        (100.0, 0.0),
    );
    let _id_dialogue = graph.add_node(dialogue_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (200.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect event -> output
    let node_event = graph.get_node(id_event).unwrap();
    let node_output = graph.get_node(id_output).unwrap();
    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program
        .unwrap_err()
        .to_string()
        .contains("unsupported lifecycle event"));
}

#[test]
fn test_unregistered_quest_event_is_rejected() {
    let mut graph = Graph::new();

    // Event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnKill".to_string(),
        },
        (0.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Quest start node
    let quest_node = Node::new(
        NodeType::QuestStart {
            quest_id: "main_quest".to_string(),
        },
        (100.0, 0.0),
    );
    let _id_quest = graph.add_node(quest_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (200.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect event -> output
    let node_event = graph.get_node(id_event).unwrap();
    let node_output = graph.get_node(id_output).unwrap();
    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program
        .unwrap_err()
        .to_string()
        .contains("unsupported lifecycle event"));
}

#[test]
fn test_datatable_node() {
    let mut graph = Graph::new();

    // Event node
    let event_node = Node::new(
        NodeType::Event {
            name: "OnStart".to_string(),
        },
        (0.0, 0.0),
    );
    let id_event = graph.add_node(event_node);

    // Data table lookup node
    let datatable_node = Node::new(
        NodeType::DataTableLookup {
            table_id: "items".to_string(),
            column_name: "damage".to_string(),
        },
        (100.0, 0.0),
    );
    let _id_datatable = graph.add_node(datatable_node);

    // Output node
    let output_node = Node::new(NodeType::Output, (200.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Connect event -> output
    let node_event = graph.get_node(id_event).unwrap();
    let node_output = graph.get_node(id_output).unwrap();
    let pin_event_trigger_id = node_event
        .pins
        .iter()
        .find(|p| p.name == "trigger")
        .unwrap()
        .id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
}

#[test]
fn test_float_constant_compiles_to_f64() {
    use kiln_ir::{Instruction, Value};

    let mut graph = Graph::new();

    let id_a = graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::Float(2.5),
        },
        (0.0, 0.0),
    ));
    let id_b = graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::Float(4.0),
        },
        (100.0, 0.0),
    ));
    let id_add = graph.add_node(Node::new(
        NodeType::Arithmetic {
            op: ArithmeticOp::Add,
        },
        (200.0, 0.0),
    ));
    let id_output = graph.add_node(Node::new(NodeType::Output, (300.0, 0.0)));

    let out_pin = |graph: &Graph, id, name: &str| {
        graph
            .get_node(id)
            .unwrap()
            .pins
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .id
    };
    let in_pin = |graph: &Graph, id, name: &str| {
        graph
            .get_node(id)
            .unwrap()
            .pins
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .id
    };

    graph.add_connection(Connection::new(
        out_pin(&graph, id_a, "value"),
        in_pin(&graph, id_add, "a"),
    ));
    graph.add_connection(Connection::new(
        out_pin(&graph, id_b, "value"),
        in_pin(&graph, id_add, "b"),
    ));
    graph.add_connection(Connection::new(
        out_pin(&graph, id_add, "result"),
        in_pin(&graph, id_output, "in"),
    ));

    // Float constants expose a Float pin, not Integer
    let pin = graph
        .get_node(id_a)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "value")
        .unwrap();
    assert_eq!(pin.pin_type, crate::pin::PinType::Float);

    let program = compile_graph(&graph).unwrap();
    let has_f64 = |v: f64| {
        program
            .instructions()
            .iter()
            .any(|i| matches!(i, Instruction::PushConst(Value::F64(x)) if *x == v))
    };
    assert!(has_f64(2.5), "Graph should push 2.5");
    assert!(has_f64(4.0), "Graph should push 4.0");
    assert!(program
        .instructions()
        .iter()
        .any(|i| matches!(i, Instruction::Add)));
}

#[test]
fn test_string_constant_node_compiles_to_push() {
    use kiln_ir::{Instruction, Value};

    let mut graph = Graph::new();
    let const_node = Node::new(
        NodeType::Constant {
            value: ConstantValue::String("hello".to_string()),
        },
        (0.0, 0.0),
    );
    graph.add_node(const_node);
    // add_node returns the id; find it back to wire it to the output.
    let id_const = graph
        .nodes
        .values()
        .find(|n| matches!(n.node_type, NodeType::Constant { .. }))
        .unwrap()
        .id;

    let output_node = Node::new(NodeType::Output, (100.0, 0.0));
    let id_output = graph.add_node(output_node);

    // Feed the string constant straight into the output's `in` pin so the
    // compiler traverses to it.
    let const_output_id = graph
        .get_node(id_const)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.direction == crate::pin::PinDirection::Output)
        .unwrap()
        .id;
    let pin_output_in_id = graph
        .get_node(id_output)
        .unwrap()
        .pins
        .iter()
        .find(|p| p.name == "in")
        .unwrap()
        .id;
    graph.add_connection(Connection::new(const_output_id, pin_output_in_id));

    let program = compile_graph(&graph).expect("string graph compiles");
    let has_str_push = program
        .instructions()
        .iter()
        .any(|instr| matches!(instr, Instruction::PushConst(Value::Str(s)) if s == "hello"));
    assert!(
        has_str_push,
        "expected a PushConst(Str) instruction, got {:?}",
        program.instructions()
    );
}

fn connect(graph: &mut Graph, source: uuid::Uuid, output: &str, target: uuid::Uuid, input: &str) {
    let find = |id, name: &str| {
        graph.nodes[&id]
            .pins
            .iter()
            .find(|pin| pin.name == name)
            .unwrap()
            .id
    };
    let from = find(source, output);
    let to = find(target, input);
    graph.add_connection(Connection::new(from, to));
}
fn constant(graph: &mut Graph, value: i64) -> uuid::Uuid {
    graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::Integer(value),
        },
        (0.0, 0.0),
    ))
}
fn call(graph: &mut Graph, name: &str) -> uuid::Uuid {
    graph.add_node(Node::new(
        NodeType::HostCall { name: name.into() },
        (0.0, 0.0),
    ))
}
#[derive(Default)]
struct Host {
    calls: Vec<(u32, Vec<kiln_ir::Value>)>,
}
impl kiln_ir::HostContext for Host {
    fn call_host(
        &mut self,
        id: u32,
        args: &[kiln_ir::Value],
    ) -> Result<kiln_ir::Value, kiln_ir::VmError> {
        self.calls.push((id, args.to_vec()));
        Ok(if id == 4 {
            kiln_ir::Value::I64(21)
        } else {
            args.last().cloned().unwrap_or(kiln_ir::Value::I64(0))
        })
    }
}
#[test]
fn shared_host_ids_arity_fanout_and_values_execute_once_per_tick() {
    use kiln_ir::{Value, Vm};
    let mut graph = Graph::new();
    let getter = call(&mut graph, "get_score");
    let add = graph.add_node(Node::new(
        NodeType::Arithmetic {
            op: ArithmeticOp::Add,
        },
        (0.0, 0.0),
    ));
    let setter = call(&mut graph, "add_score");
    let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, getter, "result", add, "a");
    connect(&mut graph, getter, "result", add, "b");
    connect(&mut graph, add, "result", setter, "arg1");
    connect(&mut graph, setter, "result", out, "in");
    let mut vm = Vm::new(Host::default());
    let program = compile_graph(&graph).unwrap();
    for tick in 1..=2 {
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(42)]);
        assert_eq!(
            vm.host().calls.iter().filter(|(id, _)| *id == 4).count(),
            tick
        );
        let setters = vm
            .host()
            .calls
            .iter()
            .filter(|(id, _)| *id == 5)
            .collect::<Vec<_>>();
        assert_eq!(setters.len(), tick);
        assert_eq!(setters[tick - 1].1, vec![Value::I64(42)]);
    }
}
#[test]
fn branch_runs_only_selected_side_effect_and_trigger_chain_retains_order() {
    let mut graph = Graph::new();
    let condition = graph.add_node(Node::new(
        NodeType::Constant {
            value: ConstantValue::Boolean(false),
        },
        (0.0, 0.0),
    ));
    let a = constant(&mut graph, 1);
    let b = constant(&mut graph, 2);
    let yes = call(&mut graph, "add_score");
    let no = call(&mut graph, "add_score");
    let after = call(&mut graph, "add_score");
    let merge = graph.add_node(Node::new(NodeType::Branch, (0.0, 0.0)));
    let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, condition, "value", merge, "condition");
    connect(&mut graph, a, "value", yes, "arg1");
    connect(&mut graph, b, "value", no, "arg1");
    connect(&mut graph, yes, "done", merge, "true");
    connect(&mut graph, no, "done", merge, "false");
    connect(&mut graph, merge, "out", after, "trigger");
    connect(&mut graph, a, "value", after, "arg1");
    connect(&mut graph, after, "done", out, "in");
    let mut vm = kiln_ir::Vm::new(Host::default());
    vm.execute(&compile_graph(&graph).unwrap()).unwrap();
    let writes = vm
        .host()
        .calls
        .iter()
        .filter(|(id, _)| *id == 5)
        .map(|(_, args)| args[0].clone())
        .collect::<Vec<_>>();
    assert_eq!(writes, vec![kiln_ir::Value::I64(2), kiln_ir::Value::I64(1)]);
}
#[test]
fn incomplete_cycles_duplicate_inputs_and_unsupported_nodes_have_diagnostics() {
    let mut graph = Graph::new();
    let setter = call(&mut graph, "add_score");
    let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, setter, "done", out, "in");
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("arg1"));
    let a = constant(&mut graph, 2);
    connect(&mut graph, a, "value", setter, "arg1");
    connect(&mut graph, setter, "done", setter, "trigger");
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("cyclic"));
    graph.connections.pop();
    connect(&mut graph, a, "value", setter, "arg1");
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("multiple connections"));
    graph.connections.clear();
    let stub = graph.add_node(Node::new(
        NodeType::Dialogue {
            dialogue_id: "unimplemented".into(),
        },
        (0.0, 0.0),
    ));
    let pin = graph.nodes[&stub]
        .pins
        .iter()
        .find(|p| p.direction == PinDirection::Output)
        .unwrap()
        .name
        .clone();
    connect(&mut graph, stub, &pin, out, "in");
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("specialized node"));
}
#[test]
fn documents_are_deterministic_roundtrip_guard_manual_changes_and_recover_invalid_drafts() {
    let mut graph = Graph::new();
    let a = constant(&mut graph, i64::MIN);
    let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, a, "value", out, "in");
    let source = document::encode(&graph).unwrap();
    let restored = document::decode(&source).unwrap().unwrap();
    assert_eq!(document::encode(&restored).unwrap(), source);
    assert!(document::decode(&(source.clone() + "print(42);\n"))
        .unwrap_err()
        .contains("edited as text"));
    let mut vm = kiln_ir::Vm::new(Host::default());
    vm.execute(&compile_graph(&restored).unwrap()).unwrap();
    assert_eq!(vm.stack(), &[kiln_ir::Value::I64(i64::MIN)]);
    graph.nodes.get_mut(&a).unwrap().node_type = NodeType::HostCall {
        name: "not_supported".into(),
    };
    graph.nodes.get_mut(&a).unwrap().refresh_pins();
    graph.connections.clear();
    connect(&mut graph, a, "done", out, "in");
    let draft = document::encode(&graph).unwrap();
    assert!(draft.contains("__kiln_graph_compile_error();"));
    assert!(document::decode(&draft).unwrap().is_some());
    assert!(compile_graph(&graph).is_err());
}

#[test]
fn graph_limits_and_strings_fail_safely_and_preserve_exact_values() {
    for value in ["quote \" slash \\ newline\n🦀", "", "tab\treturn\r"] {
        let mut graph = Graph::new();
        let a = graph.add_node(Node::new(
            NodeType::Constant {
                value: ConstantValue::String(value.into()),
            },
            (0.0, 0.0),
        ));
        let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
        connect(&mut graph, a, "value", out, "in");
        let mut vm = kiln_ir::Vm::new(Host::default());
        vm.execute(&compile_graph(&graph).unwrap()).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::Str(value.into())]);
        let encoded = document::encode(&graph).unwrap();
        assert!(encoded.lines().all(|line| line.len() < 300));
        assert!(document::decode(&encoded).unwrap().is_some());
    }
    let mut graph = Graph::new();
    let mut previous = constant(&mut graph, 1);
    let zero = constant(&mut graph, 0);
    for _ in 0..65 {
        let add = graph.add_node(Node::new(
            NodeType::Arithmetic {
                op: ArithmeticOp::Add,
            },
            (0.0, 0.0),
        ));
        connect(&mut graph, previous, "value", add, "a");
        connect(&mut graph, zero, "value", add, "b");
        // Make the next dependency use the arithmetic result, not a stale pin.
        graph
            .nodes
            .get_mut(&add)
            .unwrap()
            .pins
            .iter_mut()
            .find(|p| p.name == "result")
            .unwrap()
            .name = "value".into();
        previous = add;
    }
    let out = graph.add_node(Node::new(NodeType::Output, (0.0, 0.0)));
    connect(&mut graph, previous, "value", out, "in");
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("depth exceeds"));
    for _ in 0..256 {
        constant(&mut graph, 0);
    }
    assert!(compile_graph(&graph)
        .unwrap_err()
        .to_string()
        .contains("256 nodes"));
}
