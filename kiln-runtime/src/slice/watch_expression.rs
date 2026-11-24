//! Evaluate bounded expressions in a separate VM with no gameplay host access.
use super::{value_reads, SliceSession};
use kiln_ir::{HostContext, Value, Vm, VmError};
use kiln_script::parser::{Expr, Parser, Stmt};

const READS: &[(&str, u32, u32)] = &[
    ("slot", 1000, 1),
    ("local", 1001, 1),
    ("state", 1002, 1),
    ("game", 1003, 1),
    ("array_length", 61, 1),
    ("array_get", 62, 2),
    ("dict_get", 111, 3),
    ("dict_has", 113, 2),
];
fn rewrite(expr: &mut Expr, depth: usize, nodes: &mut usize) -> Result<(), String> {
    *nodes += 1;
    if depth > 32 || *nodes > 128 {
        return Err("Watch expression exceeds depth/node limit".into());
    }
    match expr {
        Expr::Identifier(name, span) => {
            *expr = Expr::Call {
                func: "local".into(),
                args: vec![Expr::String(name.clone(), *span)],
                span: *span,
            };
        }
        Expr::Not(value, _) => rewrite(value, depth + 1, nodes)?,
        Expr::Logical { left, right, .. } | Expr::BinaryOp { left, right, .. } => {
            rewrite(left, depth + 1, nodes)?;
            rewrite(right, depth + 1, nodes)?;
        }
        Expr::Call { func, args, .. } => {
            if !READS.iter().any(|(name, _, _)| name == func) {
                return Err(format!("Watch call {func} is not read-only or supported"));
            }
            for arg in args {
                rewrite(arg, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}
#[derive(Default)]
struct CopyBudget {
    nodes: usize,
    bytes: usize,
}
impl CopyBudget {
    fn bytes(&mut self, bytes: usize) -> Result<(), String> {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > 65536 {
            return Err("Watch value exceeds byte limit".into());
        }
        Ok(())
    }
    fn copy(&mut self, value: &Value, depth: usize) -> Result<Value, String> {
        self.nodes += 1;
        if depth > 16 || self.nodes > 4096 {
            return Err("Watch value exceeds depth/node limit".into());
        }
        self.bytes(8)?;
        Ok(match value {
            Value::Str(text) => {
                self.bytes(text.len())?;
                Value::Str(text.clone())
            }
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|v| self.copy(v, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Map(values) => {
                let mut result = std::collections::BTreeMap::new();
                for (key, value) in values {
                    self.bytes(key.len())?;
                    result.insert(key.clone(), self.copy(value, depth + 1)?);
                }
                Value::Map(result)
            }
            _ => value.clone(),
        })
    }
}
struct ReadHost<'a> {
    session: &'a SliceSession,
    game: &'a SliceSession,
    budget: CopyBudget,
}
impl HostContext for ReadHost<'_> {
    fn call_host(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        let read = || -> Result<&Value, String> {
            if id == 1000 {
                let index = args[0]
                    .as_i64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or("Watch slot expects a nonnegative integer")?;
                return self
                    .session
                    .vm
                    .locals()
                    .get(index)
                    .ok_or("Local slot was not found".into());
            }
            let name = args[0].as_str().ok_or("Watch read expects a string name")?;
            match id {
                1001 => {
                    let mut found =
                        self.session
                            .vm
                            .locals()
                            .iter()
                            .enumerate()
                            .filter(|(slot, _)| {
                                self.session.program.local_name(*slot as u16) == Some(name)
                            });
                    match (found.next(), found.next()) {
                        (Some((_, value)), None) => Ok(value),
                        (Some(_), Some(_)) => {
                            Err("Allocated local name is ambiguous; use slot(index)".into())
                        }
                        _ => Err("Allocated local name was not found".into()),
                    }
                }
                1002 => self
                    .session
                    .vm
                    .host()
                    .script_state
                    .get(name)
                    .ok_or("State key was not found".into()),
                1003 => self
                    .game
                    .vm
                    .host()
                    .game_data
                    .get(name)
                    .ok_or("Game key was not found".into()),
                _ => Err("Unknown watch read".into()),
            }
        };
        let result = if (1000..=1003).contains(&id) {
            read().and_then(|v| self.budget.copy(v, 0))
        } else {
            value_reads::call(id, args)
                .map_err(|e| e.to_string())
                .and_then(|v| self.budget.copy(&v, 0))
        };
        result.map_err(VmError::HostCallError)
    }
}
pub(super) fn evaluate(
    session: &SliceSession,
    game: &SliceSession,
    expression: &str,
) -> Result<Value, String> {
    if expression.is_empty() || expression.len() > 512 {
        return Err("Watch expression must contain 1..512 bytes".into());
    }
    let source = format!("return {expression};");
    let mut parser = Parser::new(&source);
    let mut statements = parser.parse().map_err(|e| e.to_string())?;
    let [Stmt::Return(expr)] = statements.as_mut_slice() else {
        return Err("Watch accepts a single expression, not statements".into());
    };
    rewrite(expr, 0, &mut 0)?;
    let mut compiler = kiln_script::Compiler::new();
    for (name, id, argc) in READS {
        compiler.register_builtin_with_arity(name, *id, *argc);
    }
    let program = compiler.compile(&statements).map_err(|e| e.to_string())?;
    if program.len() > 4096 {
        return Err("Watch exceeds instruction limit".into());
    }
    let mut vm = Vm::new(ReadHost {
        session,
        game,
        budget: CopyBudget::default(),
    });
    vm.set_instruction_budget(4096);
    vm.execute(&program).map_err(|e| e.to_string())?;
    vm.stack()
        .last()
        .cloned()
        .ok_or("Watch produced no value".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copy_budget_rejects_broad_and_deep_values_before_unbounded_cloning() {
        let wide = Value::Array(vec![Value::I64(1); 4096]);
        assert!(CopyBudget::default()
            .copy(&wide, 0)
            .unwrap_err()
            .contains("node"));
        let mut deep = Value::I64(1);
        for _ in 0..18 {
            deep = Value::Array(vec![deep]);
        }
        assert!(CopyBudget::default()
            .copy(&deep, 0)
            .unwrap_err()
            .contains("depth"));
    }
}
