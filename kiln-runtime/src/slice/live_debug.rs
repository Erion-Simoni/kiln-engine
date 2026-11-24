//! Bounded read-only inspection of the VMs that actually execute gameplay.
use super::{SliceSession, Value};

/// One allocated VM local, not necessarily in an active lexical scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveVmLocal {
    /// Stable allocated slot; disambiguates shadowed/inlined names.
    pub slot: u16,
    /// Compiler name, or a temporary-slot label when unavailable.
    pub name: String,
    /// Owning compiled function, or None for entry-program locals.
    pub function: Option<String>,
    /// Bounded preview; containers show size rather than recursively copying.
    pub value: String,
}

/// One actual runtime call frame, ordered from entry toward current execution.
#[derive(Debug, Clone)]
pub struct LiveVmFrame {
    /// Compiled function name, or <entry> for scene/attachment entry code.
    pub function: String,
    /// Current instruction or caller call-site instruction.
    pub instruction: usize,
    /// Mapped source origin where available.
    pub location: Option<String>,
}

/// Read-only snapshot of a scene or attached gameplay VM.
#[derive(Debug, Clone)]
pub struct LiveVmDebug {
    /// Scene marker or stable attachment entity ID.
    pub owner: String,
    /// Next/failing bytecode instruction.
    pub instruction: usize,
    /// Source origin where available; absent at an unmapped/completed position.
    pub location: Option<String>,
    /// Actual call frames (VM call depth is bounded to 32).
    pub frames: Vec<LiveVmFrame>,
    /// First 128 allocated locals, ordered by slot.
    pub locals: Vec<LiveVmLocal>,
    /// Allocated locals omitted by the bound.
    pub omitted_locals: usize,
}

fn source_location(session: &SliceSession, instruction: usize) -> Option<String> {
    session
        .program
        .source_offset(instruction)
        .and_then(|offset| {
            session.source.as_ref().map(|source| {
                source.diagnostic(&kiln_script::Diagnostic::new(
                    kiln_script::Span::new(offset, offset),
                    "VM position",
                ))
            })
        })
}

fn preview(value: &Value) -> String {
    match value {
        Value::Str(value) => {
            let prefix: String = value.chars().take(128).collect();
            let suffix = if prefix.len() < value.len() {
                "…"
            } else {
                ""
            };
            format!("\"{}{suffix}\"", prefix.escape_debug())
        }
        Value::Array(values) => format!("Array ({} items)", values.len()),
        Value::Map(values) => format!("Dictionary ({} entries)", values.len()),
        _ => format!("{value:?}"),
    }
}

impl SliceSession {
    /// Inspect actual executing VMs without executing instructions or host calls.
    /// At most 64 sessions and 128 locals per session are materialized.
    pub fn live_vm_debug(&self) -> Vec<LiveVmDebug> {
        std::iter::once(("<scene>", self))
            .chain(
                self.attached
                    .iter()
                    .map(|attached| (attached.entity_id.as_str(), &attached.session)),
            )
            .take(64)
            .map(|(owner, session)| {
                let instruction = session.vm.instruction_pointer();
                let location = source_location(session, instruction);
                let frames = session
                    .vm
                    .call_stack()
                    .iter()
                    .map(|return_ip| return_ip.saturating_sub(1))
                    .chain(std::iter::once(instruction))
                    .map(|ip| LiveVmFrame {
                        function: session
                            .program
                            .function_name(ip)
                            .unwrap_or("<entry>")
                            .chars()
                            .take(128)
                            .collect(),
                        instruction: ip,
                        location: source_location(session, ip),
                    })
                    .collect();
                let locals = session
                    .vm
                    .locals()
                    .iter()
                    .take(128)
                    .enumerate()
                    .map(|(slot, value)| LiveVmLocal {
                        slot: slot as u16,
                        function: session
                            .program
                            .local_function(slot as u16)
                            .map(|name| name.chars().take(128).collect()),
                        name: session
                            .program
                            .local_name(slot as u16)
                            .unwrap_or("<temporary>")
                            .chars()
                            .take(128)
                            .collect(),
                        value: preview(value),
                    })
                    .collect();
                LiveVmDebug {
                    owner: owner.chars().take(128).collect(),
                    instruction,
                    location,
                    frames,
                    locals,
                    omitted_locals: session.vm.locals().len().saturating_sub(128),
                }
            })
            .collect()
    }
}

