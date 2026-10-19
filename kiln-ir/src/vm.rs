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

/// Numeric/comparison binary operations handled uniformly by [`binop`].
#[derive(Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Applies `op` to two values.
///
/// If either operand is `F64`, both are evaluated as floats (`I64` is
/// promoted); otherwise integer arithmetic is used. Integer division by zero
/// is an error; float division by zero follows IEEE-754 (±inf / NaN).
fn binop(op: BinOp, a: Value, b: Value) -> Result<Value, VmError> {
    let type_err = |got: Value| VmError::TypeError {
        expected: "number",
        got,
    };
    if matches!(a, Value::F64(_)) || matches!(b, Value::F64(_)) {
        let x = a.to_f64().ok_or_else(|| type_err(a))?;
        let y = b.to_f64().ok_or_else(|| type_err(b))?;
        return Ok(match op {
            BinOp::Add => Value::F64(x + y),
            BinOp::Sub => Value::F64(x - y),
            BinOp::Mul => Value::F64(x * y),
            BinOp::Div => Value::F64(x / y),
            BinOp::Lt => Value::Bool(x < y),
            BinOp::Le => Value::Bool(x <= y),
            BinOp::Gt => Value::Bool(x > y),
            BinOp::Ge => Value::Bool(x >= y),
        });
    }
    let x = a.as_i64().ok_or_else(|| type_err(a))?;
    let y = b.as_i64().ok_or_else(|| type_err(b))?;
    Ok(match op {
        BinOp::Add => Value::I64(x + y),
        BinOp::Sub => Value::I64(x - y),
        BinOp::Mul => Value::I64(x * y),
        BinOp::Div => {
            if y == 0 {
                return Err(VmError::DivisionByZero);
            }
            Value::I64(x / y)
        }
        BinOp::Lt => Value::Bool(x < y),
        BinOp::Le => Value::Bool(x <= y),
        BinOp::Gt => Value::Bool(x > y),
        BinOp::Ge => Value::Bool(x >= y),
    })
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
    /// The VM will run instructions sequentially until the program ends, a
    /// `Return` instruction is reached, or an error occurs. The stack is
    /// cleared before execution.
    pub fn execute(&mut self, program: &Program) -> Result<(), VmError> {
        self.ip = 0;
        self.stack.clear();

        while self.ip < program.len() {
            if !self.step(program)? {
                break;
            }
        }

        Ok(())
    }

    /// Executes a single instruction.
    ///
    /// Returns `true` to continue execution, `false` when the program should
    /// halt (a `Return` instruction).
    fn step(&mut self, program: &Program) -> Result<bool, VmError> {
        let instr = program
            .instructions()
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
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Add, a, b)?)?;
            }
            Instruction::Sub => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Sub, a, b)?)?;
            }
            Instruction::Mul => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Mul, a, b)?)?;
            }
            Instruction::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Div, a, b)?)?;
            }
            Instruction::Neg => {
                let a = self.pop()?;
                let negated = match a {
                    Value::I64(v) => Value::I64(-v),
                    Value::F64(v) => Value::F64(-v),
                    other => {
                        return Err(VmError::TypeError {
                            expected: "number",
                            got: other,
                        })
                    }
                };
                self.push(negated)?;
            }
            Instruction::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(Value::values_equal(a, b)))?;
            }
            Instruction::Ne => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(!Value::values_equal(a, b)))?;
            }
            Instruction::Lt => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Lt, a, b)?)?;
            }
            Instruction::Le => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Le, a, b)?)?;
            }
            Instruction::Gt => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Gt, a, b)?)?;
            }
            Instruction::Ge => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binop(BinOp::Ge, a, b)?)?;
            }
            Instruction::Jump(label) => {
                self.ip = label.0;
                return Ok(true);
            }
            Instruction::JumpIf(label) => {
                let cond = self.pop_as_bool()?;
                if cond {
                    self.ip = label.0;
                    return Ok(true);
                }
            }
            Instruction::JumpIfNot(label) => {
                let cond = self.pop_as_bool()?;
                if !cond {
                    self.ip = label.0;
                    return Ok(true);
                }
            }
            Instruction::Call(label) => {
                self.ip = label.0;
                return Ok(true);
            }
            Instruction::Return => {
                // Halt execution; the return value stays on the stack.
                return Ok(false);
            }
            Instruction::HostCall(func_id) => {
                let result = self.host.call_host(*func_id, &self.stack)?;
                self.stack.clear();
                self.push(result)?;
            }
        }

        self.ip += 1;
        Ok(true)
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

    fn pop_as_bool(&mut self) -> Result<bool, VmError> {
        let v = self.pop()?;
        v.as_bool().ok_or(VmError::TypeError {
            expected: "bool",
            got: v,
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
    fn arith_float_add() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::F64(1.5)));
        program.add_instruction(Instruction::PushConst(Value::F64(2.0)));
        program.add_instruction(Instruction::Add);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::F64(3.5)]);
    }

    #[test]
    fn arith_mixed_int_float_promotes() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        program.add_instruction(Instruction::PushConst(Value::F64(2.5)));
        program.add_instruction(Instruction::Add);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::F64(3.5)]);
    }

    #[test]
    fn arith_float_div_by_zero_is_infinite() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::F64(1.0)));
        program.add_instruction(Instruction::PushConst(Value::F64(0.0)));
        program.add_instruction(Instruction::Div);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::F64(f64::INFINITY)]);
    }

    #[test]
    fn arith_neg_float() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::F64(2.5)));
        program.add_instruction(Instruction::Neg);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::F64(-2.5)]);
    }

    #[test]
    fn comparison_float_lt() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::F64(1.5)));
        program.add_instruction(Instruction::PushConst(Value::F64(2.5)));
        program.add_instruction(Instruction::Lt);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(true)]);
    }

    #[test]
    fn equality_is_numeric_across_int_and_float() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        program.add_instruction(Instruction::PushConst(Value::F64(1.0)));
        program.add_instruction(Instruction::Eq);

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(true)]);
    }

    #[test]
    fn return_halts_execution_and_keeps_value() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::I64(7)));
        program.add_instruction(Instruction::Return);
        // Would loop forever if `Return` did not halt the VM.
        program.add_instruction(Instruction::PushConst(Value::I64(99)));

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(7)]);
    }

    #[test]
    fn type_error_reports_actual_value() {
        let mut program = Program::new();
        program.add_instruction(Instruction::PushConst(Value::Bool(true)));
        program.add_instruction(Instruction::PushConst(Value::I64(1)));
        program.add_instruction(Instruction::Add);

        let mut vm = Vm::new(TestHost);
        let err = vm.execute(&program).unwrap_err();
        assert!(
            matches!(
                err,
                VmError::TypeError {
                    expected: "number",
                    got: Value::Bool(true)
                }
            ),
            "got {:?}",
            err
        );
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
