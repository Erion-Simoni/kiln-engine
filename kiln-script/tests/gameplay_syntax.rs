use kiln_ir::{HostContext, Program, Value, Vm, VmError};
use kiln_script::{KilnScriptFrontend, LanguageFrontend};

#[derive(Default)]
struct Host {
    calls: usize,
}
impl HostContext for Host {
    fn call_host(&mut self, _: u32, _: &[Value]) -> Result<Value, VmError> {
        self.calls += 1;
        Ok(Value::Bool(true))
    }
}
fn compile(source: &str) -> Program {
    KilnScriptFrontend::new()
        .with_builtin("probe", 99, 0)
        .compile(source)
        .unwrap()
}
fn run(source: &str) -> Vm<Host> {
    let mut vm = Vm::new(Host::default());
    vm.execute(&compile(source)).unwrap();
    vm
}

#[test]
fn boolean_operators_short_circuit_host_calls_and_have_standard_precedence() {
    let vm = run("let a = false && probe(); let b = true || probe(); let c = true && probe(); let d = false || probe(); return !a && b && c && d || false;");
    assert_eq!(vm.host().calls, 2);
    assert_eq!(vm.stack(), &[Value::Bool(true)]);
    assert_eq!(
        run("return true || false && false;").stack(),
        &[Value::Bool(true)]
    );
    assert_eq!(
        run("return !false == true && 2 < 3;").stack(),
        &[Value::Bool(true)]
    );
    assert_eq!(run("return !!false;").stack(), &[Value::Bool(false)]);
}

#[test]
fn evaluated_boolean_operands_require_bool_but_skipped_operands_are_not_evaluated() {
    for source in [
        "return !1;",
        "return 1 && true;",
        "return false || 2;",
        "while 1 { break; }",
    ] {
        assert!(
            matches!(
                Vm::new(Host::default()).execute(&compile(source)),
                Err(VmError::TypeError { .. })
            ),
            "{source}"
        );
    }
    assert_eq!(run("return false && 1;").stack(), &[Value::Bool(false)]);
}

#[test]
fn compound_assignments_and_unary_arithmetic_preserve_precedence() {
    assert_eq!(
        run("let x = 16; x += 8; x -= 4; x *= 2; x /= 5; return x;").stack(),
        &[Value::I64(8)]
    );
    assert_eq!(
        run("let x = 1.5; x += 2; x *= -2; return x;").stack(),
        &[Value::F64(-7.0)]
    );
    assert_eq!(run("return 8 / 4 / 2;").stack(), &[Value::I64(1)]);
    assert_eq!(run("return 12 / -2 * 3;").stack(), &[Value::I64(-18)]);
    assert_eq!(run("return -2 * -3;").stack(), &[Value::I64(6)]);
    assert!(KilnScriptFrontend::new().compile("missing += 1;").is_err());
}

#[test]
fn while_break_continue_and_else_if_execute_in_order() {
    let source = "let i = 0; let total = 0; while i < 6 { i += 1; if i == 2 { continue; } else if i == 5 { break; } else { total += i; } } return total;";
    assert_eq!(run(source).stack(), &[Value::I64(8)]);
    assert_eq!(run("while false { probe(); } return 4;").host().calls, 0);
}

#[test]
fn nested_loops_use_innermost_targets_and_block_local_scope() {
    let source = "let i = 0; let total = 0; while i < 3 { i += 1; let j = 0; while j < 3 { j += 1; if j == 2 { break; } total += 1; } } return total;";
    assert_eq!(run(source).stack(), &[Value::I64(3)]);
    assert!(KilnScriptFrontend::new()
        .compile("while false { let inside = 1; } return inside;")
        .is_err());
}

#[test]
fn functions_isolate_loop_control_and_restore_caller_loop_targets() {
    let source = "fn next(x) { let i = 0; while i < 3 { i += 1; if i == 2 { continue; } if i == 3 { break; } } return x + 1; } let n = 0; while n < 3 { n = next(n); continue; } return n;";
    assert_eq!(run(source).stack(), &[Value::I64(3)]);
    assert_eq!(
        run("fn choose() { while true { return 7; } return 0; } return choose() + choose();")
            .stack(),
        &[Value::I64(14)]
    );
    for keyword in ["break", "continue"] {
        let source = format!("fn bad() {{ {keyword}; }} while true {{ bad(); break; }}");
        let errors = KilnScriptFrontend::new().compile(&source).unwrap_err();
        assert!(errors[0].message.contains("same function"));
        assert_eq!(errors[0].span.start, source.find(keyword).unwrap());
        let errors = KilnScriptFrontend::new()
            .compile(&format!("\n{keyword};"))
            .unwrap_err();
        assert_eq!(errors[0].span.start, 1);
    }
}

#[test]
fn runaway_loops_terminate_at_the_existing_vm_instruction_budget() {
    let program = compile("while true { continue; }");
    let error = Vm::new(Host::default()).execute(&program).unwrap_err();
    assert!(matches!(error, VmError::InstructionBudgetExceeded));
}

#[test]
fn malformed_operators_and_excessive_nesting_report_errors_without_panicking() {
    for source in [
        "let x = 1; x += ;",
        "true & false",
        "false | true",
        "while true {",
        "if true {} else if {}",
        "@",
        "let x = 1; @",
    ] {
        assert!(
            KilnScriptFrontend::new().compile(source).is_err(),
            "{source}"
        );
    }
    let source = format!("return {}true;", "!".repeat(300));
    assert!(KilnScriptFrontend::new().compile(&source).is_err());
    let source = format!("{}{}", "while true {".repeat(70), "}".repeat(70));
    assert!(KilnScriptFrontend::new().compile(&source).is_err());
    let source = "// unicode: café\nreturn @;";
    let errors = KilnScriptFrontend::new().compile(source).unwrap_err();
    assert_eq!(errors[0].span.start, source.find('@').unwrap());
}

#[test]
fn unused_functions_cannot_hide_invalid_loop_control_and_boolean_chains_are_bounded() {
    assert!(KilnScriptFrontend::new()
        .compile("fn unused() { break; }")
        .is_err());
    assert!(KilnScriptFrontend::new()
        .compile("fn unused() { continue; }")
        .is_err());
    let source = format!("return {}true;", "true && ".repeat(150));
    assert!(KilnScriptFrontend::new().compile(&source).is_err());
}
