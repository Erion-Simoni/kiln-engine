//! Bounded read-only inspection of the VMs that actually execute gameplay.
use super::{SliceSession, Value};

/// One allocated VM local, not necessarily in an active lexical scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveVmLocal {
    /// Stable allocated slot; disambiguates shadowed/inlined names.
    pub slot: u16,
    /// Compiler name, or a temporary-slot label when unavailable.
    pub name: String,
    /// Bounded preview; containers show size rather than recursively copying.
    pub value: String,
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
    /// First 128 allocated locals, ordered by slot.
    pub locals: Vec<LiveVmLocal>,
    /// Allocated locals omitted by the bound.
    pub omitted_locals: usize,
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
                let location = session
                    .program
                    .source_offset(instruction)
                    .and_then(|offset| {
                        session.source.as_ref().map(|source| {
                            source.diagnostic(&kiln_script::Diagnostic::new(
                                kiln_script::Span::new(offset, offset),
                                "VM position",
                            ))
                        })
                    });
                let locals = session
                    .vm
                    .locals()
                    .iter()
                    .take(128)
                    .enumerate()
                    .map(|(slot, value)| LiveVmLocal {
                        slot: slot as u16,
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
                    locals,
                    omitted_locals: session.vm.locals().len().saturating_sub(128),
                }
            })
            .collect()
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
