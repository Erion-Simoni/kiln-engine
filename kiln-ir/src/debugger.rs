//! KilnScript Debugger support for the VM.
//!
//! Provides breakpoint management, stepping, and variable inspection for the
//! KilnScript bytecode VM.

use crate::program::{Label, Program};
use crate::value::Value;
use std::collections::HashMap;

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
        let mut vm = crate::Vm::new(host);
        vm.begin_execution(&program);
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
        self.state.breakpoints.insert(
            instruction,
            Breakpoint {
                instruction,
                condition,
                enabled: true,
                hit_count: 0,
            },
        );
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

    /// Restart bytecode execution and locals; host-side state is retained.
    pub fn restart(&mut self) {
        self.vm
            .restore_locals(&self.program, Vec::new())
            .expect("empty locals are valid");
        self.vm.begin_execution(&self.program);
        self.state.last_event = None;
        self.state.step_mode = StepMode::None;
        self.update_state_from_vm();
    }

    /// Execute at least one instruction when stepping or resuming a breakpoint.
    fn execute_until_event(&mut self) -> DebugEvent {
        if matches!(self.state.last_event, Some(DebugEvent::Completed)) {
            return DebugEvent::Completed;
        }
        let initial_depth = self.vm.call_stack().len();
        let mut skip_breakpoint = matches!(self.state.last_event,Some(DebugEvent::BreakpointHit{instruction}) if instruction==self.vm.instruction_pointer())
            || matches!(
                self.state.step_mode,
                StepMode::StepInto | StepMode::StepOver | StepMode::StepOut
            );
        loop {
            let ip = self.vm.instruction_pointer();
            if ip >= self.program.len() {
                return self.record_event(DebugEvent::Completed);
            }
            if !skip_breakpoint {
                if let Some(bp) = self.state.breakpoints.get_mut(&ip).filter(|bp| bp.enabled) {
                    if bp.condition.is_some() {
                        return self.record_event(DebugEvent::Error {
                            message: "Conditional bytecode breakpoints are not supported".into(),
                        });
                    }
                    bp.hit_count += 1;
                    return self.record_event(DebugEvent::BreakpointHit { instruction: ip });
                }
            }
            skip_breakpoint = false;
            match self.vm.step(&self.program) {
                Ok(false) => return self.record_event(DebugEvent::Completed),
                Err(error) => {
                    return self.record_event(DebugEvent::Error {
                        message: error.to_string(),
                    })
                }
                Ok(true) => {}
            }
            let ip = self.vm.instruction_pointer();
            if ip >= self.program.len() {
                return self.record_event(DebugEvent::Completed);
            }
            let depth = self.vm.call_stack().len();
            let stop = match self.state.step_mode {
                StepMode::StepInto => true,
                StepMode::StepOver => depth <= initial_depth,
                StepMode::StepOut => depth < initial_depth,
                _ => false,
            };
            if stop {
                return self.record_event(DebugEvent::Step { instruction: ip });
            }
        }
    }

    fn record_event(&mut self, event: DebugEvent) -> DebugEvent {
        self.state.last_event = Some(event.clone());
        self.state.step_mode = StepMode::None;
        self.update_state_from_vm();
        event
    }

    fn update_state_from_vm(&mut self) {
        self.state.current_instruction = self.vm.instruction_pointer();
        self.state.locals = self
            .vm
            .locals()
            .iter()
            .cloned()
            .enumerate()
            .map(|(slot, value)| (slot as u16, value))
            .collect();
    }

    /// Get the instruction at the current IP.
    pub fn current_instruction(&self) -> Option<&crate::program::Instruction> {
        self.program
            .instructions()
            .get(self.state.current_instruction)
    }

    /// Get the source location for the current instruction.
    pub fn current_location(&self) -> Option<&InstructionLocation> {
        self.state
            .instruction_locations
            .get(&self.state.current_instruction)
    }

    /// Set instruction locations (from source map).
    pub fn set_instruction_locations(&mut self, locations: HashMap<usize, InstructionLocation>) {
        self.state.instruction_locations = locations;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::value::Value;
    use crate::HostContext;

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

#[cfg(test)]
mod execution_tests {
    use super::*;
    use crate::{HostContext, Instruction, VmError};
    struct Host;
    impl HostContext for Host {
        fn call_host(&mut self, _: u32, _: &[Value]) -> Result<Value, VmError> {
            Ok(Value::I64(0))
        }
    }
    #[test]
    fn stepping_executes_locals_and_resuming_does_not_rehit_the_same_breakpoint() {
        let mut p = Program::new();
        p.set_local_slots(1);
        p.add_instruction(Instruction::PushConst(Value::I64(42)));
        p.add_instruction(Instruction::SetLocal(0));
        p.add_instruction(Instruction::GetLocal(0));
        let mut d = Debugger::new(p, Host);
        d.set_breakpoint(1, None);
        assert_eq!(d.step_into(), DebugEvent::Step { instruction: 1 });
        assert_eq!(d.run(), DebugEvent::BreakpointHit { instruction: 1 });
        assert_eq!(d.run(), DebugEvent::Completed);
        assert_eq!(d.state.locals[&0], Value::I64(42));
        assert_eq!(d.vm.stack(), &[Value::I64(42)]);
        assert_eq!(d.state.breakpoints[&1].hit_count, 1);
        assert_eq!(d.run(), DebugEvent::Completed);
        d.restart();
        assert_eq!(d.vm.instruction_pointer(), 0);
        assert_eq!(d.state.locals[&0], Value::I64(0));
    }
    #[test]
    fn bytecode_step_over_and_out_follow_real_call_depth() {
        let mut p = Program::new();
        for op in [
            Instruction::Call(Label(3)),
            Instruction::PushConst(Value::I64(9)),
            Instruction::Return,
            Instruction::PushConst(Value::I64(7)),
            Instruction::Return,
        ] {
            p.add_instruction(op);
        }
        let mut d = Debugger::new(p, Host);
        assert_eq!(d.step_over(), DebugEvent::Step { instruction: 1 });
        assert_eq!(d.vm.stack(), &[Value::I64(7)]);
        d.restart();
        assert_eq!(d.step_into(), DebugEvent::Step { instruction: 3 });
        assert_eq!(d.step_out(), DebugEvent::Step { instruction: 1 });
        assert!(d.vm.call_stack().is_empty());
    }
    #[test]
    fn instruction_budget_bounds_continue_single_steps_and_each_restart() {
        let mut p = Program::new();
        p.add_instruction(Instruction::Jump(Label(0)));
        let mut d = Debugger::new(p, Host);
        d.vm_mut().set_instruction_budget(3);
        for _ in 0..3 {
            assert_eq!(d.step_into(), DebugEvent::Step { instruction: 0 });
        }
        assert!(matches!(d.step_into(),DebugEvent::Error{message} if message.contains("budget")));
        d.restart();
        assert!(matches!(d.run(),DebugEvent::Error{message} if message.contains("budget")));
        let mut vm = crate::Vm::new(Host);
        vm.set_instruction_budget(1);
        let mut finite = Program::new();
        finite.add_instruction(Instruction::PushConst(Value::I64(1)));
        finite.add_instruction(Instruction::Pop);
        for _ in 0..2 {
            assert!(matches!(
                vm.execute(&finite),
                Err(VmError::InstructionBudgetExceeded)
            ));
            assert_eq!(vm.instruction_pointer(), 1);
        }
    }
}
