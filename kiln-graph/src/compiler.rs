//! Compile node graph to IR bytecode.

use crate::{Graph, NodeType, ArithmeticOp, ComparisonOp};
use kiln_ir::{Program, Instruction, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Node not found: {0}")]
    NodeNotFound(Uuid),
    #[error("Pin not found: {0}")]
    PinNotFound(Uuid),
    #[error("Multiple connections to input pin: {0}")]
    MultipleInputs(Uuid),
    #[error("No connection to input pin: {0}")]
    NoInputConnection(Uuid),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}

/// Compile a graph to an IR program.
pub fn compile_graph(graph: &Graph) -> Result<Program, CompilerError> {
    let mut program = Program::new();
    let mut label_map: HashMap<Uuid, String> = HashMap::new();
    let mut pin_value_labels: HashMap<Uuid, String> = HashMap::new();
    let mut visited: HashSet<Uuid> = HashSet::new();

    // Find output node (should be unique)
    let output_node = graph.nodes.values()
        .find(|n| matches!(n.node_type, NodeType::Output))
        .ok_or_else(|| CompilerError::NodeNotFound(Uuid::nil()))?;

    // Traverse from output backwards
    compile_node_recursive(
        graph,
        output_node.id,
        &mut program,
        &mut label_map,
        &mut pin_value_labels,
        &mut visited,
    )?;

    Ok(program)
}

fn compile_node_recursive(
    graph: &Graph,
    node_id: Uuid,
    program: &mut Program,
    label_map: &mut HashMap<Uuid, String>,
    pin_value_labels: &mut HashMap<Uuid, String>,
    visited: &mut HashSet<Uuid>,
) -> Result<(), CompilerError> {
    if visited.contains(&node_id) {
        return Ok(());
    }
    visited.insert(node_id);

    let node = graph.get_node(node_id)
        .ok_or_else(|| CompilerError::NodeNotFound(node_id))?;

    match &node.node_type {
        NodeType::Event { name: _ } => {
            // Event nodes are entry points, compile their successors
            let trigger_pin = node.pins.iter()
                .find(|p| p.name == "trigger" && p.direction == crate::pin::PinDirection::Output)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_output_connection(graph, trigger_pin.id) {
                compile_node_recursive(
                    graph,
                    get_target_node_id(graph, conn.target_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }
        }
        NodeType::Constant { value } => {
            // Push constant value
            let label = format!("const_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            match value {
                crate::node::ConstantValue::Integer(n) => {
                    program.add_instruction(Instruction::PushConst(Value::I64(*n)));
                }
                crate::node::ConstantValue::Boolean(b) => {
                    program.add_instruction(Instruction::PushConst(Value::Bool(*b)));
                }
            }

            // Register output pin as providing this value
            let output_pin = node.pins.iter()
                .find(|p| p.direction == crate::pin::PinDirection::Output)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Arithmetic { op } => {
            // Compile inputs first
            let input_a_pin = node.pins.iter()
                .find(|p| p.name == "a" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            let input_b_pin = node.pins.iter()
                .find(|p| p.name == "b" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn_a) = find_input_connection(graph, input_a_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn_a.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            if let Some(conn_b) = find_input_connection(graph, input_b_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn_b.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            // Emit arithmetic op
            let label = format!("arith_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            let instr = match op {
                ArithmeticOp::Add => Instruction::Add,
                ArithmeticOp::Sub => Instruction::Sub,
                ArithmeticOp::Mul => Instruction::Mul,
                ArithmeticOp::Div => Instruction::Div,
            };
            program.add_instruction(instr);

            // Register output pin
            let output_pin = node.pins.iter()
                .find(|p| p.name == "result" && p.direction == crate::pin::PinDirection::Output)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Comparison { op } => {
            // Similar to arithmetic
            let input_a_pin = node.pins.iter()
                .find(|p| p.name == "a" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            let input_b_pin = node.pins.iter()
                .find(|p| p.name == "b" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn_a) = find_input_connection(graph, input_a_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn_a.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            if let Some(conn_b) = find_input_connection(graph, input_b_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn_b.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            let label = format!("cmp_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            let instr = match op {
                ComparisonOp::Equal => Instruction::Eq,
                ComparisonOp::LessThan => Instruction::Lt,
            };
            program.add_instruction(instr);

            let output_pin = node.pins.iter()
                .find(|p| p.name == "result" && p.direction == crate::pin::PinDirection::Output)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Branch => {
            // Compile condition, then true/false branches
            let cond_pin = node.pins.iter()
                .find(|p| p.name == "condition" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_input_connection(graph, cond_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            let label = format!("branch_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            // Jump if false to false branch
            let false_label = format!("{}_false", label);
            program.define_label(&false_label);
            program.add_instruction(Instruction::JumpIfNot(program.get_label(&false_label).unwrap()));

            // True branch
            let true_pin = node.pins.iter()
                .find(|p| p.name == "true" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            if let Some(conn) = find_input_connection(graph, true_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }

            // Jump past false branch
            let end_label = format!("{}_end", label);
            program.define_label(&end_label);
            program.add_instruction(Instruction::Jump(program.get_label(&end_label).unwrap()));

            // False branch
            let false_pin = node.pins.iter()
                .find(|p| p.name == "false" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;
            if let Some(conn) = find_input_connection(graph, false_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }
        }
        NodeType::HostCall { name } => {
            // Emit host call (simplified: use hash of name as ID)
            let label = format!("host_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(name);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::Output => {
            // Compile the input (trigger event)
            let input_pin = node.pins.iter()
                .find(|p| p.name == "in" && p.direction == crate::pin::PinDirection::Input)
                .ok_or_else(|| CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_input_connection(graph, input_pin.id) {
                compile_node_recursive(
                    graph,
                    get_source_node_id(graph, conn.source_pin)?,
                    program,
                    label_map,
                    pin_value_labels,
                    visited,
                )?;
            }
        }
    }

    Ok(())
}

fn find_input_connection(graph: &Graph, pin_id: Uuid) -> Option<&crate::Connection> {
    graph.connections.iter().find(|c| c.target_pin == pin_id)
}

fn find_output_connection(graph: &Graph, pin_id: Uuid) -> Option<&crate::Connection> {
    graph.connections.iter().find(|c| c.source_pin == pin_id)
}

fn get_source_node_id(graph: &Graph, pin_id: Uuid) -> Result<Uuid, CompilerError> {
    graph.get_pin_node(pin_id)
        .map(|n| n.id)
        .ok_or_else(|| CompilerError::PinNotFound(pin_id))
}

fn get_target_node_id(graph: &Graph, pin_id: Uuid) -> Result<Uuid, CompilerError> {
    graph.get_pin_node(pin_id)
        .map(|n| n.id)
        .ok_or_else(|| CompilerError::PinNotFound(pin_id))
}

fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u32);
    }
    hash
}
