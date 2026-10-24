//! Node graph visual editor.
//!
//! Provides a drag-and-drop node graph editor for KilnScript gameplay logic.
//! Nodes can be dragged, connected via pins, and compiled to IR bytecode.

use egui::{vec2, Color32, Pos2, Response, Sense, Ui, Stroke, Vec2};
use kiln_graph::{
    node::{ArithmeticOp, ConstantValue, Node, NodeType},
    pin::{Pin, PinDirection, PinType},
    Graph, Connection,
};
use uuid::Uuid;

const PIN_RADIUS: f32 = 6.0;
const NODE_WIDTH: f32 = 140.0;
const NODE_HEIGHT: f32 = 80.0;
const PIN_LABEL_COLOR: Color32 = Color32::from_gray(180);
const NODE_BG_COLOR: Color32 = Color32::from_rgb(45, 45, 55);
const NODE_BORDER_COLOR: Color32 = Color32::from_gray(100);
const CONNECTION_COLOR: Color32 = Color32::from_rgb(100, 180, 255);
const CONNECTION_WIDTH: f32 = 2.0;

/// State of the node graph editor.
#[derive(Debug, Default)]
pub struct GraphEditorState {
    pub graph: Graph,
    pub selected_node: Option<uuid::Uuid>,
    pub dragging: Option<(uuid::Uuid, Pos2)>,
    pub creating_connection: Option<uuid::Uuid>,
    pub pan_offset: Pos2,
    pub zoom: f32,
    pub dirty: bool,
}

impl GraphEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node_type: NodeType, pos: Pos2) -> uuid::Uuid {
        let mut node = Node::new(node_type, (pos.x, pos.y));
        let id = node.id;
        self.graph.add_node(node);
        self.selected_node = Some(id);
        self.dirty = true;
        id
    }

    pub fn remove_selected(&mut self) {
        if let Some(id) = self.selected_node.take() {
            self.graph.remove_node(id);
            self.dirty = true;
        }
    }

    pub fn get_node_mut(&mut self, id: uuid::Uuid) -> Option<&mut Node> {
        self.graph.nodes.get_mut(&id)
    }

    pub fn get_node(&self, id: uuid::Uuid) -> Option<&Node> {
        self.graph.nodes.get(&id)
    }

    pub fn can_connect(&self, source: uuid::Uuid, target: uuid::Uuid) -> bool {
        let source_pin = self.graph.nodes.values().find_map(|n| {
            n.pins.iter().find(|p| p.id == source && p.direction == PinDirection::Output)
        });
        let target_pin = self.graph.nodes.values().find_map(|n| {
            n.pins.iter().find(|p| p.id == target && p.direction == PinDirection::Input)
        });

        match (source_pin, target_pin) {
            (Some(s), Some(t)) => s.pin_type == t.pin_type,
            _ => false,
        }
    }

    pub fn add_connection(&mut self, source: uuid::Uuid, target: uuid::Uuid) {
        if self.can_connect(source, target) {
            let exists = self.graph.connections.iter().any(|c| {
                c.source_pin == source && c.target_pin == target
            });
            if !exists {
                self.graph.connections.push(Connection {
                    id: uuid::Uuid::new_v4(),
                    source_pin: source,
                    target_pin: target,
                });
                self.dirty = true;
            }
        }
    }

    pub fn remove_connection(&mut self, id: uuid::Uuid) {
        self.graph.remove_connection(id);
        self.dirty = true;
    }
}

