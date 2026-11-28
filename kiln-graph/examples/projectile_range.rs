//! Regenerate the checked-in editable projectile range graphs deterministically.
use kiln_graph::{ArithmeticOp, ComparisonOp, Connection, ConstantValue, Graph, Node, NodeType};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct Port(Uuid, &'static str);
impl Port {
    fn done(self) -> Self {
        Self(self.0, "done")
    }
}
#[derive(Default)]
struct Builder {
    graph: Graph,
    next: u128,
}
impl Builder {
    fn id(&mut self) -> Uuid {
        self.next += 1;
        Uuid::from_u128(self.next)
    }
    fn node(&mut self, kind: NodeType, output: &'static str) -> Port {
        let n = self.graph.nodes.len();
        let mut node = Node::new(kind, ((n % 6) as f32 * 240.0, (n / 6) as f32 * 180.0));
        node.id = self.id();
        for pin in &mut node.pins {
            pin.id = self.id();
        }
        Port(self.graph.add_node(node), output)
    }
    fn wire(&mut self, from: Port, target: Port, input: &str) {
        let pin = |id: Uuid, name: &str| {
            self.graph.nodes[&id]
                .pins
                .iter()
                .find(|pin| pin.name == name)
                .unwrap()
                .id
        };
        let mut edge = Connection::new(pin(from.0, from.1), pin(target.0, input));
        edge.id = self.id();
        self.graph.add_connection(edge);
    }
    fn text(&mut self, value: &str) -> Port {
        self.node(
            NodeType::Constant {
                value: ConstantValue::String(value.into()),
            },
            "value",
        )
    }
    fn number(&mut self, value: f64) -> Port {
        self.node(
            NodeType::Constant {
                value: ConstantValue::Float(value),
            },
            "value",
        )
    }
    fn event(&mut self, name: &str) -> Port {
        self.node(NodeType::Event { name: name.into() }, "trigger")
    }
    fn call(&mut self, name: &str, args: &[Port], trigger: Option<Port>) -> Port {
        let node = self.node(NodeType::HostCall { name: name.into() }, "result");
        for (n, arg) in args.iter().enumerate() {
            self.wire(*arg, node, &format!("arg{}", n + 1));
        }
        if let Some(trigger) = trigger {
            self.wire(trigger, node, "trigger");
        }
        node
    }
    fn arithmetic(&mut self, op: ArithmeticOp, a: Port, b: Port) -> Port {
        let node = self.node(NodeType::Arithmetic { op }, "result");
        self.wire(a, node, "a");
        self.wire(b, node, "b");
        node
    }
    fn equal(&mut self, a: Port, b: Port) -> Port {
        let node = self.node(
            NodeType::Comparison {
                op: ComparisonOp::Equal,
            },
            "result",
        );
        self.wire(a, node, "a");
        self.wire(b, node, "b");
        node
    }
    fn branch(&mut self, condition: Port, yes: Port, no: Port) -> Port {
        let node = self.node(NodeType::Branch, "out");
        self.wire(condition, node, "condition");
        self.wire(yes, node, "true");
        self.wire(no, node, "false");
        node
    }
    fn finish(mut self, value: Port) -> Graph {
        let output = self.node(NodeType::Output, "in");
        self.wire(value, output, "in");
        self.graph
    }
    fn motion(&mut self, speed: Port) -> Port {
        let this = self.call("self_entity", &[], None);
        let pos = self.call("get_entity_pos", &[this], None);
        let zero = self.node(
            NodeType::Constant {
                value: ConstantValue::Integer(0),
            },
            "value",
        );
        let x = self.call("array_get", &[pos, zero], None);
        let dt = self.call("get_delta_time", &[], None);
        let dx = self.arithmetic(ArithmeticOp::Mul, speed, dt);
        let next = self.arithmetic(ArithmeticOp::Add, x, dx);
        let y = self.number(80.0);
        let update = self.event("On Update");
        self.call("set_entity_pos", &[this, next, y], Some(update))
    }
}
fn enemy() -> Graph {
    let mut g = Builder::default();
    let speed = g.number(-12.0);
    let movement = g.motion(speed);
    let one = g.node(
        NodeType::Constant {
            value: ConstantValue::Integer(1),
        },
        "value",
    );
    let collision = g.event("On Collision");
    let hits = g.text("hits");
    let score = g.call("game_set", &[hits, one], Some(collision));
    let this = g.call("self_entity", &[], None);
    let despawn = g.call("despawn_entity", &[this], Some(score.done()));
    let phase = g.call("script_phase", &[], None);
    let update = g.text("update");
    let condition = g.equal(phase, update);
    let output = g.branch(condition, movement.done(), despawn.done());
    g.finish(output)
}
fn projectile() -> Graph {
    let mut g = Builder::default();
    let input = g.call("get_input_x", &[], None);
    let base = g.number(100.0);
    let adjustment = g.arithmetic(ArithmeticOp::Mul, input, base);
    let speed = g.arithmetic(ArithmeticOp::Add, base, adjustment);
    let movement = g.motion(speed);
    g.finish(movement.done())
}
fn trigger() -> Graph {
    let mut g = Builder::default();
    let collision = g.event("On Collision");
    let hits = g.text("hits");
    let zero = g.node(
        NodeType::Constant {
            value: ConstantValue::Integer(0),
        },
        "value",
    );
    let score = g.call("game_get", &[hits, zero], None);
    let one = g.node(
        NodeType::Constant {
            value: ConstantValue::Integer(1),
        },
        "value",
    );
    let condition = g.equal(score, one);
    let key = g.text("activated");
    let yes = g.node(
        NodeType::Constant {
            value: ConstantValue::Boolean(true),
        },
        "value",
    );
    let state = g.call("state_set", &[key, yes], Some(collision));
    let status = g.text("Range cleared");
    let message = g.call("set_status", &[status], Some(state.done()));
    let zero = g.number(0.0);
    let alpha = g.number(1.0);
    let mut color = g.call("array_new", &[], None);
    for value in [zero, alpha, zero, alpha] {
        color = g.call("array_push", &[color, value], None);
    }
    let actor = g.text("actor");
    let field = g.text("sprite_color");
    let paint = g.call(
        "set_entity_component",
        &[actor, field, color],
        Some(message.done()),
    );
    let inactive = g.event("On Stop");
    let output = g.branch(condition, paint.done(), inactive);
    g.finish(output)
}
fn hud() -> Graph {
    let mut g = Builder::default();
    let key = g.text("hits");
    let zero = g.node(
        NodeType::Constant {
            value: ConstantValue::Integer(0),
        },
        "value",
    );
    let hits = g.call("game_get", &[key, zero], None);
    let current = g.call("get_score", &[], None);
    let condition = g.node(
        NodeType::Comparison {
            op: ComparisonOp::LessThan,
        },
        "result",
    );
    g.wire(current, condition, "a");
    g.wire(hits, condition, "b");
    let difference = g.arithmetic(ArithmeticOp::Sub, hits, current);
    let score = g.call("add_score", &[difference], None);
    let inactive = g.event("On Stop");
    let output = g.branch(condition, score.done(), inactive);
    g.finish(output)
}
fn main() {
    let directory = std::env::args().nth(1).expect("output scripts directory");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, graph) in [
        ("hud", hud()),
        ("enemy", enemy()),
        ("projectile", projectile()),
        ("trigger", trigger()),
    ] {
        kiln_graph::compile_graph(&graph).unwrap();
        let source = kiln_graph::document::encode(&graph).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.ks")),
            source,
        )
        .unwrap();
    }
}
