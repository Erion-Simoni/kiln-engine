//! Kiln node-graph crate — blueprint-style graph model that compiles to IR.

pub mod node;
pub mod graph;
pub mod pin;
pub mod connection;
pub mod compiler;

#[cfg(test)]
mod tests;

pub use node::{Node, NodeType, ConstantValue, ArithmeticOp, ComparisonOp};
pub use graph::Graph;
pub use pin::{Pin, PinDirection, PinType};
pub use connection::Connection;
pub use compiler::{compile_graph, CompilerError};
