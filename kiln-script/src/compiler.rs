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
    #[error("Duplicate variable: {0}")]
    DuplicateVariable(String, Span),
    #[error("Program error: {0}")]
    Program(#[from] kiln_ir::ProgramError),
    #[error("{0} is only valid inside a loop in the same function")]
    InvalidLoopControl(String, Span),
}

struct LoopContext {
    condition: usize,
    breaks: Vec<usize>,
    continues: Option<Vec<usize>>,
}

pub struct Compiler {
    loops: Vec<LoopContext>,
    functions: HashMap<String, (Vec<String>, Vec<Stmt>)>,
    function_stack: Vec<String>,
    function_returns: Vec<Vec<usize>>,
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
            loops: Vec::new(),
            functions: HashMap::new(),
            function_stack: Vec::new(),
            function_returns: Vec::new(),
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
        Self::validate_loop_controls(stmts, 0)?;
        for stmt in stmts {
            if let Stmt::Function { name, params, body } = stmt {
                if self.builtins.contains_key(name)
                    || self
                        .functions
                        .insert(name.clone(), (params.clone(), body.clone()))
                        .is_some()
                {
                    return Err(CompileError::Custom(format!("Duplicate function '{name}'")));
                }
            }
        }
        for stmt in stmts {
            if !matches!(stmt, Stmt::Function { .. }) {
                self.compile_stmt(stmt)?;
            }
        }
        self.program.set_local_slots(self.next_slot);
        self.program.validate_jump_targets()?;
        Ok(self.program.clone())
    }

    fn validate_loop_controls(stmts: &[Stmt], depth: usize) -> Result<(), CompileError> {
        for stmt in stmts {
            match stmt {
                Stmt::Break(span) | Stmt::Continue(span) if depth == 0 => {
                    return Err(CompileError::InvalidLoopControl(
                        if matches!(stmt, Stmt::Break(_)) {
                            "break"
                        } else {
                            "continue"
                        }
                        .into(),
                        *span,
                    ));
                }
                Stmt::While { body, .. } | Stmt::For { body, .. } => {
                    Self::validate_loop_controls(body, depth + 1)?
                }
                Stmt::Function { body, .. } => Self::validate_loop_controls(body, 0)?,
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    Self::validate_loop_controls(then_block, depth)?;
                    if let Some(body) = else_block {
                        Self::validate_loop_controls(body, depth)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        if self.next_slot >= 32000 {
            return Err(CompileError::Custom("Too many script locals".into()));
        }
        match stmt {
            Stmt::Function { .. } => {
                return Err(CompileError::Custom(
                    "Function definitions must be at module top level".into(),
                ));
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.program.add_instruction(Instruction::Pop);
            }
            Stmt::Return(expr) => {
                self.compile_expr(expr)?;
                if let Some(returns) = self.function_returns.last_mut() {
                    let index = self.program.instructions().len();
                    self.program.add_instruction(Instruction::Jump(Label(0)));
                    returns.push(index);
                } else {
                    self.program.add_instruction(Instruction::Return);
                }
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
            Stmt::While { condition, body } => {
                let start = self.program.len();
                self.compile_expr(condition)?;
                let exit = self.program.len();
                self.program
                    .add_instruction(Instruction::JumpIfNot(Label(0)));
                self.loops.push(LoopContext {
                    condition: start,
                    breaks: Vec::new(),
                    continues: None,
                });
                self.scopes.push(HashMap::new());
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.scopes.pop();
                self.program
                    .add_instruction(Instruction::Jump(Label(start)));
                let end = self.program.len();
                self.program
                    .set_instruction(exit, Instruction::JumpIfNot(Label(end)));
                for index in self.loops.pop().expect("Current loop").breaks {
                    self.program
                        .set_instruction(index, Instruction::Jump(Label(end)));
                }
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                self.scopes.push(HashMap::new());
                let Stmt::Let { name, value, .. } = init.as_ref() else {
                    return Err(CompileError::Custom(
                        "for initializer must declare a variable".into(),
                    ));
                };
                self.compile_expr(value)?;
                let slot = self.allocate_slot(name);
                self.program.add_instruction(Instruction::SetLocal(slot));
                let start = self.program.len();
                self.compile_expr(condition)?;
                let exit = self.program.len();
                self.program
                    .add_instruction(Instruction::JumpIfNot(Label(0)));
                self.loops.push(LoopContext {
                    condition: start,
                    breaks: Vec::new(),
                    continues: Some(Vec::new()),
                });
                self.scopes.push(HashMap::new());
                for statement in body {
                    self.compile_stmt(statement)?;
                }
                self.scopes.pop();
                let increment = self.program.len();
                self.compile_stmt(step)?;
                self.program
                    .add_instruction(Instruction::Jump(Label(start)));
                let end = self.program.len();
                self.program
                    .set_instruction(exit, Instruction::JumpIfNot(Label(end)));
                let context = self.loops.pop().expect("Current for loop");
                for index in context.breaks {
                    self.program
                        .set_instruction(index, Instruction::Jump(Label(end)));
                }
                for index in context.continues.unwrap_or_default() {
                    self.program
                        .set_instruction(index, Instruction::Jump(Label(increment)));
                }
                self.scopes.pop();
            }
            Stmt::Break(span) | Stmt::Continue(span) => {
                let is_break = matches!(stmt, Stmt::Break(_));
                let context = self.loops.last_mut().ok_or_else(|| {
                    CompileError::InvalidLoopControl(
                        if is_break { "break" } else { "continue" }.into(),
                        *span,
                    )
                })?;
                let index = self.program.len();
                let target = if is_break {
                    context.breaks.push(index);
                    0
                } else if let Some(continues) = context.continues.as_mut() {
                    continues.push(index);
                    0
                } else {
                    context.condition
                };
                self.program
                    .add_instruction(Instruction::Jump(Label(target)));
                self.program.record_source_offset(index, span.start);
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
        fn offset(expr: &Expr) -> usize {
            match expr {
                Expr::Number(_, span)
                | Expr::Boolean(_, span)
                | Expr::Float(_, span)
                | Expr::String(_, span)
                | Expr::Identifier(_, span)
                | Expr::Call { span, .. } => span.start,
                Expr::BinaryOp { left, .. } | Expr::Logical { left, .. } => offset(left),
                Expr::Not(_, span) => span.start,
            }
        }
        let begin = self.program.instructions().len();
        let source_offset = offset(expr);
        match expr {
            Expr::Number(n, _span) => {
                self.program
                    .add_instruction(Instruction::PushConst(Value::I64(*n)));
            }
            Expr::Boolean(value, _) => {
                self.program
                    .add_instruction(Instruction::PushConst(Value::Bool(*value)));
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
            Expr::Not(operand, _) => {
                self.compile_expr(operand)?;
                let false_branch = self.program.len();
                self.program
                    .add_instruction(Instruction::JumpIfNot(Label(0)));
                self.program
                    .add_instruction(Instruction::PushConst(Value::Bool(false)));
                let end_jump = self.program.len();
                self.program.add_instruction(Instruction::Jump(Label(0)));
                let true_branch = self.program.len();
                self.program
                    .add_instruction(Instruction::PushConst(Value::Bool(true)));
                let end = self.program.len();
                self.program
                    .set_instruction(false_branch, Instruction::JumpIfNot(Label(true_branch)));
                self.program
                    .set_instruction(end_jump, Instruction::Jump(Label(end)));
            }
            Expr::Logical { and, left, right } => {
                let mut branches = Vec::new();
                for operand in [left, right] {
                    self.compile_expr(operand)?;
                    branches.push(self.program.len());
                    self.program.add_instruction(if *and {
                        Instruction::JumpIfNot(Label(0))
                    } else {
                        Instruction::JumpIf(Label(0))
                    });
                }
                self.program
                    .add_instruction(Instruction::PushConst(Value::Bool(*and)));
                let end_jump = self.program.len();
                self.program.add_instruction(Instruction::Jump(Label(0)));
                let decisive = self.program.len();
                self.program
                    .add_instruction(Instruction::PushConst(Value::Bool(!*and)));
                let end = self.program.len();
                for index in branches {
                    self.program.set_instruction(
                        index,
                        if *and {
                            Instruction::JumpIfNot(Label(decisive))
                        } else {
                            Instruction::JumpIf(Label(decisive))
                        },
                    );
                }
                self.program
                    .set_instruction(end_jump, Instruction::Jump(Label(end)));
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
                if let Some((params, body)) = self.functions.get(func).cloned() {
                    if params.len() != args.len() {
                        return Err(CompileError::Custom(format!(
                            "Function '{func}' expects {} arguments, got {}",
                            params.len(),
                            args.len()
                        )));
                    }
                    if self.function_stack.contains(func)
                        || self.function_stack.len() >= 32
                        || self.program.instructions().len() > 65536
                    {
                        return Err(CompileError::Custom(format!(
                            "Recursive or excessively expanded function '{func}'"
                        )));
                    }
                    let caller_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                    let caller_loops = std::mem::take(&mut self.loops);
                    let slots: Vec<_> =
                        params.iter().map(|name| self.allocate_slot(name)).collect();
                    for slot in slots.into_iter().rev() {
                        self.program.add_instruction(Instruction::SetLocal(slot));
                    }
                    self.function_stack.push(func.clone());
                    self.function_returns.push(Vec::new());
                    for stmt in &body {
                        self.compile_stmt(stmt)?;
                    }
                    self.program
                        .add_instruction(Instruction::PushConst(Value::I64(0)));
                    let end = self.program.instructions().len();
                    for index in self.function_returns.pop().unwrap_or_default() {
                        self.program
                            .set_instruction(index, Instruction::Jump(Label(end)));
                    }
                    self.function_stack.pop();
                    self.scopes = caller_scopes;
                    self.loops = caller_loops;
                    return Ok(());
                }
                let func_id = self
                    .builtins
                    .get(func.as_str())
                    .copied()
                    .ok_or_else(|| CompileError::UndefinedFunction(func.clone(), *span))?;
                let expected_argc = self.arities.get(func.as_str()).copied().unwrap_or(0);
                let actual_argc = args.len() as u32;
                if expected_argc != actual_argc {
                    return Err(CompileError::Custom(format!(
                        "Function '{}' expects {} arguments, got {}",
                        func, expected_argc, actual_argc
                    )));
                }
                self.program.add_instruction(Instruction::HostCall {
                    id: func_id,
                    argc: actual_argc,
                });
            }
        }
        for instruction in begin..self.program.instructions().len() {
            self.program
                .record_source_offset(instruction, source_offset);
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
