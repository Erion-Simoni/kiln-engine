//! Bounded gameplay graph lowering through the ordinary KilnScript frontend.
use crate::{Graph, PinType};
use kiln_ir::Program;
use kiln_script::{KilnScriptFrontend, LanguageFrontend};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Graph error: {0}")]
    Invalid(String),
}
fn invalid(message: impl Into<String>) -> CompilerError {
    CompilerError::Invalid(message.into())
}

/// Numeric pins interoperate; Value accepts any ordinary VM value, never events.
pub fn compatible(source: PinType, target: PinType) -> bool {
    source == target
        || (source != PinType::Event
            && target != PinType::Event
            && (source == PinType::Value
                || target == PinType::Value
                || matches!(
                    (source, target),
                    (PinType::Integer, PinType::Float) | (PinType::Float, PinType::Integer)
                )))
}

/// Compile using the exact host names, IDs, arities and frontend used by gameplay.
pub fn compile_graph(graph: &Graph) -> Result<Program, CompilerError> {
    let source = graph_source(graph)?;
    let mut frontend = KilnScriptFrontend::new();
    for (name, id, argc) in kiln_script::BUILTINS {
        frontend = frontend.with_builtin(*name, *id, *argc);
    }
    frontend.compile(&source).map_err(|diagnostics| {
        invalid(
            diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })
}

mod demand;
pub(crate) mod legacy;

/// Generate bounded ordinary source, evaluating each demanded node once.
pub fn graph_source(graph: &Graph) -> Result<String, CompilerError> {
    demand::graph_source(graph)
}
