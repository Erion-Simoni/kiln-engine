//! Frozen v1 source emitter, used only to verify legacy visual documents.
use crate::{ArithmeticOp, ComparisonOp, Graph, NodeType, PinDirection, PinType};
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

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

struct Lower<'a> {
    graph: &'a Graph,
    numbers: BTreeMap<Uuid, usize>,
    inputs: HashMap<Uuid, (Uuid, String)>,
    visiting: HashSet<Uuid>,
    source: String,
}
impl Lower<'_> {
    fn line(&mut self, line: &str) -> Result<(), CompilerError> {
        if self.source.len() + line.len() + 1 > 262_144 {
            return Err(invalid("Generated graph exceeds 256 KiB; split the graph"));
        }
        self.source.push_str(line);
        self.source.push('\n');
        Ok(())
    }
    fn input(
        &mut self,
        id: Uuid,
        name: &str,
        optional: bool,
    ) -> Result<Option<String>, CompilerError> {
        let node = &self.graph.nodes[&id];
        let pin = node
            .pins
            .iter()
            .find(|p| p.name == name && p.direction == PinDirection::Input)
            .ok_or_else(|| {
                invalid(format!(
                    "Node {id}: missing input pin {name}; reconnect an updated node"
                ))
            })?;
        let Some((source, output)) = self.inputs.get(&pin.id).cloned() else {
            return if optional {
                Ok(None)
            } else {
                Err(invalid(format!("Node {id}: connect input {name}")))
            };
        };
        self.node(source)?;
        let number = self.numbers[&source];
        let source_node = &self.graph.nodes[&source];
        let event = source_node
            .pins
            .iter()
            .find(|p| p.name == output && p.direction == PinDirection::Output)
            .is_some_and(|p| p.pin_type == PinType::Event);
        Ok(Some(format!(
            "{}{number}",
            if event { "event" } else { "value" }
        )))
    }
    fn required(&mut self, id: Uuid, name: &str) -> Result<String, CompilerError> {
        self.input(id, name, false)
            .map(|value| value.expect("required input"))
    }
    fn node(&mut self, id: Uuid) -> Result<(), CompilerError> {
        if self.visiting.len() >= 64 {
            return Err(invalid(format!("Node {id}: dependency depth exceeds 64")));
        }
        if !self.visiting.insert(id) {
            return Err(invalid(format!("Node {id}: cyclic dependency")));
        }
        let n = self.numbers[&id];
        let node_type = self.graph.nodes[&id].node_type.clone();
        self.line(&format!("if !computed{n} {{"))?;
        match node_type {
            NodeType::Constant {value} => {
                let value = match value {
                    crate::ConstantValue::Integer(i64::MIN) => "(-9223372036854775807 - 1)".into(),
                    crate::ConstantValue::Integer(v) => v.to_string(),
                    crate::ConstantValue::Float(v) if v.is_finite() => {
                        let text = v.to_string(); if text.contains('.') {text} else {format!("{text}.0")}
                    },
                    crate::ConstantValue::Float(_) => return Err(invalid(format!("Node {id}: non-finite number"))),
                    crate::ConstantValue::Boolean(v) => v.to_string(),
                    crate::ConstantValue::String(v) => quoted(&v)?,
                };
                self.line(&format!("value{n} = {value};"))?;
            }
            NodeType::Arithmetic {op} => {
                let a = self.required(id,"a")?; let b = self.required(id,"b")?;
                let op = match op {ArithmeticOp::Add=>"+",ArithmeticOp::Sub=>"-",ArithmeticOp::Mul=>"*",ArithmeticOp::Div=>"/"};
                self.line(&format!("value{n} = {a} {op} {b};"))?;
            }
            NodeType::Comparison {op} => {
                let a = self.required(id,"a")?; let b = self.required(id,"b")?;
                let op = match op {ComparisonOp::Equal=>"==",ComparisonOp::LessThan=>"<"};
                self.line(&format!("value{n} = {a} {op} {b};"))?;
            }
            NodeType::HostCall {name} => {
                let (_,_,argc) = kiln_script::BUILTINS.iter().find(|(n,_,_)| *n == name)
                    .ok_or_else(|| invalid(format!("Node {id}: unknown host function {name}")))?;
                let trigger = self.input(id,"trigger",true)?.unwrap_or_else(|| "true".into());
                self.line(&format!("if {trigger} {{"))?;
                let args = (1..=*argc).map(|i| self.required(id,&format!("arg{i}"))).collect::<Result<Vec<_>,_>>()?;
                self.line(&format!("value{n} = {name}({});", args.join(", ")))?;
                self.line(&format!("event{n} = true;\n}}"))?;
            }
            NodeType::Event {name} => {
                let normalized = name.to_lowercase().replace([' ', '_'], "");
                let phase = normalized.strip_prefix("on").unwrap_or(&normalized);
                let expression = match phase {
                    "start" => "script_phase() == \"start\" || (self_entity() == \"\" && get_frame_count() == 1)".into(),
                    "update" => "script_phase() == \"update\"".into(),
                    "collision" | "trigger" | "collisionenter" => "script_phase() == \"collision_enter\"".into(),
                    "collisionexit" => "script_phase() == \"collision_exit\"".into(),
                    "signal" | "stop" => format!("script_phase() == {}",quoted(phase)?),
                    "uievent" => "script_phase() == \"ui_event\"".into(),
                    "animationevent" => "script_phase() == \"animation_event\"".into(),
                    _ => return Err(invalid(format!("Node {id}: unsupported lifecycle event {name}"))),
                };
                self.line(&format!("event{n} = {expression};\nvalue{n} = event{n};"))?;
            }
            NodeType::Branch => {
                let condition = self.required(id,"condition")?;
                self.line(&format!("if {condition} {{"))?;
                let a = self.required(id,"true")?;
                self.line(&format!("event{n} = {a};\n}} else {{"))?;
                let b = self.required(id,"false")?;
                self.line(&format!("event{n} = {b};\n}}\nvalue{n} = event{n};"))?;
            }
            NodeType::Output => {
                let value = self.input(id,"in",true)?.unwrap_or_else(|| "0".into());
                self.line(&format!("value{n} = {value};"))?;
            }
            _ => return Err(invalid(format!("Node {id}: specialized node has no supported gameplay contract; use a registered Host Call"))),
        }
        self.line(&format!(
            "computed{n} = true;\nstate_set(\"__kiln_graph:{id}\",value{n});\n}}"
        ))?;
        self.visiting.remove(&id);
        Ok(())
    }
}
fn quoted(value: &str) -> Result<String, CompilerError> {
    if value.len() > 65_536
        || value
            .chars()
            .any(|c| c.is_ascii_control() && !['\n', '\r', '\t'].contains(&c))
    {
        return Err(invalid(
            "Graph string exceeds value bounds or contains unsupported control characters",
        ));
    }
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    ))
}

