//! Compiler from KilnScript AST to IR bytecode.

use crate::diagnostic::Span;
use crate::parser::{BinaryOp, Expr, Stmt};
use kiln_ir::{Instruction, Label, Program, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Compile error: {0}")]
    Custom(String),
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String, Span),
    #[error("Undefined function: {0}")]
    UndefinedFunction(String, Span),
}

pub struct Compiler {
    program: Program,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            program: Program::new(),
        }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Result<Program, CompileError> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
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

                for stmt in then_block {
                    self.compile_stmt(stmt)?;
                }

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
                    for stmt in else_block {
                        self.compile_stmt(stmt)?;
                    }
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
            Expr::Identifier(name, span) => {
                return Err(CompileError::UndefinedVariable(name.clone(), *span));
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
                // For now, we use fixed host call IDs
                let func_id = match func.as_str() {
                    "print" => 0,
                    _ => return Err(CompileError::UndefinedFunction(func.clone(), *span)),
                };
                self.program.add_instruction(Instruction::HostCall(func_id));
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
}
