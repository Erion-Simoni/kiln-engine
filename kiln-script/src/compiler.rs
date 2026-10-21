//! Compiler from KilnScript AST to IR bytecode.

use crate::diagnostic::Span;
use crate::parser::{BinaryOp, Expr, Stmt};
use kiln_ir::{Instruction, Label, Program, Value};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Compile error: {0}")]
    Custom(String),
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String, Span),
    #[error("Undefined function: {0}")]
    UndefinedFunction(String, Span),
    #[error("Variable already declared: {0}")]
    DuplicateVariable(String, Span),
}

pub struct Compiler {
    program: Program,
    /// Name → host-call id for script `name(...)` calls.
    builtins: HashMap<String, u32>,
    /// Name → argument count for script `name(...)` calls.
    arities: HashMap<String, u32>,
    /// Variable scopes, innermost last. Each maps a name to its local slot.
    scopes: Vec<HashMap<String, u16>>,
    /// Number of local slots allocated so far.
    next_slot: u16,
}

impl Compiler {
    pub fn new() -> Self {
        let mut builtins = HashMap::new();
        builtins.insert("print".to_string(), 0u32);
        let mut arities = HashMap::new();
        arities.insert("print".to_string(), 1u32);
        Self {
            program: Program::new(),
            builtins,
            arities,
            scopes: vec![HashMap::new()],
            next_slot: 0,
        }
    }

    /// Allocate a local slot for `name` in the innermost scope.
    fn allocate_slot(&mut self, name: &str) -> u16 {
        let slot = self.next_slot;
        self.next_slot += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), slot);
        }
        slot
    }

    /// Find the local slot of `name`, searching scopes innermost first.
    fn find_slot(&self, name: &str) -> Option<u16> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    /// Register a host function name callable from scripts.
    pub fn register_builtin(&mut self, name: &str, host_id: u32) {
        self.builtins.insert(name.to_string(), host_id);
    }

    /// Register a host function with the number of arguments it takes.
    pub fn register_builtin_with_arity(&mut self, name: &str, host_id: u32, argc: u32) {
        self.builtins.insert(name.to_string(), host_id);
        self.arities.insert(name.to_string(), argc);
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Result<Program, CompileError> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
        self.program.set_local_slots(self.next_slot);
        Ok(self.program.clone())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.program.add_instruction(Instruction::Pop);
            }
            Stmt::Return(expr) => {
                self.compile_expr(expr)?;
                self.program.add_instruction(Instruction::Return);
            }
            Stmt::Let { name, value, span } => {
                if self.scopes.iter().any(|scope| scope.contains_key(name)) {
                    return Err(CompileError::DuplicateVariable(name.clone(), *span));
                }
                self.compile_expr(value)?;
                let slot = self.allocate_slot(name);
                self.program.add_instruction(Instruction::SetLocal(slot));
            }
            Stmt::Assign { name, value, span } => {
                let slot = self
                    .find_slot(name)
                    .ok_or_else(|| CompileError::UndefinedVariable(name.clone(), *span))?;
                self.compile_expr(value)?;
                self.program.add_instruction(Instruction::SetLocal(slot));
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_expr(condition)?;

                // Emit JumpIfNot with placeholder; we'll fix it after compiling the then block
                let jump_if_not_idx = self.program.len();
                self.program
                    .add_instruction(Instruction::JumpIfNot(Label(0)));

                // The then block gets its own scope.
                self.scopes.push(HashMap::new());
                for stmt in then_block {
                    self.compile_stmt(stmt)?;
                }
                self.scopes.pop();

                // After then block, if there's an else block, jump over it
                let jump_end_idx = if else_block.is_some() {
                    let idx = self.program.len();
                    self.program.add_instruction(Instruction::Jump(Label(0)));
                    Some(idx)
                } else {
                    None
                };

                // Else block starts here
                let else_idx = self.program.len();

                if let Some(else_block) = else_block {
                    // The else block gets its own scope too.
                    self.scopes.push(HashMap::new());
                    for stmt in else_block {
                        self.compile_stmt(stmt)?;
                    }
                    self.scopes.pop();
                }
                // End of if statement
                let end_idx = self.program.len();

                // Now fix up the jumps
                // Replace the JumpIfNot target with the else index
                self.program
                    .set_instruction(jump_if_not_idx, Instruction::JumpIfNot(Label(else_idx)));

                // Replace the Jump over else with the end index
                if let Some(jump_end_idx) = jump_end_idx {
                    self.program
                        .set_instruction(jump_end_idx, Instruction::Jump(Label(end_idx)));
                }
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr {
            Expr::Number(n, _span) => {
                self.program
                    .add_instruction(Instruction::PushConst(Value::I64(*n)));
            }
            Expr::Float(f, _span) => {
                self.program
                    .add_instruction(Instruction::PushConst(Value::F64(*f)));
            }
            Expr::String(s, _span) => {
                self.program
                    .add_instruction(Instruction::PushConst(Value::Str(s.clone())));
            }
            Expr::Identifier(name, span) => {
                let slot = self
                    .find_slot(name)
                    .ok_or_else(|| CompileError::UndefinedVariable(name.clone(), *span))?;
                self.program.add_instruction(Instruction::GetLocal(slot));
            }
            Expr::BinaryOp { op, left, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                self.program.add_instruction(match op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::Ne => Instruction::Ne,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Le => Instruction::Le,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::Ge => Instruction::Ge,
                });
            }
            Expr::Call { func, args, span } => {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                let func_id = self
                    .builtins
                    .get(func.as_str())
                    .copied()
                    .ok_or_else(|| CompileError::UndefinedFunction(func.clone(), *span))?;
                let argc = self
                    .arities
                    .get(func.as_str())
                    .copied()
                    .unwrap_or(args.len() as u32);
                self.program
                    .add_instruction(Instruction::HostCall { id: func_id, argc });
            }
        }
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_ir::Vm;

    struct TestHost;

    impl kiln_ir::HostContext for TestHost {
        fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, kiln_ir::VmError> {
            match func_id {
                0 => Ok(Value::I64(args.len() as i64)),
                _ => Err(kiln_ir::VmError::HostCallError(
                    "Unknown function".to_string(),
                )),
            }
        }
    }

    #[test]
    fn compile_and_run_arith() {
        let mut parser = crate::parser::Parser::new("1 + 2");
        let stmts = parser.parse().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(&stmts).unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
    }

    #[test]
    fn compile_and_run_call() {
        let mut parser = crate::parser::Parser::new("print(42)");
        let stmts = parser.parse().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(&stmts).unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
    }

    #[test]
    fn compile_undefined_variable() {
        let mut parser = crate::parser::Parser::new("unknown_var");
        let stmts = parser.parse().unwrap();

        let mut compiler = Compiler::new();
        let result = compiler.compile(&stmts);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompileError::UndefinedVariable(_, span) => {
                assert!(span.length() > 0);
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }

    #[test]
    fn compile_and_run_string_return() {
        let mut parser = crate::parser::Parser::new("return \"hi\";");
        let stmts = parser.parse().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(&stmts).unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Str("hi".to_string())]);
    }

    #[test]
    fn compile_and_run_string_equality() {
        let mut parser = crate::parser::Parser::new("return \"ab\" == \"ab\";");
        let stmts = parser.parse().unwrap();

        let mut compiler = Compiler::new();
        let program = compiler.compile(&stmts).unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[Value::Bool(true)]);
    }
}