/// Generate ordinary source. A node is evaluated once per invocation, including fan-out.
pub fn graph_source(graph: &Graph) -> Result<String, CompilerError> {
    if graph.nodes.len() > 256 || graph.connections.len() > 2048 {
        return Err(invalid("Graph limit: 256 nodes / 2048 connections"));
    }
    let mut pins = HashMap::new();
    let numbers = graph
        .nodes
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(n, id)| (id, n))
        .collect::<BTreeMap<_, _>>();
    for (id, node) in &graph.nodes {
        if *id != node.id
            || !node.position.0.is_finite()
            || !node.position.1.is_finite()
            || node.pins.len() > 260
        {
            return Err(invalid(format!(
                "Node {id}: invalid identity, position or pin count"
            )));
        }
        let mut names = HashSet::new();
        for pin in &node.pins {
            if pins.insert(pin.id, (*id, pin)).is_some()
                || !names.insert((pin.direction as u8, pin.name.as_str()))
            {
                return Err(invalid(format!("Node {id}: duplicate pin identity/name")));
            }
        }
    }
    let mut inputs = HashMap::new();
    for conn in &graph.connections {
        let (source, sp) = pins
            .get(&conn.source_pin)
            .ok_or_else(|| invalid("Connection source pin is missing"))?;
        let (target, tp) = pins
            .get(&conn.target_pin)
            .ok_or_else(|| invalid("Connection target pin is missing"))?;
        if sp.direction != PinDirection::Output || tp.direction != PinDirection::Input {
            return Err(invalid("Connections must run from output to input"));
        }
        if !matches!(graph.nodes[target].node_type, NodeType::Output)
            && !compatible(sp.pin_type, tp.pin_type)
        {
            return Err(invalid(format!(
                "Node {target}: incompatible input {}",
                tp.name
            )));
        }
        if inputs
            .insert(conn.target_pin, (*source, sp.name.clone()))
            .is_some()
        {
            return Err(invalid(format!(
                "Node {target}: multiple connections to input {}",
                tp.name
            )));
        }
    }
    let outputs = graph
        .nodes
        .values()
        .filter(|node| matches!(node.node_type, NodeType::Output))
        .map(|n| n.id)
        .collect::<Vec<_>>();
    if outputs.len() != 1 {
        return Err(invalid("Use exactly one Output node"));
    }
    let mut lower = Lower {
        graph,
        numbers,
        inputs,
        visiting: HashSet::new(),
        source: String::new(),
    };
    for n in 0..graph.nodes.len() {
        lower.line(&format!(
            "let computed{n}=false;\nlet value{n}=0;\nlet event{n}=false;"
        ))?;
    }
    lower.node(outputs[0])?;
    lower.line(&format!("return value{};", lower.numbers[&outputs[0]]))?;
    kiln_script::format_source(&lower.source).map_err(invalid)
}
