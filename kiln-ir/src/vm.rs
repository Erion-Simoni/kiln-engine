//! Bytecode VM with host call interface.

use crate::program::{Instruction, Label, Program};
use crate::value::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VmError {
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),
    #[error("Label not found: {0}")]
    LabelNotFound(Label),
    #[error("Type error: expected {expected}, got {got:?}")]
    TypeError { expected: &'static str, got: Value },
    #[error("Host call error: {0}")]
    HostCallError(String),
}

/// Host call context for external functions.
///
/// Implement this trait to provide custom host functions that can be called
/// from bytecode via the HostCall instruction.
pub trait HostContext {
    /// Call a host function by ID with the given arguments.
    ///
    /// The function should return a Value that will be pushed onto the stack.
    fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError>;
}

/// Bytecode VM.
///
/// The VM uses a pre-allocated stack and does not allocate during instruction
/// dispatch when the stack remains within its initial capacity. Hot path is
/// allocation-free for typical programs that stay within stack limits.
pub struct Vm<H: HostContext> {
    stack: Vec<Value>,
    ip: usize,
    host: H,
    max_stack: usize,
}

impl<H: HostContext> Vm<H> {
    /// Create a new VM with default stack capacity (256, max 1024).
    pub fn new(host: H) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            ip: 0,
            host,
            max_stack: 1024,
        }
    }

    /// Create a new VM with custom max stack size.
    pub fn with_max_stack(host: H, max_stack: usize) -> Self {
        Self {
            stack: Vec::with_capacity(max_stack.min(256)),
            ip: 0,
            host,
            max_stack,
        }
    }

    /// Execute a bytecode program.
    ///
    /// The VM will run instructions sequentially until the program ends or an
    /// error occurs. The stack is cleared before execution.
    pub fn execute(&mut self, program: &Program) -> Result<(), VmError> {
        self.ip = 0;
        self.stack.clear();

        while self.ip < program.len() {
            self.step(program)?;
        }

        Ok(())
    }

    fn step(&mut self, program: &Program) -> Result<(), VmError> {
        let instr = program.instructions()
            .get(self.ip)
            .ok_or(VmError::InvalidOpcode(0xFF))?;

        match instr {
            Instruction::PushConst(v) => {
                self.push(*v)?;
            }
            Instruction::Pop => {
                self.pop()?;
            }
            Instruction::Dup => {
                let v = self.pop()?;
                self.push(v)?;
                self.push(v)?;
            }
            Instruction::Swap => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(b)?;
                self.push(a)?;
            }
            Instruction::Add => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::I64(a + b))?;
            }
            Instruction::Sub => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::I64(a - b))?;
            }
            Instruction::Mul => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::I64(a * b))?;
            }
            Instruction::Div => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                self.push(Value::I64(a / b))?;
            }
            Instruction::Neg => {
                let a = self.pop_as_i64()?;
                self.push(Value::I64(-a))?;
            }
            Instruction::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b))?;
            }
            Instruction::Ne => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b))?;
            }
            Instruction::Lt => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::Bool(a < b))?;
            }
            Instruction::Le => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::Bool(a <= b))?;
            }
            Instruction::Gt => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::Bool(a > b))?;
            }
            Instruction::Ge => {
                let b = self.pop_as_i64()?;
                let a = self.pop_as_i64()?;
                self.push(Value::Bool(a >= b))?;
            }
            Instruction::Jump(label) => {
                self.ip = label.0;
                return Ok(());
            }
            Instruction::JumpIf(label) => {
                let cond = self.pop_as_bool()?;
                if cond {
                    self.ip = label.0;
                    return Ok(());
                }
            }
            Instruction::JumpIfNot(label) => {
                let cond = self.pop_as_bool()?;
                if !cond {
                    self.ip = label.0;
                    return Ok(());
                }
            }
            Instruction::Call(label) => {
                self.ip = label.0;
                return Ok(());
            }
            Instruction::Return => {
                return Ok(());
            }
            Instruction::HostCall(func_id) => {
                let result = self.host.call_host(*func_id, &self.stack)?;
                self.stack.clear();
                self.push(result)?;
            }
        }

        self.ip += 1;
        Ok(())
    }

    fn push(&mut self, value: Value) -> Result<(), VmError> {
        if self.stack.len() >= self.max_stack {
            return Err(VmError::StackOverflow);
        }
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn pop_as_i64(&mut self) -> Result<i64, VmError> {
        self.pop()?.as_i64().ok_or(VmError::TypeError {
            expected: "i64",
            got: Value::Bool(false),
        })
    }

    fn pop_as_bool(&mut self) -> Result<bool, VmError> {
        self.pop()?.as_bool().ok_or(VmError::TypeError {
            expected: "bool",
            got: Value::I64(0),
        })
    }

    /// Get a reference to the current stack contents.
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHost;

    impl HostContext for TestHost {
        fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
            match func_id {
                0 => Ok(Value::I64(args.len() as i64)),
                1 => {
                    if let Some(v) = args.first() {
                        Ok(*v)
                    } else {
                        Ok(Value::I64(0))
                    }
                }
                _ => Err(VmError::HostCallError("Unknown host function".to_string())),
            }
        }
    }

    #[test]
    fn arith_add() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(5)));
        program.add_instruction(Instruction::PushConst(Value::I64(3)));
        program.add_instruction(Instruction::Add);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(8)]);
    }

    #[test]
    fn arith_sub() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(10)));
        program.add_instruction(Instruction::PushConst(Value::I64(4)));
        program.add_instruction(Instruction::Sub);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(6)]);
    }

    #[test]
    fn arith_mul() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(6)));
        program.add_instruction(Instruction::PushConst(Value::I64(7)));
        program.add_instruction(Instruction::Mul);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(42)]);
    }

    #[test]
    fn arith_div() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(20)));
        program.add_instruction(Instruction::PushConst(Value::I64(4)));
        program.add_instruction(Instruction::Div);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(5)]);
    }

    #[test]
    fn arith_div_by_zero() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(10)));
        program.add_instruction(Instruction::PushConst(Value::I64(0)));
        program.add_instruction(Instruction::Div);

        let mut vm = Vm::new(TestHost);
        assert!(matches!(vm.execute(&program), Err(VmError::DivisionByZero)));
    }

    #[test]
    fn branch_jump() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        program.add_instruction(Instruction::Jump(Label(2)));
        program.add_instruction(Instruction::PushConst(Value::I64(2)));

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(1), Value::I64(2)]);
    }

    #[test]
    fn branch_jump_if_true() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::Bool(true)));
        program.add_instruction(Instruction::JumpIf(Label(2)));
        program.add_instruction(Instruction::PushConst(Value::I64(2)));

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(2)]);
    }

    #[test]
    fn branch_jump_if_false() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::Bool(false)));
        program.add_instruction(Instruction::JumpIf(Label(2)));
        program.add_instruction(Instruction::PushConst(Value::I64(2)));

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(2)]);
    }

    #[test]
    fn host_call() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(42)));
        program.add_instruction(Instruction::HostCall(1));

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(42)]);
    }

    #[test]
    fn comparison_eq() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(5)));
        program.add_instruction(Instruction::PushConst(Value::I64(5)));
        program.add_instruction(Instruction::Eq);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(true)]);
    }

    #[test]
    fn comparison_lt() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(3)));
        program.add_instruction(Instruction::PushConst(Value::I64(5)));
        program.add_instruction(Instruction::Lt);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(true)]);
    }
}
