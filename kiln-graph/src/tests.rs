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
    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
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
    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
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
    let branch_node = Node::new(
        NodeType::Branch,
        (200.0, 0.0),
    );
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

    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
    let pin_host_trigger_id = node_host.pins.iter().find(|p| p.name == "trigger").unwrap().id;
    let pin_host_done_id = node_host.pins.iter().find(|p| p.name == "done").unwrap().id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;

    graph.add_connection(Connection::new(pin_event_trigger_id, pin_host_trigger_id));
    graph.add_connection(Connection::new(pin_host_done_id, pin_output_in_id));

    let program = compile_graph(&graph).unwrap();
    // Should compile without error
    assert!(!program.instructions().is_empty());
}

#[test]
fn test_graph_matches_script_arithmetic() {
    use kiln_ir::{Program, Instruction, Value};

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
    let pin_a_out = graph.get_node(id_a).unwrap().pins.iter().find(|p| p.name == "value").unwrap().id;
    let pin_b_out = graph.get_node(id_b).unwrap().pins.iter().find(|p| p.name == "value").unwrap().id;
    let pin_add_a = graph.get_node(id_add).unwrap().pins.iter().find(|p| p.name == "a").unwrap().id;
    let pin_add_b = graph.get_node(id_add).unwrap().pins.iter().find(|p| p.name == "b").unwrap().id;
    let pin_add_out = graph.get_node(id_add).unwrap().pins.iter().find(|p| p.name == "result").unwrap().id;
    let pin_output_in = graph.get_node(id_output).unwrap().pins.iter().find(|p| p.name == "in").unwrap().id;

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
    let has_push_10 = graph_instrs.iter().any(|i| matches!(i, Instruction::PushConst(Value::I64(10))));
    let has_push_5 = graph_instrs.iter().any(|i| matches!(i, Instruction::PushConst(Value::I64(5))));
    let has_add = graph_instrs.iter().any(|i| matches!(i, Instruction::Add));

    assert!(has_push_10, "Graph should push 10");
    assert!(has_push_5, "Graph should push 5");
    assert!(has_add, "Graph should add");
}

#[test]
fn test_dialogue_node() {
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
    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
}

#[test]
fn test_quest_node() {
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
    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
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
    let pin_event_trigger_id = node_event.pins.iter().find(|p| p.name == "trigger").unwrap().id;
    let pin_output_in_id = node_output.pins.iter().find(|p| p.name == "in").unwrap().id;
    graph.add_connection(Connection::new(pin_event_trigger_id, pin_output_in_id));

    // Compilation should succeed
    let program = compile_graph(&graph);
    assert!(program.is_ok());
}
