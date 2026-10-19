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
pub struct KilnScriptFrontend;

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
        compiler.compile(&stmts).map_err(|e| {
            let span = match &e {
                compiler::CompileError::UndefinedVariable(_, s) => *s,
                compiler::CompileError::UndefinedFunction(_, s) => *s,
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

    struct TestHost;

    impl kiln_ir::HostContext for TestHost {
        fn call_host(
            &mut self,
            func_id: u32,
            args: &[kiln_ir::Value],
        ) -> Result<kiln_ir::Value, kiln_ir::VmError> {
            match func_id {
                0 => Ok(kiln_ir::Value::I64(args.len() as i64)),
                _ => Err(kiln_ir::VmError::HostCallError(
                    "Unknown function".to_string(),
                )),
            }
        }
    }

    #[test]
    fn frontend_compiles_and_runs() {
        let mut frontend = KilnScriptFrontend;
        let program = frontend.compile("1 + 2").unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
    }

    #[test]
    fn frontend_float_arith_runs() {
        let mut frontend = KilnScriptFrontend;
        let program = frontend.compile("return 2.0 * 2.5;").unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::F64(5.0)]);
    }

    #[test]
    fn frontend_mixed_int_float_arith_runs() {
        let mut frontend = KilnScriptFrontend;
        let program = frontend.compile("return 1 + 2.5;").unwrap();

        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
        assert_eq!(vm.stack(), &[kiln_ir::Value::F64(3.5)]);
    }

    #[test]
    fn frontend_parse_error_returns_diagnostic() {
        let mut frontend = KilnScriptFrontend;
        let result = frontend.compile("if {");
        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn frontend_compile_error_returns_diagnostic() {
        let mut frontend = KilnScriptFrontend;
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

        let mut frontend = KilnScriptFrontend;
        let program = frontend.compile(script).unwrap();

        // Verify the program has instructions
        assert!(!program.instructions().is_empty());

        // Run it
        let mut vm = Vm::new(TestHost);
        vm.execute(&program).unwrap();
    }
}
