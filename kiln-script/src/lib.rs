//! KilnScript — minimal scripting language for Kiln.

pub mod compiler;
pub mod diagnostic;
pub mod lexer;
pub mod parser;

pub use compiler::Compiler;
pub use diagnostic::{Diagnostic, Span};

use kiln_ir::Program;

// Note: Tween functions are still deferred. The VM now supports `Value::F64`
// (added with F4), but the tween *language surface* (lerp/ease builtins) has
// not been wired into the script frontend yet.

/// Trait for language frontends that compile to Kiln IR.
pub trait LanguageFrontend {
    /// Parse source code and return a program with diagnostics.
    fn compile(&mut self, source: &str) -> Result<Program, Vec<Diagnostic>>;
}

/// KilnScript frontend implementation.
#[derive(Default)]
pub struct KilnScriptFrontend {
    /// Host functions callable from scripts: name → (host id, argument count).
    builtins: Vec<(String, u32, u32)>,
}

impl KilnScriptFrontend {
    /// A frontend with no extra host functions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a host function callable from scripts.
    ///
    /// `host_id` is passed to [`kiln_ir::HostContext::call_host`] and `argc` is
    /// the number of arguments the function takes.
    pub fn with_builtin(mut self, name: impl Into<String>, host_id: u32, argc: u32) -> Self {
        self.builtins.push((name.into(), host_id, argc));
        self
    }
}

impl LanguageFrontend for KilnScriptFrontend {
    fn compile(&mut self, source: &str) -> Result<Program, Vec<Diagnostic>> {
        let mut parser = parser::Parser::new(source);
        let stmts = match parser.parse() {
            Ok(stmts) => stmts,
            Err(e) => {
                let mut diagnostics = parser.take_diagnostics();
                if diagnostics.is_empty() {
                    diagnostics.push(Diagnostic::new(
                        Span::new(0, source.len()),
                        format!("Parse error: {}", e),
                    ));
                }
                return Err(diagnostics);
            }
        };

        let mut compiler = Compiler::new();
        for (name, host_id, argc) in &self.builtins {
            compiler.register_builtin_with_arity(name, *host_id, *argc);
        }
        compiler.compile(&stmts).map_err(|e| {
            let span = match &e {
                compiler::CompileError::UndefinedVariable(_, s) => *s,
                compiler::CompileError::UndefinedFunction(_, s) => *s,
                compiler::CompileError::DuplicateVariable(_, s) => *s,
                compiler::CompileError::Program(_) => Span::new(0, source.len()),
                compiler::CompileError::Custom(_) => Span::new(0, source.len()),
            };
            vec![Diagnostic::new(span, format!("Compile error: {}", e))]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_ir::Vm;

    struct TestHost {
        /// Score accumulated by the `add_score` builtin.
        score: i64,
    }

    impl TestHost {
        fn new() -> Self {
            Self { score: 0 }
        }
    }

    impl kiln_ir::HostContext for TestHost {
        fn call_host(
            &mut self,
            func_id: u32,
            args: &[kiln_ir::Value],
        ) -> Result<kiln_ir::Value, kiln_ir::VmError> {
            match func_id {
                0 => Ok(kiln_ir::Value::I64(args.len() as i64)),
                7 => {
                    let amount = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                    self.score += amount;
                    Ok(kiln_ir::Value::I64(self.score))
                }
                _ => Err(kiln_ir::VmError::HostCallError(
                    "Unknown function".to_string(),
                )),
            }
        }
    }

    #[test]
    fn frontend_compiles_and_runs() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile("1 + 2").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
    }

    #[test]
    fn frontend_float_arith_runs() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile("return 2.0 * 2.5;").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::F64(5.0)]);
    }

    #[test]
    fn frontend_mixed_int_float_arith_runs() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile("return 1 + 2.5;").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::F64(3.5)]);
    }

    #[test]
    fn frontend_parse_error_returns_diagnostic() {
        let mut frontend = KilnScriptFrontend::new();
        let result = frontend.compile("if {");
        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn frontend_compile_error_returns_diagnostic() {
        let mut frontend = KilnScriptFrontend::new();
        let result = frontend.compile("unknown_var");
        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
        // The diagnostic should have a span pointing to the variable name
        assert!(diagnostics[0].span.length() > 0);
    }

    #[test]
    fn sample_script_compiles() {
        let script = r#"
            if 1 + 2 == 3 {
                print(42);
            }
        "#;

        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile(script).unwrap();

        // Verify the program has instructions
        assert!(!program.instructions().is_empty());

        // Run it
        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
    }

    #[test]
    fn custom_builtin_is_callable() {
        let mut frontend = KilnScriptFrontend::new().with_builtin("add_score", 7, 1);
        let program = frontend.compile("add_score(5);").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.host().score, 5);
    }

    #[test]
    fn unknown_function_is_rejected() {
        let mut frontend = KilnScriptFrontend::new();
        let result = frontend.compile("not_a_builtin(1);");
        assert!(result.is_err());
    }

    #[test]
    fn let_binds_a_local_variable() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile("let x = 7; return x;").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::I64(7)]);
    }

    #[test]
    fn assignment_updates_a_local_variable() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend
            .compile("let x = 1; x = x + 41; return x;")
            .unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::I64(42)]);
    }

    #[test]
    fn variables_work_inside_if_blocks() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend
            .compile("let x = 0; if 1 == 1 { let y = 5; x = y; } return x;")
            .unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::I64(5)]);
    }

    #[test]
    fn undefined_variable_is_rejected() {
        let mut frontend = KilnScriptFrontend::new();
        let result = frontend.compile("return missing;");
        assert!(result.is_err());
    }

    #[test]
    fn redeclaring_a_variable_is_rejected() {
        let mut frontend = KilnScriptFrontend::new();
        let result = frontend.compile("let x = 1; let x = 2;");
        assert!(result.is_err());
    }

    #[test]
    fn unary_minus_negates_a_number() {
        let mut frontend = KilnScriptFrontend::new();
        let program = frontend.compile("return -3;").unwrap();

        let mut vm = Vm::new(TestHost::new());
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::I64(-3)]);
    }
}

