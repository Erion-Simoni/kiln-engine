//! KilnScript Debugger support for the VM.
//!
//! Provides breakpoint management, stepping, and variable inspection for the
//! KilnScript bytecode VM.

use crate::program::{Instruction, Label, Program};
use crate::value::Value;
use std::collections::{HashMap, HashSet};

/// Debug event that can occur during VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugEvent {
    /// Hit a breakpoint at the given instruction index.
    BreakpointHit { instruction: usize },
    /// Stepped to a new instruction.
    Step { instruction: usize },
    /// VM execution completed.
    Completed,
    /// VM hit an error.
    Error { message: String },
}

/// Instruction address with source location info (for debugger UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionLocation {
    pub instruction: usize,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
    pub source_column: Option<u32>,
}

/// Breakpoint with optional condition.
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub instruction: usize,
    pub condition: Option<String>, // KilnScript expression that must evaluate to true
    pub enabled: bool,
    pub hit_count: usize,
}


/// Step mode for debugger control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepMode {
    #[default]
    None,
    StepInto,
    StepOver,
    StepOut,
    Continue,
}

/// Call frame for call stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function_label: Label,
    pub return_instruction: usize,
    pub base_pointer: u16,
    pub locals: HashMap<u16, Value>,
}

/// Watch expression for debugger UI.
#[derive(Debug, Clone)]
pub struct WatchExpression {
    pub expression: String,
    pub value: Option<Value>,
    pub error: Option<String>,
}

/// Debug state for a VM session.
#[derive(Debug, Default)]
pub struct DebugState {
    pub breakpoints: HashMap<usize, Breakpoint>,
    pub step_mode: StepMode,
    pub current_instruction: usize,
    pub call_stack: Vec<CallFrame>,
    pub locals: HashMap<u16, Value>,
    pub globals: HashMap<String, Value>,
    pub watch_expressions: Vec<WatchExpression>,
    pub last_event: Option<DebugEvent>,
    pub instruction_locations: HashMap<usize, InstructionLocation>,
}


/// Debugger controller that wraps a VM and provides debugging operations.
pub struct Debugger<H: crate::HostContext> {
    vm: crate::Vm<H>,
    state: DebugState,
    program: Program,
}

impl<H: crate::HostContext> Debugger<H> {
    /// Create a new debugger for the given program.
    pub fn new(program: Program, host: H) -> Self {
        let vm = crate::Vm::new(program.clone(), host);
        Self {
            vm,
            state: DebugState::default(),
            program,
        }
    }

    /// Get a mutable reference to the VM.
    pub fn vm_mut(&mut self) -> &mut crate::Vm<H> {
        &mut self.vm
    }

    /// Get a reference to the VM.
    pub fn vm(&self) -> &crate::Vm<H> {
        &self.vm
    }

    /// Get the current debug state.
    pub fn state(&self) -> &DebugState {
        &self.state
    }

    /// Get mutable debug state.
    pub fn state_mut(&mut self) -> &mut DebugState {
        &mut self.state
    }

    /// Get the program being debugged.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Set a breakpoint at the given instruction.
    pub fn set_breakpoint(&mut self, instruction: usize, condition: Option<String>) {
        self.state.breakpoints.insert(instruction, Breakpoint {
            instruction,
            condition,
            enabled: true,
            hit_count: 0,
        });
    }

    /// Remove a breakpoint.
    pub fn remove_breakpoint(&mut self, instruction: usize) -> bool {
        self.state.breakpoints.remove(&instruction).is_some()
    }

    /// Toggle breakpoint enabled state.
    pub fn toggle_breakpoint(&mut self, instruction: usize) -> bool {
        if let Some(bp) = self.state.breakpoints.get_mut(&instruction) {
            bp.enabled = !bp.enabled;
            true
        } else {
            false
        }
    }

    /// Clear all breakpoints.
    pub fn clear_breakpoints(&mut self) {
        self.state.breakpoints.clear();
    }

    /// Set step mode.
    pub fn set_step_mode(&mut self, mode: StepMode) {
        self.state.step_mode = mode;
    }

    /// Add a watch expression.
    pub fn add_watch(&mut self, expression: String) {
        self.state.watch_expressions.push(WatchExpression {
            expression,
            value: None,
            error: None,
        });
    }

    /// Remove a watch expression.
    pub fn remove_watch(&mut self, index: usize) -> bool {
        if index < self.state.watch_expressions.len() {
            self.state.watch_expressions.remove(index);
            true
        } else {
            false
        }
    }

    /// Evaluate all watch expressions.
    pub fn evaluate_watches(&mut self) {
        for watch in &mut self.state.watch_expressions {
            watch.value = None;
            watch.error = Some("Expression evaluation not yet implemented".to_string());
        }
    }

    /// Run the VM until the next debug event.
    pub fn run(&mut self) -> DebugEvent {
        self.state.step_mode = StepMode::Continue;
        self.execute_until_event()
    }

    /// Step into the next instruction.
    pub fn step_into(&mut self) -> DebugEvent {
        self.state.step_mode = StepMode::StepInto;
        self.execute_until_event()
    }

