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
            ProgramError::DuplicateLabel(name) => write!(f, "Duplicate label: {name}"),
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
    source_offsets: HashMap<usize, usize>,
    local_names: HashMap<u16, String>,
    local_functions: HashMap<u16, Label>,
    local_scopes: HashMap<u16, (usize, usize)>,
    function_ranges: Vec<(usize, usize, String)>,
}

impl Program {
    /// Associate a compiled body range with its function; nested bodies retain
    /// their more specific ownership without per-instruction name copies.
    pub fn record_function(&mut self, start: usize, end: usize, name: &str) {
        self.function_ranges.push((start, end, name.to_owned()));
    }

    /// Function owning an instruction, or None for the entry program.
    pub fn function_name(&self, instruction: usize) -> Option<&str> {
        self.function_ranges
            .iter()
            .filter(|(start, end, _)| *start <= instruction && instruction < *end)
            .min_by_key(|(start, end, _)| end - start)
            .map(|(_, _, name)| name.as_str())
    }

    /// Record a function-local slot's entry label.
    pub fn record_local_function(&mut self, slot: u16, entry: Label) {
        self.local_functions.insert(slot, entry);
    }

    /// Function owning a local slot; None denotes entry-program locals.
    pub fn local_function(&self, slot: u16) -> Option<&str> {
        self.local_functions
            .get(&slot)
            .and_then(|entry| self.function_name(entry.0))
    }

    /// Mark a binding initialized and visible at the next instruction.
    pub fn record_local_scope_start(&mut self, slot: u16, instruction: usize) {
        self.local_scopes.insert(slot, (instruction, usize::MAX));
    }

    /// Close the lexical instruction range of an allocated binding.
    pub fn record_local_scope_end(&mut self, slot: u16, instruction: usize) {
        if let Some(range) = self.local_scopes.get_mut(&slot) {
            range.1 = instruction;
        }
    }

    /// Whether a binding is initialized and in the current function's lexical scope.
    /// Completed positions and programs without scope metadata report false.
    pub fn local_in_scope(&self, slot: u16, instruction: usize) -> bool {
        instruction < self.len()
            && self.local_function(slot) == self.function_name(instruction)
            && self
                .local_scopes
                .get(&slot)
                .is_some_and(|(start, end)| *start <= instruction && instruction < *end)
    }

    /// Resolve a current-frame lexical name, choosing the innermost shadowing binding.
    pub fn scoped_local(&self, name: &str, instruction: usize) -> Option<u16> {
        self.local_names
            .iter()
            .filter(|(slot, local)| {
                local.as_str() == name && self.local_in_scope(**slot, instruction)
            })
            .max_by_key(|(slot, _)| self.local_scopes[slot].0)
            .map(|(slot, _)| *slot)
    }

    /// Record a compiler-allocated local name for read-only live inspection.
    pub fn record_local_name(&mut self, slot: u16, name: &str) {
        self.local_names.insert(slot, name.to_owned());
    }

    /// Name associated with an allocated slot; scope lifetime is not implied.
    pub fn local_name(&self, slot: u16) -> Option<&str> {
        self.local_names.get(&slot).map(String::as_str)
    }

    /// Associate an instruction with its source byte offset, retaining a more
    /// precise mapping already recorded by a nested expression.
    pub fn record_source_offset(&mut self, instruction: usize, offset: usize) {
        self.source_offsets.entry(instruction).or_insert(offset);
    }

    /// Source byte offset for a failing or inspected instruction.
    pub fn source_offset(&self, instruction: usize) -> Option<usize> {
        self.source_offsets.get(&instruction).copied()
    }
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

    /// Check that every jump target points inside the instruction stream.
    ///
    /// Returns a human-readable error naming the first bad instruction.
    pub fn validate_jump_targets(&self) -> Result<(), ProgramError> {
        let len = self.instructions.len();
        for (index, instr) in self.instructions.iter().enumerate() {
            let label = match instr {
                Instruction::Jump(l)
                | Instruction::JumpIf(l)
                | Instruction::JumpIfNot(l)
                | Instruction::Call(l) => *l,
                _ => continue,
            };
            if label.0 > len {
                return Err(ProgramError::InvalidJumpTarget {
                    instruction: index,
                    target: label,
                    length: len,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_points_at_the_current_instruction() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        let label = program.define_label("start").expect("label is free");
        assert_eq!(label, Label(1));
        assert_eq!(program.get_label("start"), Some(Label(1)));
    }

    #[test]
    fn a_duplicate_label_is_reported() {
        let mut program = Program::new();
        program
            .define_label("start")
            .expect("the first label is free");
        let err = program
            .define_label("start")
            .expect_err("the duplicate is reported");
        assert!(matches!(err, ProgramError::DuplicateLabel(_)));
    }

    #[test]
    fn a_jump_past_the_end_of_the_program_is_reported() {
        let mut program = Program::new();
        program.add_instruction(Instruction::Jump(Label(9)));
        let err = program
            .validate_jump_targets()
            .expect_err("the bad jump is reported");
        assert!(matches!(err, ProgramError::InvalidJumpTarget { .. }));
    }

    #[test]
    fn a_jump_inside_the_program_is_accepted() {
        let mut program = Program::new();
        program.add_instruction(Instruction::Jump(Label(0)));
        program
            .validate_jump_targets()
            .expect("a jump to the start is fine");
    }
}
