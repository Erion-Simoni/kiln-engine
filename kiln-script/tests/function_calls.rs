//! Shared function bodies must not duplicate locals or bytecode at every call.
use kiln_ir::{HostContext, Instruction, Program, Value, Vm, VmError};
use kiln_script::{KilnScriptFrontend, LanguageFrontend};
#[derive(Default)]
struct Host;
impl HostContext for Host {
    fn call_host(&mut self, _: u32, _: &[Value]) -> Result<Value, VmError> {
        Ok(Value::I64(0))
    }
}
fn compile(source: &str) -> Program {
    KilnScriptFrontend::new().compile(source).unwrap()
}
#[test]
fn repeated_function_calls_share_one_body_and_parameter_slot() {
    let program = compile("fn bump(value) { return value + 1; } return bump(2) + bump(3);");
    assert_eq!(
        program.local_slots(),
        1,
        "A function parameter must allocate once, not per call site"
    );
    assert_eq!(
        program
            .instructions()
            .iter()
            .filter(|i| matches!(i, Instruction::Call(_)))
            .count(),
        2
    );
    let mut vm = Vm::new(Host);
    vm.execute(&program).unwrap();
    assert_eq!(vm.stack(), &[Value::I64(7)]);
}
#[test]
fn valid_large_reuse_stays_within_compiler_resource_limits() {
    let body = (0..100)
        .map(|n| format!("let value{n} = input + {n};\n"))
        .collect::<String>();
    let calls = (0..400)
        .map(|n| format!("heavy({n});\n"))
        .collect::<String>();
    let program = compile(&format!(
        "fn heavy(input) {{ {body} return input; }}\n{calls}"
    ));
    assert_eq!(program.local_slots(), 101);
    assert!(program.len() < 3000);
    Vm::new(Host).execute(&program).unwrap();
}

#[test]
fn repeated_and_nested_calls_preserve_evaluation_order_returns_and_caller_locals() {
    for (source, expected) in [
        ("fn f(x) { let y=x+1; } return f(3)+7;", 7),
        (
            "fn bump(x) { return x+1; } let skipped=false && bump(1); return bump(2);",
            3,
        ),
        ("fn id(x) { return x; } return id(id(3)) + id(4);", 7),
        (
            "fn f(x) { if x < 1 { return 8; } return x + 1; } let x=20; return f(0)+f(2)+x;",
            31,
        ),
        (
            "fn f(x) { let y=x+1; return y; } fn g(x) { return f(x)+f(x+1); } return g(3)+g(5);",
            22,
        ),
    ] {
        let mut vm = Vm::new(Host);
        vm.execute(&compile(source)).unwrap();
        assert_eq!(vm.stack(), &[Value::I64(expected)], "{source}");
        assert!(vm.call_stack().is_empty());
    }
}
#[test]
fn recursion_and_cached_transitive_depth_are_rejected() {
    for source in [
        "fn f() { return f(); } f();",
        "fn f() { return g(); } fn g() { return f(); } f();",
    ] {
        assert!(KilnScriptFrontend::new().compile(source).unwrap_err()[0]
            .message
            .contains("Recursive"));
    }
    let mut source = "fn f0(x) { return x; }\n".to_owned();
    for n in 1..32 {
        source.push_str(&format!("fn f{n}(x) {{ return f{}(x); }}\n", n - 1));
    }
    source.push_str("f31(1);\n"); // Compiles/cache the entire legal chain first.
    let mut vm = Vm::new(Host);
    vm.execute(&compile(&source)).unwrap();
    source.push_str("fn too_deep(x) { return f31(x); } too_deep(1);");
    assert!(KilnScriptFrontend::new().compile(&source).unwrap_err()[0]
        .message
        .contains("depth"));
}
#[test]
fn real_calls_retain_frames_at_failure() {
    let program = compile(
        "fn inner(x) { return x / 0; } fn outer(x) { return inner(x+1); } return outer(3);",
    );
    let mut vm = Vm::new(Host);
    assert!(matches!(vm.execute(&program), Err(VmError::DivisionByZero)));
    assert_eq!(vm.call_stack().len(), 2);
    assert_eq!(
        program.function_name(vm.instruction_pointer()),
        Some("inner")
    );
    let caller = vm.call_stack()[1] - 1;
    assert_eq!(program.function_name(caller), Some("outer"));
}

#[test]
fn nested_reuse_preserves_argument_side_effect_order() {
    #[derive(Default)]
    struct Trace(Vec<i64>);
    impl HostContext for Trace {
        fn call_host(&mut self, _: u32, args: &[Value]) -> Result<Value, VmError> {
            self.0.push(args[0].as_i64().unwrap());
            Ok(args[0].clone())
        }
    }
    let source = "fn sum(a,b) { return a+b; } return sum(probe(1),sum(probe(2),probe(3)));";
    let program = KilnScriptFrontend::new()
        .with_builtin("probe", 9, 1)
        .compile(source)
        .unwrap();
    let mut vm = Vm::new(Trace::default());
    vm.execute(&program).unwrap();
    assert_eq!(vm.host().0, vec![1, 2, 3]);
    assert_eq!(vm.stack(), &[Value::I64(6)]);
}
