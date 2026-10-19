//! Compile node graph to IR bytecode.

use crate::{ArithmeticOp, ComparisonOp, Graph, NodeType};
use kiln_ir::{Instruction, Program, Value};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

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
    let output_node = graph
        .nodes
        .values()
        .find(|n| matches!(n.node_type, NodeType::Output))
        .ok_or(CompilerError::NodeNotFound(Uuid::nil()))?;

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

    let node = graph
        .get_node(node_id)
        .ok_or(CompilerError::NodeNotFound(node_id))?;

    match &node.node_type {
        NodeType::Event { name: _ } => {
            // Event nodes are entry points, compile their successors
            let trigger_pin = node
                .pins
                .iter()
                .find(|p| p.name == "trigger" && p.direction == crate::pin::PinDirection::Output)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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
                crate::node::ConstantValue::Float(f) => {
                    program.add_instruction(Instruction::PushConst(Value::F64(*f)));
                }
                crate::node::ConstantValue::Boolean(b) => {
                    program.add_instruction(Instruction::PushConst(Value::Bool(*b)));
                }
                crate::node::ConstantValue::String(_) => {
                    // String constants not yet supported in IR - use host call as placeholder
                    let id = simple_hash("string_const");
                    program.add_instruction(Instruction::HostCall(id));
                }
            }

            // Register output pin as providing this value
            let output_pin = node
                .pins
                .iter()
                .find(|p| p.direction == crate::pin::PinDirection::Output)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Arithmetic { op } => {
            // Compile inputs first
            let input_a_pin = node
                .pins
                .iter()
                .find(|p| p.name == "a" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
            let input_b_pin = node
                .pins
                .iter()
                .find(|p| p.name == "b" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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
            let output_pin = node
                .pins
                .iter()
                .find(|p| p.name == "result" && p.direction == crate::pin::PinDirection::Output)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Comparison { op } => {
            // Similar to arithmetic
            let input_a_pin = node
                .pins
                .iter()
                .find(|p| p.name == "a" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
            let input_b_pin = node
                .pins
                .iter()
                .find(|p| p.name == "b" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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

            let output_pin = node
                .pins
                .iter()
                .find(|p| p.name == "result" && p.direction == crate::pin::PinDirection::Output)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
            pin_value_labels.insert(output_pin.id, label);
        }
        NodeType::Branch => {
            // Compile condition, then true/false branches
            let cond_pin = node
                .pins
                .iter()
                .find(|p| p.name == "condition" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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
            program.add_instruction(Instruction::JumpIfNot(
                program.get_label(&false_label).unwrap(),
            ));

            // True branch
            let true_pin = node
                .pins
                .iter()
                .find(|p| p.name == "true" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
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
            let false_pin = node
                .pins
                .iter()
                .find(|p| p.name == "false" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;
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
            let input_pin = node
                .pins
                .iter()
                .find(|p| p.name == "in" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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
        NodeType::AnimationClip { clip_name } => {
            // Emit a host call to play the animation clip
            let label = format!("anim_clip_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            // Push clip name as string (simplified: use hash as placeholder)
            let id = simple_hash(clip_name);
            program.add_instruction(Instruction::HostCall(id));

            // Handle loop input if connected
            let loop_pin = node
                .pins
                .iter()
                .find(|p| p.name == "loop" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_input_connection(graph, loop_pin.id) {
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
        NodeType::AnimationState { state_name } => {
            // Emit a host call to enter the animation state
            let label = format!("anim_state_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(state_name);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::AnimationTransition => {
            // Compile condition and trigger
            let cond_pin = node
                .pins
                .iter()
                .find(|p| p.name == "condition" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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

            let label = format!("anim_trans_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            // Jump if false to skip transition
            let skip_label = format!("{}_skip", label);
            program.define_label(&skip_label);
            program.add_instruction(Instruction::JumpIfNot(
                program.get_label(&skip_label).unwrap(),
            ));

            // Trigger transition
            let trigger_pin = node
                .pins
                .iter()
                .find(|p| p.name == "trigger" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_input_connection(graph, trigger_pin.id) {
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
        NodeType::Selector => {
            // Emit host call for selector behavior tree node
            let label = format!("selector_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash("selector");
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::Sequence => {
            // Emit host call for sequence behavior tree node
            let label = format!("sequence_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash("sequence");
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::BehaviorCondition => {
            // Emit host call for behavior condition
            let label = format!("cond_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash("behavior_condition");
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::BehaviorAction { name } => {
            // Emit host call for behavior action
            let label = format!("action_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(name);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::AIState { state_name } => {
            // Emit host call for AI state
            let label = format!("ai_state_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(state_name);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::AITransition => {
            // Compile condition and trigger (similar to animation transition)
            let cond_pin = node
                .pins
                .iter()
                .find(|p| p.name == "condition" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

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

            let label = format!("ai_trans_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label.clone());

            // Jump if false to skip transition
            let skip_label = format!("{}_skip", label);
            program.define_label(&skip_label);
            program.add_instruction(Instruction::JumpIfNot(
                program.get_label(&skip_label).unwrap(),
            ));

            // Trigger transition
            let trigger_pin = node
                .pins
                .iter()
                .find(|p| p.name == "trigger" && p.direction == crate::pin::PinDirection::Input)
                .ok_or(CompilerError::PinNotFound(Uuid::nil()))?;

            if let Some(conn) = find_input_connection(graph, trigger_pin.id) {
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
        NodeType::Dialogue { dialogue_id } => {
            // Emit host call to show dialogue
            let label = format!("dialogue_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(dialogue_id);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::DialogueChoice => {
            // Emit host call for dialogue choice
            let label = format!("dialogue_choice_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash("dialogue_choice");
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::QuestStart { quest_id } => {
            // Emit host call to start quest
            let label = format!("quest_start_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(quest_id);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::QuestComplete { quest_id } => {
            // Emit host call to complete quest
            let label = format!("quest_complete_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(quest_id);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::QuestObjectiveUpdate {
            quest_id,
            objective_index,
        } => {
            // Emit host call to update quest objective
            let label = format!("quest_obj_update_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            // Push objective index
            program.add_instruction(Instruction::PushConst(Value::I64(*objective_index as i64)));

            let id = simple_hash(quest_id);
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::DataTableLookup {
            table_id,
            column_name,
        } => {
            // Emit host call for data table lookup
            let label = format!("datatable_lookup_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(&format!("{}:{}", table_id, column_name));
            program.add_instruction(Instruction::HostCall(id));
        }
        NodeType::DataTableRowGet { table_id } => {
            // Emit host call for data table row get
            let label = format!("datatable_row_{}", node_id);
            program.define_label(&label);
            label_map.insert(node_id, label);

            let id = simple_hash(table_id);
            program.add_instruction(Instruction::HostCall(id));
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
    graph
        .get_pin_node(pin_id)
        .map(|n| n.id)
        .ok_or(CompilerError::PinNotFound(pin_id))
}

fn get_target_node_id(graph: &Graph, pin_id: Uuid) -> Result<Uuid, CompilerError> {
    graph
        .get_pin_node(pin_id)
        .map(|n| n.id)
        .ok_or(CompilerError::PinNotFound(pin_id))
}

fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u32);
    }
    hash
}
