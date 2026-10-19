//! Kiln node-graph crate — blueprint-style graph model that compiles to IR.

pub mod compiler;
pub mod connection;
pub mod graph;
pub mod node;
pub mod pin;

#[cfg(test)]
mod tests;

pub use compiler::{compile_graph, CompilerError};
pub use connection::Connection;
pub use graph::Graph;
pub use node::{ArithmeticOp, ComparisonOp, ConstantValue, Node, NodeType};
pub use pin::{Pin, PinDirection, PinType};
