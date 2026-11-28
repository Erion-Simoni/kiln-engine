//! Lazy memoized DAG evaluation without recursive source duplication.
use super::{compatible, invalid, CompilerError};
use crate::{ArithmeticOp, ComparisonOp, ConstantValue, Graph, NodeType, PinDirection, PinType};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

type Result<T> = std::result::Result<T, CompilerError>;
#[derive(Clone, Copy)]
struct Input {
    source: Uuid,
    event: bool,
}
struct Layout<'a> {
    graph: &'a Graph,
    numbers: BTreeMap<Uuid, usize>,
    inputs: HashMap<Uuid, Input>,
    output: Uuid,
}
impl Layout<'_> {
    fn input(&self, id: Uuid, name: &str, optional: bool) -> Result<Option<Input>> {
        let pin = self.graph.nodes[&id]
            .pins
            .iter()
            .find(|pin| pin.name == name && pin.direction == PinDirection::Input)
            .ok_or_else(|| {
                invalid(format!(
                    "Node {id}: missing input pin {name}; reconnect an updated node"
                ))
            })?;
        match self.inputs.get(&pin.id).copied() {
            None if !optional => Err(invalid(format!("Node {id}: connect input {name}"))),
            input => Ok(input),
        }
    }
    fn dependencies(&self, id: Uuid) -> Result<Vec<Input>> {
        let mut dependencies = Vec::new();
        let mut add = |name: &str, optional| -> Result<()> {
            dependencies.extend(self.input(id, name, optional)?);
            Ok(())
        };
        match &self.graph.nodes[&id].node_type {
            NodeType::Constant { value } => { constant(value)?; }
            NodeType::Event { name } => { event(name)?; }
            NodeType::Arithmetic { .. } | NodeType::Comparison { .. } => {add("a",false)?;add("b",false)?;}
            NodeType::Branch => {add("condition",false)?;add("true",false)?;add("false",false)?;}
            NodeType::HostCall { name } => {
                let argc = arity(id,name)?;add("trigger",true)?;
                for n in 1..=argc {add(&format!("arg{n}"),false)?;}
            }
            NodeType::Output => add("in",true)?,
            _ => return Err(invalid(format!("Node {id}: specialized node has no supported gameplay contract; use a registered Host Call"))),
        }
        Ok(dependencies)
    }
    fn visit(
        &self,
        id: Uuid,
        active: &mut HashSet<Uuid>,
        heights: &mut BTreeMap<Uuid, usize>,
    ) -> Result<usize> {
        if let Some(height) = heights.get(&id) {
            return Ok(*height);
        }
        if active.len() >= 64 {
            return Err(invalid(format!("Node {id}: dependency depth exceeds 64")));
        }
        if !active.insert(id) {
            return Err(invalid(format!("Node {id}: cyclic dependency")));
        }
        let mut height = 1;
        for input in self.dependencies(id)? {
            height = height.max(1 + self.visit(input.source, active, heights)?);
        }
        if height > 64 {
            return Err(invalid(format!("Node {id}: dependency depth exceeds 64")));
        }
        active.remove(&id);
        heights.insert(id, height);
        Ok(height)
    }
}
fn layout(graph: &Graph) -> Result<Layout<'_>> {
    if graph.nodes.len() > 256 || graph.connections.len() > 2048 {
        return Err(invalid("Graph limit: 256 nodes / 2048 connections"));
    }
    let numbers = graph
        .nodes
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(n, id)| (id, n))
        .collect::<BTreeMap<_, _>>();
    let mut pins = HashMap::new();
    for id in numbers.keys() {
        let node = &graph.nodes[id];
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
    for connection in &graph.connections {
        let (source, sp) = pins
            .get(&connection.source_pin)
            .ok_or_else(|| invalid("Connection source pin is missing"))?;
        let (target, tp) = pins
            .get(&connection.target_pin)
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
            .insert(
                connection.target_pin,
                Input {
                    source: *source,
                    event: sp.pin_type == PinType::Event,
                },
            )
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
        .filter(|n| matches!(n.node_type, NodeType::Output))
        .map(|n| n.id)
        .collect::<Vec<_>>();
    if outputs.len() != 1 {
        return Err(invalid("Use exactly one Output node"));
    }
    Ok(Layout {
        graph,
        numbers,
        inputs,
        output: outputs[0],
    })
}
fn quoted(value: &str) -> Result<String> {
    if value.len() > 65536
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
fn constant(value: &ConstantValue) -> Result<String> {
    Ok(match value {
        ConstantValue::Integer(i64::MIN) => "(-9223372036854775807 - 1)".into(),
        ConstantValue::Integer(value) => value.to_string(),
        ConstantValue::Float(value) if value.is_finite() => {
            let text = value.to_string();
            if text.contains('.') {
                text
            } else {
                format!("{text}.0")
            }
        }
        ConstantValue::Float(_) => return Err(invalid("Non-finite graph number")),
        ConstantValue::Boolean(value) => value.to_string(),
        ConstantValue::String(value) => quoted(value)?,
    })
}
fn arity(id: Uuid, name: &str) -> Result<u32> {
    kiln_script::BUILTINS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, _, argc)| *argc)
        .ok_or_else(|| invalid(format!("Node {id}: unknown host function {name}")))
}
fn event(name: &str) -> Result<String> {
    let normalized = name.to_lowercase().replace([' ', '_'], "");
    let phase = normalized.strip_prefix("on").unwrap_or(&normalized);
    Ok(match phase {
        "start" => {
            "script_phase() == \"start\" || (self_entity() == \"\" && get_frame_count() == 1)"
                .into()
        }
        "update" => "script_phase() == \"update\"".into(),
        "collision" | "trigger" | "collisionenter" => {
            "script_phase() == \"collision_enter\"".into()
        }
        "collisionexit" => "script_phase() == \"collision_exit\"".into(),
        "signal" | "stop" => format!("script_phase() == {}", quoted(phase)?),
        "uievent" => "script_phase() == \"ui_event\"".into(),
        "animationevent" => "script_phase() == \"animation_event\"".into(),
        _ => return Err(invalid(format!("unsupported lifecycle event {name}"))),
    })
}
struct Emit<'a> {
    layout: Layout<'a>,
    source: String,
}
impl Emit<'_> {
    fn line(&mut self, line: &str) -> Result<()> {
        if self.source.len() + line.len() + 1 > 262144 {
            return Err(invalid("Generated graph exceeds 256 KiB; split the graph"));
        }
        self.source.push_str(line);
        self.source.push('\n');
        Ok(())
    }
    fn need(&mut self, id: Uuid, name: &str, optional: bool) -> Result<Option<String>> {
        let Some(input) = self.layout.input(id, name, optional)? else {
            return Ok(None);
        };
        let n = self.layout.numbers[&input.source];
        self.line(&format!(
            "if !computed{n} {{ graph_cursor={n}; continue; }}"
        ))?;
        Ok(Some(format!(
            "{}{n}",
            if input.event { "event" } else { "value" }
        )))
    }
    fn required(&mut self, id: Uuid, name: &str) -> Result<String> {
        self.need(id, name, false)
            .map(|value| value.expect("Validated required input"))
    }
    fn node(&mut self, id: Uuid) -> Result<()> {
        let n = self.layout.numbers[&id];
        match self.layout.graph.nodes[&id].node_type.clone() {
            NodeType::Constant { value } => {
                self.line(&format!("value{n} = {};", constant(&value)?))?
            }
            NodeType::Event { name } => {
                self.line(&format!("event{n} = {}; value{n}=event{n};", event(&name)?))?
            }
            NodeType::Arithmetic { op } => {
                let a = self.required(id, "a")?;
                let b = self.required(id, "b")?;
                let op = match op {
                    ArithmeticOp::Add => "+",
                    ArithmeticOp::Sub => "-",
                    ArithmeticOp::Mul => "*",
                    ArithmeticOp::Div => "/",
                };
                self.line(&format!("value{n} = {a} {op} {b};"))?;
            }
            NodeType::Comparison { op } => {
                let a = self.required(id, "a")?;
                let b = self.required(id, "b")?;
                let op = match op {
                    ComparisonOp::Equal => "==",
                    ComparisonOp::LessThan => "<",
                };
                self.line(&format!("value{n} = {a} {op} {b};"))?;
            }
            NodeType::HostCall { name } => {
                let trigger = self
                    .need(id, "trigger", true)?
                    .unwrap_or_else(|| "true".into());
                self.line(&format!("if {trigger} {{"))?;
                let args = (1..=arity(id, &name)?)
                    .map(|arg| self.required(id, &format!("arg{arg}")))
                    .collect::<Result<Vec<_>>>()?;
                self.line(&format!(
                    "value{n} = {name}({}); event{n}=true; }}",
                    args.join(", ")
                ))?;
            }
            NodeType::Branch => {
                let condition = self.required(id, "condition")?;
                self.line(&format!("if {condition} {{"))?;
                let yes = self.required(id, "true")?;
                self.line(&format!("event{n}={yes}; }} else {{"))?;
                let no = self.required(id, "false")?;
                self.line(&format!("event{n}={no}; }} value{n}=event{n};"))?;
            }
            NodeType::Output => {
                let value = self.need(id, "in", true)?.unwrap_or_else(|| "0".into());
                self.line(&format!("value{n}={value};"))?;
            }
            _ => unreachable!("Only validated reachable nodes are emitted"),
        }
        self.line(&format!("computed{n}=true; state_set(\"__kiln_graph:{id}\",value{n}); graph_cursor={}; continue;",self.layout.numbers[&self.layout.output]))
    }
    fn dispatch(&mut self, nodes: &[Uuid]) -> Result<()> {
        if nodes.len() == 1 {
            return self.node(nodes[0]);
        }
        let middle = nodes.len() / 2;
        let pivot = self.layout.numbers[&nodes[middle]];
        self.line(&format!("if graph_cursor < {pivot} {{"))?;
        self.dispatch(&nodes[..middle])?;
        self.line("} else {")?;
        self.dispatch(&nodes[middle..])?;
        self.line("}")
    }
}
pub(super) fn graph_source(graph: &Graph) -> Result<String> {
    let layout = layout(graph)?;
    let mut heights = BTreeMap::new();
    layout.visit(layout.output, &mut HashSet::new(), &mut heights)?;
    let nodes = heights.keys().copied().collect::<Vec<_>>();
    let mut emit = Emit {
        layout,
        source: String::new(),
    };
    for n in 0..graph.nodes.len() {
        emit.line(&format!(
            "let computed{n}=false; let value{n}=0; let event{n}=false;"
        ))?;
    }
    let output = emit.layout.numbers[&emit.layout.output];
    emit.line(&format!(
        "let graph_cursor={output}; while !computed{output} {{"
    ))?;
    emit.dispatch(&nodes)?;
    emit.line("}")?;
    emit.line(&format!("return value{output};"))?;
    let formatted = kiln_script::format_source(&emit.source).map_err(invalid)?;
    if formatted.len() > 262144 {
        return Err(invalid("Generated graph exceeds 256 KiB; split the graph"));
    }
    Ok(formatted)
}
