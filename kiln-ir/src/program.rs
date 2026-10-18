//! Bytecode program representation.

use crate::value::Value;
use std::collections::HashMap;
use std::fmt;

/// Label for jump targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub usize);

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Label({})", self.0)
    }
}

/// Bytecode instruction.
#[derive(Debug, Clone)]
pub enum Instruction {
    /// Push a constant value onto the stack.
    PushConst(Value),
    /// Pop the top value from the stack.
    Pop,
    /// Duplicate the top value on the stack.
    Dup,
    /// Swap the top two values on the stack.
    Swap,
    /// Add two i64 values.
    Add,
    /// Subtract two i64 values.
    Sub,
    /// Multiply two i64 values.
    Mul,
    /// Divide two i64 values.
    Div,
    /// Negate an i64 value.
    Neg,
    /// Compare two values for equality.
    Eq,
    /// Compare two values for inequality.
    Ne,
    /// Compare two i64 values for less-than.
    Lt,
    /// Compare two i64 values for less-than-or-equal.
    Le,
    /// Compare two i64 values for greater-than.
    Gt,
    /// Compare two i64 values for greater-than-or-equal.
    Ge,
    /// Unconditional jump to a label.
    Jump(Label),
    /// Jump to label if top of stack is true.
    JumpIf(Label),
    /// Jump to label if top of stack is false.
    JumpIfNot(Label),
    /// Call a function at a label.
    Call(Label),
    /// Return from a function.
    Return,
    /// Call a host function by ID.
    HostCall(u32),
}
    PushConst(Value),
    Pop,
    Dup,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Jump(Label),
    JumpIf(Label),
    JumpIfNot(Label),
    Call(Label),
    Return,
    HostCall(u32),
}

/// Bytecode program.
///
/// A program consists of a sequence of instructions and a set of named labels
/// that can be used as jump targets.
#[derive(Debug, Clone, Default)]
pub struct Program {
    instructions: Vec<Instruction>,
    labels: HashMap<String, Label>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_instruction(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    pub fn define_label(&mut self, name: &str) -> Label {
        let label = Label(self.instructions.len());
        self.labels.insert(name.to_string(), label);
        label
    }

    pub fn get_label(&self, name: &str) -> Option<Label> {
        self.labels.get(name).copied()
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
