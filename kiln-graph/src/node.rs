//! Node types for node graph.

use serde::{Deserialize, Serialize};
use crate::pin::{Pin, PinDirection, PinType};

/// Type of node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    /// Event input node (e.g., "On Start", "On Collision").
    Event {
        name: String,
    },
    /// Constant value node.
    Constant {
        value: ConstantValue,
    },
    /// Arithmetic operation.
    Arithmetic {
        op: ArithmeticOp,
    },
    /// Comparison operation.
    Comparison {
        op: ComparisonOp,
    },
    /// Branch/conditional.
    Branch,
    /// Host function call.
    HostCall {
        name: String,
    },
    /// Graph output (entry point to actual game logic).
    Output,
}

/// Constant value types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstantValue {
    Integer(i64),
    Boolean(bool),
}

/// Arithmetic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Comparison operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    LessThan,
}

/// A node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: uuid::Uuid,
    pub node_type: NodeType,
    pub pins: Vec<Pin>,
    pub position: (f32, f32), // UI position in graph editor
}

impl Node {
    pub fn new(node_type: NodeType, position: (f32, f32)) -> Self {
        let pins = Self::default_pins(&node_type);
        Self {
            id: uuid::Uuid::new_v4(),
            node_type,
            pins,
            position,
        }
    }

    fn default_pins(node_type: &NodeType) -> Vec<Pin> {
        match node_type {
            NodeType::Event { .. } => {
                vec![Pin::new(PinDirection::Output, PinType::Event, "trigger")]
            }
            NodeType::Constant { .. } => {
                vec![Pin::new(PinDirection::Output, PinType::Integer, "value")]
            }
            NodeType::Arithmetic { op: _ } => {
                vec![
                    Pin::new(PinDirection::Input, PinType::Integer, "a"),
                    Pin::new(PinDirection::Input, PinType::Integer, "b"),
                    Pin::new(PinDirection::Output, PinType::Integer, "result"),
                ]
            }
            NodeType::Comparison { .. } => {
                vec![
                    Pin::new(PinDirection::Input, PinType::Integer, "a"),
                    Pin::new(PinDirection::Input, PinType::Integer, "b"),
                    Pin::new(PinDirection::Output, PinType::Boolean, "result"),
                ]
            }
            NodeType::Branch => {
                vec![
                    Pin::new(PinDirection::Input, PinType::Boolean, "condition"),
                    Pin::new(PinDirection::Input, PinType::Event, "true"),
                    Pin::new(PinDirection::Input, PinType::Event, "false"),
                    Pin::new(PinDirection::Output, PinType::Event, "out"),
                ]
            }
            NodeType::HostCall { name: _ } => {
                vec![
                    Pin::new(PinDirection::Input, PinType::Event, "trigger"),
                    Pin::new(PinDirection::Output, PinType::Event, "done"),
                ]
            }
            NodeType::Output => {
                vec![Pin::new(PinDirection::Input, PinType::Event, "in")]
            }
        }
    }
}
