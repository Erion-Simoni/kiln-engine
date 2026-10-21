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
    /// Push the value of a local variable.
    GetLocal(u16),
    /// Pop a value into a local variable.
    SetLocal(u16),
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
    ///
    /// `argc` values are popped from the stack and passed to the host as the
    /// call's arguments, and the host's result is pushed back. Keeping the
    /// argument count in the instruction lets calls nest: the arguments of a
    /// call are exactly the values pushed for it, not the whole stack.
    HostCall { id: u32, argc: u32 },
}

/// Bytecode program.
///
/// A program consists of a sequence of instructions and a set of named labels
/// that can be used as jump targets.
#[derive(Debug, Clone, Default)]
pub struct Program {
    instructions: Vec<Instruction>,
    labels: HashMap<String, Label>,
    /// Number of local variable slots the program uses.
    local_slots: u16,
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

    pub fn set_instruction(&mut self, index: usize, instr: Instruction) {
        if index < self.instructions.len() {
            self.instructions[index] = instr;
        }
    }

    /// Number of local variable slots the program uses.
    pub fn local_slots(&self) -> u16 {
        self.local_slots
    }

    /// Set the number of local variable slots the program uses.
    pub fn set_local_slots(&mut self, slots: u16) {
        self.local_slots = slots;
    }
}
