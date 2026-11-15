//! Single-file visual metadata plus ordinary generated gameplay source.
use crate::{
    compiler::{graph_source, CompilerError},
    Connection, Graph, Node,
};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;
const HEADER: &str = "// kiln-graph-v1\n";

/// Save both visual authoring data and executable text atomically as one .ks file.
pub fn encode(graph: &Graph) -> Result<String, CompilerError> {
    document_bounds(graph).map_err(CompilerError::Invalid)?;
    #[derive(Serialize)]
    struct Ordered<'a> {
        nodes: BTreeMap<Uuid, &'a Node>,
        connections: Vec<&'a Connection>,
    }
    let mut connections = graph.connections.iter().collect::<Vec<_>>();
    connections.sort_by_key(|connection| connection.id);
    let metadata = ron::to_string(&Ordered {
        nodes: graph.nodes.iter().map(|(id, node)| (*id, node)).collect(),
        connections,
    })
    .map_err(|error| CompilerError::Invalid(error.to_string()))?;
    let body = draft_body(graph);
    if metadata.len() * 2 + metadata.len().div_ceil(60) * 4 + HEADER.len() + body.len() + 17
        > 1_048_576
    {
        return Err(CompilerError::Invalid(
            "Graph document exceeds 1 MiB".into(),
        ));
    }
    use std::fmt::Write;
    let mut source = String::from(HEADER);
    for chunk in metadata.as_bytes().chunks(60) {
        source.push_str("// ");
        for byte in chunk {
            write!(source, "{byte:02x}").expect("String write");
        }
        source.push('\n');
    }
    source.push_str("// kiln-graph-end\n");
    source.push_str(&body);
    Ok(source)
}

/// Return None for ordinary text. Preserve manually edited generated bodies as text.
pub fn decode(source: &str) -> Result<Option<Graph>, String> {
    let Some(rest) = source.strip_prefix(HEADER) else {
        return Ok(None);
    };
    if source.len() > 1_048_576 {
        return Err("Graph document exceeds 1 MiB".into());
    }
    let (metadata, body) = rest
        .split_once("// kiln-graph-end\n")
        .ok_or("Graph body is missing")?;
    let metadata = metadata
        .lines()
        .map(|line| {
            line.strip_prefix("// ")
                .ok_or("Invalid visual graph header")
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    if metadata.len() % 2 != 0 || !metadata.is_ascii() {
        return Err("Invalid visual graph header".into());
    }
    let bytes = (0..metadata.len())
        .step_by(2)
        .map(|n| {
            u8::from_str_radix(&metadata[n..n + 2], 16)
                .map_err(|_| "Invalid visual graph header".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let graph: Graph =
        ron::de::from_bytes(&bytes).map_err(|e| format!("Invalid visual graph: {e}"))?;
    document_bounds(&graph)?;
    if draft_body(&graph).trim_end() != body.trim_end() {
        return Err("Generated source was edited as text. Preserve/merge those edits before reopening it as a visual graph.".into());
    }
    Ok(Some(graph))
}

fn draft_body(graph: &Graph) -> String {
    graph_source(graph).unwrap_or_else(|error| {
        format!(
            "// {}\n__kiln_graph_compile_error();\n",
            error.to_string().replace(['\n', '\r'], " ")
        )
    })
}

fn document_bounds(graph: &Graph) -> Result<(), String> {
    if graph.nodes.len() > 256
        || graph.connections.len() > 2048
        || graph.nodes.values().any(|node| {
            node.pins.len() > 260
                || !node.position.0.is_finite()
                || !node.position.1.is_finite()
                || node.position.0.abs() > 1_000_000.0
                || node.position.1.abs() > 1_000_000.0
        })
    {
        return Err("Visual document exceeds node/connection/pin/position bounds".into());
    }
    Ok(())
}