fn segment(value: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '~' {
            decoded.push(match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                _ => return Err("Use ~0 for tilde and ~1 for slash in watch keys".into()),
            });
        } else {
            decoded.push(character);
        }
    }
    Ok(decoded)
}

impl SliceSession {
    /// Read a bounded data selector or isolated read-only expr: expression.
    /// Owner is `scene` or `entity:ID`; roots are slot:N, local:NAME, state:KEY, expr:.
    pub fn live_watch(&self, owner: &str, selector: &str) -> Result<String, String> {
        if owner.len() > 256 || selector.len() > 512 {
            return Err("Watch exceeds owner/selector/path limit".into());
        }
        let session = if owner == "scene" {
            self
        } else if let Some(id) = owner.strip_prefix("entity:") {
            let mut matches = self
                .attached
                .iter()
                .filter(|attached| attached.entity_id == id);
            match (matches.next(), matches.next()) {
                (Some(attached), None) => &attached.session,
                (Some(_), Some(_)) => return Err("Entity VM owner is ambiguous".into()),
                _ => return Err("Entity VM owner was not found".into()),
            }
        } else {
            return Err("Owner must be scene or entity:ID".into());
        };
        if let Some(expression) = selector.strip_prefix("expr:") {
            return super::watch_expression::evaluate(session, self, expression)
                .map(|value| preview(&value));
        }
        if selector.split('/').count() > 16 {
            return Err("Watch exceeds owner/selector/path limit".into());
        }
        let mut path = selector.split('/');
        let (kind, key) = path
            .next()
            .unwrap_or_default()
            .split_once(':')
            .ok_or("Use slot:N, local:NAME, state:KEY or expr:expression")?;
        let key = segment(key)?;
        let mut value = match kind {
            "slot" => session
                .vm
                .locals()
                .get(
                    key.parse::<usize>()
                        .map_err(|_| "Slot must be an unsigned index")?,
                )
                .ok_or("Local slot was not found")?,
            "local" => {
                let mut matches = session.vm.locals().iter().enumerate().filter(|(slot, _)| {
                    session.program.local_name(*slot as u16) == Some(key.as_str())
                });
                match (matches.next(), matches.next()) {
                    (Some((_, value)), None) => value,
                    (Some(_), Some(_)) => {
                        return Err("Allocated local name is ambiguous; use slot:N".into())
                    }
                    _ => return Err("Allocated local name was not found".into()),
                }
            }
            "state" => session
                .vm
                .host()
                .script_state
                .get(&key)
                .ok_or("State key was not found")?,
            _ => return Err("Use slot:N, local:NAME, state:KEY or expr:expression".into()),
        };
        for key in path {
            let key = segment(key)?;
            value = match value {
                Value::Array(values) => values
                    .get(
                        key.parse::<usize>()
                            .map_err(|_| "Array index must be unsigned")?,
                    )
                    .ok_or("Array index is out of range")?,
                Value::Map(values) => values.get(&key).ok_or("Dictionary key was not found")?,
                _ => return Err("Watch path cannot descend into a scalar".into()),
            };
        }
        Ok(preview(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previews_bound_strings_and_never_walk_container_contents() {
        let text = preview(&Value::Str("界".repeat(10000)));
        assert!(text.ends_with("…\""));
        assert!(text.len() < 400);
        assert_eq!(
            preview(&Value::Array(vec![Value::Str("large".repeat(10000))])),
            "Array (1 items)"
        );
        assert_eq!(
            preview(&Value::Map(Default::default())),
            "Dictionary (0 entries)"
        );
    }
}