    /// Step over the next instruction (skip function calls).
    pub fn step_over(&mut self) -> DebugEvent {
        self.state.step_mode = StepMode::StepOver;
        self.execute_until_event()
    }

    /// Step out of the current function.
    pub fn step_out(&mut self) -> DebugEvent {
        self.state.step_mode = StepMode::StepOut;
        self.execute_until_event()
    }


    /// Execute VM until a debug event occurs.
    fn execute_until_event(&mut self) -> DebugEvent {
        loop {
            self.state.current_instruction = self.vm.instruction_pointer();

            // Check for breakpoint
            if let Some(bp) = self.state.breakpoints.get(&self.state.current_instruction) {
                if bp.enabled {
                    // Check condition if present
                    let should_break = bp.condition.as_ref().map(|c| self.evaluate_condition(c)).unwrap_or(true);
                    if should_break {
                        // We need mutable access to hit_count
                        if let Some(bp_mut) = self.state.breakpoints.get_mut(&self.state.current_instruction) {
                            bp_mut.hit_count += 1;
                        }
                        self.state.last_event = Some(DebugEvent::BreakpointHit { instruction: self.state.current_instruction });
                        self.update_state_from_vm();
                        return DebugEvent::BreakpointHit { instruction: self.state.current_instruction };
                    }
                }
            }

            // Check step mode
            match self.state.step_mode {
                StepMode::StepInto => {
                    self.state.step_mode = StepMode::None;
                    self.state.last_event = Some(DebugEvent::Step { instruction: self.state.current_instruction });
                    self.update_state_from_vm();
                    return DebugEvent::Step { instruction: self.state.current_instruction };
                }
                StepMode::StepOver => {
                    // Step over: execute one instruction, but don't stop inside calls
                    // For simplicity, treat as StepInto for now
                    self.state.step_mode = StepMode::None;
                    self.state.last_event = Some(DebugEvent::Step { instruction: self.state.current_instruction });
                    self.update_state_from_vm();
                    return DebugEvent::Step { instruction: self.state.current_instruction };
                }
                StepMode::StepOut => {
                    // Step out: continue until we return from current function
                    // For simplicity, treat as Continue for now
                    self.state.step_mode = StepMode::Continue;
                }
                StepMode::Continue => {}
                StepMode::None => {
                    // No stepping, check if we should break anyway
                }
            }

            // Execute one instruction
            match self.vm.step() {
                Ok(()) => {
                    // Check if execution completed
                    if self.vm.instruction_pointer() >= self.program.instructions().len() {
                        self.state.last_event = Some(DebugEvent::Completed);
                        self.update_state_from_vm();
                        return DebugEvent::Completed;
                    }
                }
                Err(e) => {
                    self.state.last_event = Some(DebugEvent::Error { message: e.to_string() });
                    self.update_state_from_vm();
                    return DebugEvent::Error { message: e.to_string() };
                }
            }
        }
    }

    /// Update debug state from VM after an event.
    fn update_state_from_vm(&mut self) {
        self.state.current_instruction = self.vm.instruction_pointer();
        // Update locals from VM (would need VM to expose stack/locals)
        // For now, we'll need to add VM methods to inspect state
    }

    /// Evaluate a breakpoint condition (simplified).
    fn evaluate_condition(&self, condition: &str) -> bool {
        // TODO: Implement condition evaluation
        // For now, always break
        true
    }

    /// Get the instruction at the current IP.
    pub fn current_instruction(&self) -> Option<&crate::program::Instruction> {
        self.program.instructions().get(self.state.current_instruction)
    }

    /// Get the source location for the current instruction.
    pub fn current_location(&self) -> Option<&InstructionLocation> {
        self.state.instruction_locations.get(&self.state.current_instruction)
    }

    /// Set instruction locations (from source map).
    pub fn set_instruction_locations(&mut self, locations: HashMap<usize, InstructionLocation>) {
        self.state.instruction_locations = locations;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostContext;
    use crate::value::Value;
    use crate::program::Instruction;

    struct TestHost;

    impl HostContext for TestHost {
        fn call_host(&mut self, _func_id: u32, _args: &[Value]) -> Result<Value, crate::VmError> {
            Ok(Value::I64(0))
        }
    }

    #[test]
    fn test_debugger_creation() {
        let program = Program::new();
        let _debugger = Debugger::new(program, TestHost);
    }

    #[test]
    fn test_breakpoint_management() {
        let program = Program::new();
        let mut debugger = Debugger::new(program, TestHost);

        debugger.set_breakpoint(10, None);
        assert!(debugger.state().breakpoints.contains_key(&10));

        debugger.remove_breakpoint(10);
        assert!(!debugger.state().breakpoints.contains_key(&10));
    }

    #[test]
    fn test_step_modes() {
        let program = Program::new();
        let mut debugger = Debugger::new(program, TestHost);

        debugger.set_step_mode(StepMode::StepInto);
        assert_eq!(debugger.state().step_mode, StepMode::StepInto);

        debugger.set_step_mode(StepMode::StepOver);
        assert_eq!(debugger.state().step_mode, StepMode::StepOver);
    }
}