/// Render the node graph editor panel.
pub fn render_graph_editor(ui: &mut egui::Ui, state: &mut GraphEditorState) {
    ui.heading("Node Graph");

    ui.horizontal(|ui| {
        if ui.button("+ Event").clicked() {
            state.add_node(NodeType::Event { name: "On Start".to_string() }, ui.cursor().min);
        }
        if ui.button("+ Constant").clicked() {
            state.add_node(NodeType::Constant { value: ConstantValue::Integer(0) }, ui.cursor().min);
        }
        if ui.button("+ Arithmetic").clicked() {
            state.add_node(NodeType::Arithmetic { op: ArithmeticOp::Add }, ui.cursor().min);
        }
        if ui.button("+ Branch").clicked() {
            state.add_node(NodeType::Branch, ui.cursor().min);
        }
        if ui.button("+ Output").clicked() {
            state.add_node(NodeType::Output, ui.cursor().min);
        }
        if ui.button("Delete").clicked() {
            state.remove_selected();
        }
        if state.dirty {
            ui.colored_label(Color32::from_rgb(255, 200, 0), "unsaved changes");
        }
    });

    ui.separator();

    egui::ScrollArea::both().show(ui, |ui| {
        let rect = ui.available_rect_before_wrap();
        
        // First, draw all connections in a scope that drops the painter
        {
            let mut painter = ui.painter();
            painter.rect_filled(ui.available_rect_before_wrap(), 0.0, Color32::from_rgb(30, 30, 35));

            let mut connections_to_draw: Vec<(Pos2, Pos2)> = Vec::new();
            let mut connections_to_remove: Vec<uuid::Uuid> = Vec::new();

            for conn in &state.graph.connections {
                let source_pin = state.graph.nodes.values().find_map(|n| {
                    n.pins.iter().find(|p| p.id == conn.source_pin)
                });
                let target_pin = state.graph.nodes.values().find_map(|n| {
                    n.pins.iter().find(|p| p.id == conn.target_pin)
                });

                if let (Some(source_pin), Some(target_pin)) = (source_pin, target_pin) {
                    let source_node = state.graph.nodes.values().find(|n| {
                        n.pins.iter().any(|p| p.id == source_pin.id)
                    });
                    let target_node = state.graph.nodes.values().find(|n| n.pins.iter().any(|p| p.id == target_pin.id));

                    if let (Some(src_node), Some(tgt_node)) = (source_node, target_node) {
                        let src_pin_idx = src_node.pins.iter().position(|p| p.id == conn.source_pin).unwrap_or(0);
                        let tgt_pin_idx = tgt_node.pins.iter().position(|p| p.id == conn.target_pin).unwrap_or(0);

                        let src_pos = Pos2::new(
                            src_node.position.0 + NODE_WIDTH,
                            src_node.position.1 + 24.0 + src_pin_idx as f32 * 20.0,
                        );
                        let tgt_pos = Pos2::new(
                            tgt_node.position.0,
                            tgt_node.position.1 + 24.0 + tgt_pin_idx as f32 * 20.0,
                        );

                        connections_to_draw.push((src_pos, tgt_pos));
                    }
                } else {
                    connections_to_remove.push(conn.id);
                }
            }

            for conn_id in connections_to_remove {
                state.remove_connection(conn_id);
            }

            for (start, end) in &connections_to_draw {
                let mut painter = ui.painter();
                painter.line_segment([*start, *end], egui::Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR));
            }
        }

        // Draw nodes in a separate scope that drops the painter
        let mut node_rects: Vec<(uuid::Uuid, egui::Rect)> = Vec::new();
        {
            let mut painter = ui.painter();
            for node in state.graph.nodes.values() {
                let rect = egui::Rect::from_center_size(
                    Pos2::new(node.position.0, node.position.1),
                    vec2(NODE_WIDTH, NODE_HEIGHT),
                );
                node_rects.push((node.id, rect));

                painter.rect_filled(rect, 4.0, NODE_BG_COLOR);
                painter.rect_stroke(rect, 4.0, egui::Stroke::new(4.0, NODE_BORDER_COLOR));

                let title = node_type_name(&node.node_type);
                painter.text(
                    rect.center_top() + Vec2::new(0.0, 8.0),
                    egui::Align2::CENTER_TOP,
                    title,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );

                draw_pin_pins(painter, node, rect);
            }
            // painter is dropped here, releasing the borrow on ui
        }

        // Get pointer pos before any other borrows
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());

        // Get pointer pos before any other borrows
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        
        if let Some(id) = state.selected_node {
            if let Some(node) = state.graph.nodes.get_mut(&id) {
                if let Some(pointer_pos) = pointer_pos {  // Use cloned value
                    if ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
                        node.position.0 = pointer_pos.x;
                        node.position.1 = pointer_pos.y;
                        state.dirty = true;
                    }
                }
            }
        }

        if let Some(source_pin_id) = state.creating_connection {
            if let Some(source_pin) = state.graph.nodes.values().find_map(|n| {
                n.pins.iter().find(|p| p.id == source_pin_id)
            }) {
                if let Some(source_node) = state.graph.nodes.values().find(|n| n.pins.iter().any(|p| p.id == source_pin_id)) {
                    let start = Pos2::new(
                        source_node.position.0 + NODE_WIDTH,
                        source_node.position.1 + 24.0 + source_node.pins.iter().position(|p| p.id == source_pin_id).unwrap_or(0) as f32 * 20.0,
                    );
                    if let Some(end) = ui.input(|i| i.pointer.hover_pos()) {
                        let mut painter = ui.painter();
                        painter.line_segment([start, end], egui::Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR));
                    }
                }
            }
        }

        let mut clicked_node: Option<uuid::Uuid> = None;
        for (node_id, rect) in &node_rects {
            let response = ui.interact(*rect, ui.id().with(node_id), Sense::click_and_drag());
            if response.clicked() {
                clicked_node = Some(*node_id);
            }
            if response.hovered() {
                state.selected_node = Some(*node_id);
            }
            if response.drag_started() {
                state.selected_node = Some(*node_id);
            }
        }

        if let Some(id) = clicked_node {
            state.selected_node = Some(id);
        }
    });
}