#[cfg(test)]
mod call_tests {
    use super::*;
    use kiln_ir::{HostContext, Value, Vm, VmError};

    /// A host that records the arguments of every call.
    #[derive(Default)]
    struct RecordingHost {
        calls: Vec<(u32, Vec<Value>)>,
    }

    impl HostContext for RecordingHost {
        fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError> {
            self.calls.push((func_id, args.to_vec()));
            // The result marks which call produced it, as a number so it can
            // be compared.
            Ok(Value::I64(func_id as i64))
        }
    }

    /// Compile a script with the given builtins registered.
    fn compile(
        source: &str,
        builtins: &[(&str, u32, u32)],
    ) -> Result<kiln_ir::Program, Vec<Diagnostic>> {
        let mut frontend = KilnScriptFrontend::new();
        for (name, id, argc) in builtins {
            frontend = frontend.with_builtin(*name, *id, *argc);
        }
        frontend.compile(source)
    }

    #[test]
    fn a_call_receives_only_its_own_arguments() {
        let program = compile("add(1, 2);", &[("add", 1, 2)]).unwrap();
        let mut vm = Vm::new(RecordingHost::default());
        vm.execute(&program).unwrap();

        let host = vm.host();
        assert_eq!(host.calls.len(), 1);
        assert_eq!(host.calls[0].0, 1);
        assert_eq!(
            host.calls[0].1,
            vec![Value::I64(1), Value::I64(2)],
            "the arguments keep their order"
        );
    }

    #[test]
    fn nested_calls_pass_their_own_arguments() {
        // `add(mul(2, 3), 4)` must give `add` the result of `mul` and 4,
        // not the result of `mul` twice.
        let program = compile("add(mul(2, 3), 4);", &[("add", 1, 2), ("mul", 2, 2)]).unwrap();
        let mut vm = Vm::new(RecordingHost::default());
        vm.execute(&program).unwrap();

        let host = vm.host();
        assert_eq!(host.calls.len(), 2);
        assert_eq!(host.calls[0].0, 2, "the inner call runs first");
        assert_eq!(host.calls[0].1, vec![Value::I64(2), Value::I64(3)]);
        assert_eq!(host.calls[1].0, 1);
        assert_eq!(
            host.calls[1].1,
            vec![Value::I64(2), Value::I64(4)],
            "the outer call gets the inner result, not the inner arguments"
        );
    }

    #[test]
    fn a_call_result_can_be_compared() {
        // The result of a call stays on the stack, so it can be compared.
        let program = compile(
            "if score() > 3 { add(1, 0); }",
            &[("score", 1, 0), ("add", 2, 2)],
        )
        .unwrap();
        let mut vm = Vm::new(RecordingHost::default());
        vm.execute(&program).unwrap();

        let host = vm.host();
        assert_eq!(host.calls[0].0, 1, "the score is read");
        // The host returns 1 for every call, so 1 > 3 is false and the body
        // is skipped.
        assert_eq!(host.calls.len(), 1);
    }
}
