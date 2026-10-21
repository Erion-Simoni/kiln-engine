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

/// A problem found while building or validating a [`Program`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramError {
    /// A label name was defined twice.
    DuplicateLabel(String),
    /// A jump or call points past the end of the program.
    InvalidJumpTarget {
        /// Index of the instruction holding the bad jump.
        instruction: usize,
        /// The target the jump points at.
        target: Label,
        /// Number of instructions in the program.
        length: usize,
    },
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::DuplicateLabel(name) => {
                write!(f, "Duplicate label: {name}")
            }
            ProgramError::InvalidJumpTarget {
                instruction,
                target,
                length,
            } => write!(
                f,
                "Invalid jump target {target} at instruction {instruction} (program has {length} instructions)"
            ),
        }
    }
}

impl std::error::Error for ProgramError {}

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

    /// Define a named label at the current instruction position.
    ///
    /// Returns an error when the name is already defined, because a duplicate
    /// label silently overwrites the first definition and makes jumps target
    /// the wrong instruction.
    pub fn define_label(&mut self, name: &str) -> Result<Label, ProgramError> {
        if self.labels.contains_key(name) {
            return Err(ProgramError::DuplicateLabel(name.to_string()));
        }
        let label = Label(self.instructions.len());
        self.labels.insert(name.to_string(), label);
        Ok(label)
    }

    /// Check that every jump and call target points at a real instruction.
    ///
    /// A jump to an instruction index outside the program would run past the
    /// end of the bytecode, so the compiler refuses to emit such a program.
    pub fn validate_jump_targets(&self) -> Result<(), ProgramError> {
        for (index, instruction) in self.instructions.iter().enumerate() {
            let target = match instruction {
                Instruction::Jump(label)
                | Instruction::JumpIf(label)
                | Instruction::JumpIfNot(label)
                | Instruction::Call(label) => *label,
                _ => continue,
            };
            if target.0 > self.instructions.len() {
                return Err(ProgramError::InvalidJumpTarget {
                    instruction: index,
                    target,
                    length: self.instructions.len(),
                });
            }
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_label_returns_the_current_position() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        let label = program.define_label("start").expect("label is new");
        assert_eq!(label, Label(1));
        assert_eq!(program.get_label("start"), Some(Label(1)));
    }

    #[test]
    fn define_label_rejects_duplicates() {
        let mut program = Program::new();
        program.define_label("start").expect("first label");
        let err = program
            .define_label("start")
            .expect_err("duplicate is rejected");
        assert_eq!(err, ProgramError::DuplicateLabel("start".to_string()));
        // The original definition is kept, so jumps still target it.
        assert_eq!(program.get_label("start"), Some(Label(0)));
    }

    #[test]
    fn validate_jump_targets_accepts_a_valid_program() {
        let mut program = Program::new();
        program.add_instruction(Instruction::Jump(Label(2)));
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        program.add_instruction(Instruction::Return);
        assert!(program.validate_jump_targets().is_ok());
    }

    #[test]
    fn validate_jump_targets_rejects_a_target_past_the_end() {
        let mut program = Program::new();
        program.add_instruction(Instruction::Jump(Label(9)));
        let err = program
            .validate_jump_targets()
            .expect_err("out of range jump is rejected");
        assert_eq!(
            err,
            ProgramError::InvalidJumpTarget {
                instruction: 0,
                target: Label(9),
                length: 1,
            }
        );
    }

    #[test]
    fn validate_jump_targets_rejects_a_bad_call() {
        let mut program = Program::new();
        program.add_instruction(Instruction::Call(Label(5)));
        let err = program
            .validate_jump_targets()
            .expect_err("out of range call is rejected");
        assert!(matches!(err, ProgramError::InvalidJumpTarget { .. }));
    }

    #[test]
    fn validate_jump_targets_ignores_non_jump_instructions() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(42)));
        assert!(program.validate_jump_targets().is_ok());
    }
}