fn draw_pin_pins(painter: &egui::Painter, node: &Node, rect: egui::Rect) {
    let mut y = rect.top() + 24.0;
    for pin in &node.pins {
        let pin_pos = match pin.direction {
            PinDirection::Input => Pos2::new(rect.left(), y),
            PinDirection::Output => Pos2::new(rect.right(), y),
        };

        painter.circle_filled(pin_pos, PIN_RADIUS, NODE_BORDER_COLOR);

        y += 20.0;
    }
}

fn node_type_name(node_type: &NodeType) -> String {
    match node_type {
        NodeType::Event { name } => format!("Event: {name}"),
        NodeType::Constant { .. } => "Constant".to_string(),
        NodeType::Arithmetic { .. } => "Arithmetic".to_string(),
        NodeType::Comparison { .. } => "Compare".to_string(),
        NodeType::Branch => "Branch".to_string(),
        NodeType::HostCall { name } => format!("Call: {name}"),
        NodeType::Output => "Output".to_string(),
        NodeType::AnimationClip { .. } => "Anim Clip".to_string(),
        NodeType::AnimationState { .. } => "Anim State".to_string(),
        NodeType::AnimationTransition => "Anim Transition".to_string(),
        NodeType::Selector => "Selector".to_string(),
        NodeType::Sequence => "Sequence".to_string(),
        NodeType::BehaviorCondition => "Behavior Condition".to_string(),
        NodeType::BehaviorAction { .. } => "Behavior Action".to_string(),
        NodeType::AIState { .. } => "AI State".to_string(),
        NodeType::AITransition => "AI Transition".to_string(),
        NodeType::Dialogue { .. } => "Dialogue".to_string(),
        NodeType::DialogueChoice => "Dialogue Choice".to_string(),
        NodeType::QuestStart { .. } => "Quest Start".to_string(),
        NodeType::QuestComplete { .. } => "Quest Complete".to_string(),
        NodeType::QuestObjectiveUpdate { .. } => "Quest Objective".to_string(),
        NodeType::DataTableLookup { .. } => "DataTable Lookup".to_string(),
        NodeType::DataTableRowGet { .. } => "DataTable Row".to_string(),
    }
}
