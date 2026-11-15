//! Node graph visual editor.
//!
//! Provides a drag-and-drop node graph editor for KilnScript gameplay logic.
//! Nodes can be dragged, connected via pins, and compiled to IR bytecode.

use egui::{Color32, Pos2, Response, RichText, Sense, Stroke, TextStyle, Vec2};
use kiln_graph::{
    compiler::compile_graph,
    node::{ArithmeticOp, ComparisonOp, ConstantValue, Node, NodeType},
    pin::{PinDirection, PinType},
    Connection, Graph,
};
use uuid::Uuid;

const PIN_RADIUS: f32 = 6.0;
const NODE_WIDTH: f32 = 160.0;
const NODE_HEIGHT: f32 = 80.0;
const NODE_BG_COLOR: Color32 = Color32::from_rgb(45, 45, 55);
const NODE_BORDER_COLOR: Color32 = Color32::from_gray(100);
const NODE_SELECTED_BORDER: Color32 = Color32::from_rgb(100, 200, 255);
const CONNECTION_COLOR: Color32 = Color32::from_rgb(100, 180, 255);
const CONNECTION_WIDTH: f32 = 2.5;
const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(100, 100, 120, 30);
const GRID_SIZE: f32 = 50.0;

/// State of the node graph editor.
#[derive(Debug, Default)]
pub struct GraphEditorState {
    pub graph: Graph,
    source_buffer: String,
    undo: Vec<String>,
    redo: Vec<String>,
    history_resource: Option<(std::path::PathBuf, String)>,
    drag_undo_recorded: bool,
    visual_document: bool,
    document_message: String,
    pub document_path: String,
    editing_draft: Option<Node>,
    pub selected_node: Option<uuid::Uuid>,
    pub dragging: Option<(uuid::Uuid, Pos2)>,
    pub creating_connection: Option<uuid::Uuid>,
    pub pan_offset: Pos2,
    pub zoom: f32,
    pub dirty: bool,
    /// Compilation result (program or error)
    pub compile_result: Option<Result<kiln_ir::Program, kiln_graph::compiler::CompilerError>>,
    /// Node being edited (for parameter dialog)
    pub editing_node: Option<uuid::Uuid>,
    /// Search filter for node types
    pub search_filter: String,
    /// Whether to show the add node context menu
    pub show_add_menu: bool,
    /// Position where add menu was opened
    pub add_menu_pos: Pos2,
}

impl GraphEditorState {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            ..Default::default()
        }
    }

    fn trim_history(&mut self) {
        while self.undo.len() + self.redo.len() > 32
            || self
                .undo
                .iter()
                .chain(&self.redo)
                .map(String::len)
                .sum::<usize>()
                > 8_388_608
        {
            if !self.undo.is_empty() {
                self.undo.remove(0);
            } else {
                self.redo.remove(0);
            }
        }
    }
    fn remember_undo(&mut self, source: String) {
        self.undo.push(source);
        self.trim_history();
    }

    pub fn add_node(&mut self, node_type: NodeType, pos: Pos2) -> uuid::Uuid {
        let node = Node::new(node_type, (pos.x, pos.y));
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
            n.pins
                .iter()
                .find(|p| p.id == source && p.direction == PinDirection::Output)
        });
        let target_pin = self.graph.nodes.values().find_map(|n| {
            n.pins
                .iter()
                .find(|p| p.id == target && p.direction == PinDirection::Input)
        });

        match (source_pin, target_pin) {
            (Some(s), Some(t)) => {
                kiln_graph::compiler::compatible(s.pin_type, t.pin_type)
                    || self
                        .graph
                        .get_pin_node(target)
                        .is_some_and(|n| matches!(n.node_type, NodeType::Output))
            }
            _ => false,
        }
    }

    pub fn add_connection(&mut self, source: uuid::Uuid, target: uuid::Uuid) {
        if self.can_connect(source, target) {
            let exists = self
                .graph
                .connections
                .iter()
                .any(|c| c.source_pin == source && c.target_pin == target);
            if !exists {
                self.graph
                    .connections
                    .retain(|connection| connection.target_pin != target);
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

    /// Compile the graph to IR.
    pub fn compile(&mut self) {
        self.compile_result = Some(compile_graph(&self.graph));
    }
}

/// Category for node types in the add menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeCategory {
    Events,
    Values,
    Operations,
    ControlFlow,
    HostCalls,
    Output,
    Animation,
    Behavior,
    AI,
    Dialogue,
    Quests,
    DataTables,
}

/// Get all available node types organized by category.
fn node_types_by_category() -> Vec<(NodeCategory, Vec<(NodeType, &'static str)>)> {
    vec![
        (
            NodeCategory::Events,
            vec![
                (
                    NodeType::Event {
                        name: "On Start".to_string(),
                    },
                    "On Start",
                ),
                (
                    NodeType::Event {
                        name: "On Update".to_string(),
                    },
                    "On Update",
                ),
                (
                    NodeType::Event {
                        name: "On Collision".to_string(),
                    },
                    "On Collision",
                ),
                (
                    NodeType::Event {
                        name: "On Trigger".to_string(),
                    },
                    "On Trigger",
                ),
            ],
        ),
        (
            NodeCategory::Values,
            vec![
                (
                    NodeType::Constant {
                        value: ConstantValue::Integer(0),
                    },
                    "Integer Constant",
                ),
                (
                    NodeType::Constant {
                        value: ConstantValue::Float(0.0),
                    },
                    "Float Constant",
                ),
                (
                    NodeType::Constant {
                        value: ConstantValue::Boolean(false),
                    },
                    "Boolean Constant",
                ),
                (
                    NodeType::Constant {
                        value: ConstantValue::String("".to_string()),
                    },
                    "String Constant",
                ),
            ],
        ),
        (
            NodeCategory::Operations,
            vec![
                (
                    NodeType::Arithmetic {
                        op: ArithmeticOp::Add,
                    },
                    "Add",
                ),
                (
                    NodeType::Arithmetic {
                        op: ArithmeticOp::Sub,
                    },
                    "Subtract",
                ),
                (
                    NodeType::Arithmetic {
                        op: ArithmeticOp::Mul,
                    },
                    "Multiply",
                ),
                (
                    NodeType::Arithmetic {
                        op: ArithmeticOp::Div,
                    },
                    "Divide",
                ),
                (
                    NodeType::Comparison {
                        op: ComparisonOp::Equal,
                    },
                    "Equal",
                ),
                (
                    NodeType::Comparison {
                        op: ComparisonOp::LessThan,
                    },
                    "Less Than",
                ),
            ],
        ),
        (
            NodeCategory::ControlFlow,
            vec![(NodeType::Branch, "Branch")],
        ),
        (
            NodeCategory::HostCalls,
            vec![
                (
                    NodeType::HostCall {
                        name: "print".to_string(),
                    },
                    "Print",
                ),
                (
                    NodeType::HostCall {
                        name: "move_player".to_string(),
                    },
                    "Move Player",
                ),
                (
                    NodeType::HostCall {
                        name: "add_score".to_string(),
                    },
                    "Add Score",
                ),
                (
                    NodeType::HostCall {
                        name: "collect_coin".to_string(),
                    },
                    "Collect Coin",
                ),
                (
                    NodeType::HostCall {
                        name: "play_sound".to_string(),
                    },
                    "Play Sound",
                ),
                (
                    NodeType::HostCall {
                        name: "set_status".to_string(),
                    },
                    "Set Status",
                ),
                (
                    NodeType::HostCall {
                        name: "spawn_entity".to_string(),
                    },
                    "Spawn Entity",
                ),
                (
                    NodeType::HostCall {
                        name: "find_nearest".to_string(),
                    },
                    "Find Nearest",
                ),
            ],
        ),
        (NodeCategory::Output, vec![(NodeType::Output, "Output")]),
        (
            NodeCategory::Animation,
            vec![
                (
                    NodeType::AnimationClip {
                        clip_name: "".to_string(),
                    },
                    "Play Clip",
                ),
                (
                    NodeType::AnimationState {
                        state_name: "".to_string(),
                    },
                    "Anim State",
                ),
                (NodeType::AnimationTransition, "Anim Transition"),
            ],
        ),
        (
            NodeCategory::Behavior,
            vec![
                (NodeType::Selector, "Selector"),
                (NodeType::Sequence, "Sequence"),
                (NodeType::BehaviorCondition, "Condition"),
                (
                    NodeType::BehaviorAction {
                        name: "".to_string(),
                    },
                    "Action",
                ),
            ],
        ),
        (
            NodeCategory::AI,
            vec![
                (
                    NodeType::AIState {
                        state_name: "".to_string(),
                    },
                    "AI State",
                ),
                (NodeType::AITransition, "AI Transition"),
            ],
        ),
        (
            NodeCategory::Dialogue,
            vec![
                (
                    NodeType::Dialogue {
                        dialogue_id: "".to_string(),
                    },
                    "Show Dialogue",
                ),
                (NodeType::DialogueChoice, "Dialogue Choice"),
            ],
        ),
        (
            NodeCategory::Quests,
            vec![
                (
                    NodeType::QuestStart {
                        quest_id: "".to_string(),
                    },
                    "Start Quest",
                ),
                (
                    NodeType::QuestComplete {
                        quest_id: "".to_string(),
                    },
                    "Complete Quest",
                ),
                (
                    NodeType::QuestObjectiveUpdate {
                        quest_id: "".to_string(),
                        objective_index: 0,
                    },
                    "Update Objective",
                ),
            ],
        ),
        (
            NodeCategory::DataTables,
            vec![
                (
                    NodeType::DataTableLookup {
                        table_id: "".to_string(),
                        column_name: "".to_string(),
                    },
                    "Lookup",
                ),
                (
                    NodeType::DataTableRowGet {
                        table_id: "".to_string(),
                    },
                    "Get Row",
                ),
            ],
        ),
    ]
}

fn category_name(cat: NodeCategory) -> &'static str {
    match cat {
        NodeCategory::Events => "📅 Events",
        NodeCategory::Values => "🔢 Values",
        NodeCategory::Operations => "⚙ Operations",
        NodeCategory::ControlFlow => "🔀 Control Flow",
        NodeCategory::HostCalls => "📞 Host Calls",
        NodeCategory::Output => "📤 Output",
        NodeCategory::Animation => "🎬 Animation",
        NodeCategory::Behavior => "🌳 Behavior Tree",
        NodeCategory::AI => "🤖 AI",
        NodeCategory::Dialogue => "💬 Dialogue",
        NodeCategory::Quests => "🎯 Quests",
        NodeCategory::DataTables => "📊 Data Tables",
    }
}

/// Edit a visual .ks resource using the normal guarded source/recovery buffer.
pub fn render_document(
    ui: &mut egui::Ui,
    state: &mut GraphEditorState,
    script: &mut crate::script_editor::ScriptEditor,
    root: &std::path::Path,
) {
    ui.horizontal(|ui| {
        ui.label("Visual script:");
        if state.document_path.is_empty() {
            state.document_path = "scripts/gameplay_graph.ks".into();
        }
        ui.text_edit_singleline(&mut state.document_path);
        if ui.button("Open / New").clicked() {
            let path = state.document_path.clone();
            match script.open(root, &path) {
                Ok(()) if script.source.is_empty() => {
                    let mut graph = Graph::new();
                    graph.add_node(Node::new(NodeType::Output, (450.0, 100.0)));
                    match kiln_graph::document::encode(&graph) {
                        Ok(source) => script.source = source,
                        Err(error) => state.document_message = error.to_string(),
                    }
                }
                Ok(()) => {}
                Err(error) => state.document_message = error,
            }
        }
        if ui.button("Save Script").clicked() {
            state.document_message = match script.save() {
                Ok(()) => "Visual script saved".into(),
                Err(error) => error,
            };
        }
        if ui
            .add_enabled(!state.undo.is_empty(), egui::Button::new("Undo"))
            .clicked()
        {
            if let Some(source) = state.undo.pop() {
                state
                    .redo
                    .push(std::mem::replace(&mut script.source, source));
                state.trim_history();
            }
        }
        if ui
            .add_enabled(!state.redo.is_empty(), egui::Button::new("Redo"))
            .clicked()
        {
            if let Some(source) = state.redo.pop() {
                state.remember_undo(std::mem::replace(&mut script.source, source));
            }
        }
        if script.is_dirty() {
            ui.colored_label(Color32::YELLOW, "Unsaved script");
        }
    });
    let resource = (root.to_owned(), script.loaded_path().to_owned());
    if state.history_resource.as_ref() != Some(&resource) {
        state.undo.clear();
        state.redo.clear();
        state.history_resource = Some(resource);
        state.dragging = None;
        state.drag_undo_recorded = false;
    }
    if script.source != state.source_buffer {
        match kiln_graph::document::decode(&script.source) {
            Ok(Some(graph)) => {
                state.visual_document = true;
                state.graph = graph;
                state.document_path = script.loaded_path().into();
                state.editing_node = None;
                state.editing_draft = None;
                state.compile_result = None;
                state.document_message =
                    "Visual script uses normal project Save, recovery and export.".into();
            }
            Ok(None) => {
                state.visual_document = false;
                state.document_message="Choose a new .ks path or open an existing visual script. Current text is preserved.".into();
            }
            Err(error) => {
                state.visual_document = false;
                state.document_message = error;
            }
        }
        state.source_buffer = script.source.clone();
    }
    ui.label(&state.document_message);
    // A changed/manual body remains text; never replace it with stale visual state.
    if !state.visual_document {
        return;
    }
    render_graph_editor(ui, state);
    if state.dirty {
        match kiln_graph::document::encode(&state.graph) {
            Ok(source) => {
                if source != script.source {
                    state.redo.clear();
                    if state.dragging.is_none() || !state.drag_undo_recorded {
                        state.remember_undo(script.source.clone());
                        if state.dragging.is_some() {
                            state.drag_undo_recorded = true;
                        }
                    }
                    state.redo.clear();
                }
                script.source = source.clone();
                state.source_buffer = source;
                state.compile_result = None;
            }
            Err(error) => {
                // Reject edits outside the document budget as one transaction.
                if let Ok(Some(previous)) = kiln_graph::document::decode(&script.source) {
                    state.graph = previous;
                }
                state.document_message = format!("Graph edit rejected: {error}");
            }
        }
        state.dirty = false;
    }
    if !ui.input(|input| input.pointer.primary_down()) {
        state.dragging = None;
        state.drag_undo_recorded = false;
    }
}

/// Render the node graph editor panel.
pub fn render_graph_editor(ui: &mut egui::Ui, state: &mut GraphEditorState) {
    state.zoom = if state.zoom.is_finite() && state.zoom > 0.0 {
        state.zoom.clamp(0.1, 5.0)
    } else {
        1.0
    };
    ui.heading("Node Graph");

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("+ Event").clicked() {
            state.add_node(
                NodeType::Event {
                    name: "On Update".to_string(),
                },
                Pos2::new(100.0, 100.0),
            );
        }
        if ui.button("+ Constant").clicked() {
            state.add_node(
                NodeType::Constant {
                    value: ConstantValue::Integer(0),
                },
                Pos2::new(100.0, 200.0),
            );
        }
        if ui.button("+ Arithmetic").clicked() {
            state.add_node(
                NodeType::Arithmetic {
                    op: ArithmeticOp::Add,
                },
                Pos2::new(100.0, 300.0),
            );
        }
        if ui.button("+ Host Call").clicked() {
            state.add_node(
                NodeType::HostCall {
                    name: "print".to_string(),
                },
                Pos2::new(100.0, 400.0),
            );
        }
        if ui.button("+ Output").clicked() {
            state.add_node(NodeType::Output, Pos2::new(450.0, 100.0));
        }
        if ui
            .add_enabled(
                state.selected_node.is_some(),
                egui::Button::new("Edit Selected"),
            )
            .clicked()
        {
            state.editing_node = state.selected_node;
            state.editing_draft = state
                .selected_node
                .and_then(|id| state.graph.nodes.get(&id).cloned());
        }
        ui.separator();
        if ui.button("Compile to IR").clicked() {
            state.compile();
        }
        if let Some(result) = &state.compile_result {
            match result {
                Ok(_) => ui.label(RichText::new("✓ Compiled successfully").color(Color32::GREEN)),
                Err(e) => ui.label(RichText::new(format!("✗ {}", e)).color(Color32::RED)),
            };
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("Zoom: {:.0}%", state.zoom * 100.0));
            if ui.button("Reset View").clicked() {
                state.pan_offset = Pos2::ZERO;
                state.zoom = 1.0;
            }
        });
    });

    ui.add_space(4.0);

    // Graph canvas
    let available = ui.available_rect_before_wrap();
    let (canvas_rect, response) = ui.allocate_exact_size(available.size(), Sense::click_and_drag());

    // Handle zoom with Ctrl+Scroll
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 && ui.input(|i| i.modifiers.ctrl) {
            let zoom_factor = (1.0 + scroll_delta * 0.001).clamp(0.1, 5.0);
            state.zoom = (state.zoom * zoom_factor).clamp(0.1, 5.0);
        }
    }

    // Handle panning with middle mouse or Shift+Left drag
    if response.dragged() {
        let drag_delta = response.drag_delta();
        if ui.input(|i| i.pointer.middle_down() || (i.modifiers.shift && i.pointer.primary_down()))
        {
            state.pan_offset += drag_delta;
        }
    }

    // Right-click context menu for adding nodes
    if response.secondary_clicked() {
        state.show_add_menu = true;
        state.add_menu_pos = response.interact_pointer_pos().unwrap_or(Pos2::ZERO);
    }

    // Draw grid background
    draw_grid(ui.painter(), canvas_rect, state);

    // Draw connections
    let node_rects = draw_connections(ui.painter(), state, canvas_rect);

    // Draw nodes
    draw_nodes(ui, state, canvas_rect, &node_rects);

    // Draw preview connection being created
    if let Some(source_pin_id) = state.creating_connection {
        let mouse_pos = ui.ctx().input(|i| i.pointer.hover_pos());
        draw_preview_connection(ui.painter(), state, canvas_rect, source_pin_id, mouse_pos);
    }

    // Handle node interactions
    if ui.is_enabled() {
        handle_node_interactions(ui, state, canvas_rect, &node_rects, &response);
    }

    // Show context menu for adding nodes
    if state.show_add_menu && ui.is_enabled() {
        show_add_node_menu(ui.ctx(), state, canvas_rect);
    }

    // Show node parameter editor if a node is being edited
    if let Some(node_id) = state.editing_node.filter(|_| ui.is_enabled()) {
        show_node_editor(ui.ctx(), state, node_id);
    }
}

/// Draw grid background.
fn draw_grid(painter: &egui::Painter, canvas_rect: egui::Rect, state: &GraphEditorState) {
    let grid_size = GRID_SIZE * state.zoom;
    let offset_x = (state.pan_offset.x % grid_size + grid_size) % grid_size;
    let offset_y = (state.pan_offset.y % grid_size + grid_size) % grid_size;

    let start_x = canvas_rect.left() - offset_x;
    let start_y = canvas_rect.top() - offset_y;

    // Vertical lines
    let mut x = start_x;
    while x <= canvas_rect.right() {
        painter.line_segment(
            [
                Pos2::new(x, canvas_rect.top()),
                Pos2::new(x, canvas_rect.bottom()),
            ],
            Stroke::new(0.5_f32, GRID_COLOR),
        );
        x += grid_size;
    }

    // Horizontal lines
    let mut y = start_y;
    while y <= canvas_rect.bottom() {
        painter.line_segment(
            [
                Pos2::new(canvas_rect.left(), y),
                Pos2::new(canvas_rect.right(), y),
            ],
            Stroke::new(0.5_f32, GRID_COLOR),
        );
        y += grid_size;
    }
}

/// Get world-space position from canvas position.
fn canvas_to_world(pos: Pos2, canvas_rect: egui::Rect, state: &GraphEditorState) -> Pos2 {
    Pos2::new(
        (pos.x - canvas_rect.left() - state.pan_offset.x) / state.zoom,
        (pos.y - canvas_rect.top() - state.pan_offset.y) / state.zoom,
    )
}

/// Get canvas position from world-space position.
fn world_to_canvas(pos: Pos2, canvas_rect: egui::Rect, state: &GraphEditorState) -> Pos2 {
    Pos2::new(
        canvas_rect.left() + state.pan_offset.x + pos.x * state.zoom,
        canvas_rect.top() + state.pan_offset.y + pos.y * state.zoom,
    )
}

/// Draw all connections.
fn draw_connections(
    painter: &egui::Painter,
    state: &GraphEditorState,
    canvas_rect: egui::Rect,
) -> Vec<(uuid::Uuid, egui::Rect)> {
    let mut node_rects = Vec::new();

    // First collect all node rects
    for (node_id, node) in &state.graph.nodes {
        let pos = world_to_canvas(
            Pos2::new(node.position.0, node.position.1),
            canvas_rect,
            state,
        );
        let width = NODE_WIDTH * state.zoom;
        let height = (NODE_HEIGHT + node.pins.len() as f32 * 20.0).max(NODE_HEIGHT) * state.zoom;
        let rect = egui::Rect::from_min_size(pos, Vec2::new(width, height));
        node_rects.push((*node_id, rect));
    }

    // Draw connections
    for conn in &state.graph.connections {
        let source_pin = state
            .graph
            .nodes
            .values()
            .find_map(|n| n.pins.iter().find(|p| p.id == conn.source_pin));
        let target_pin = state
            .graph
            .nodes
            .values()
            .find_map(|n| n.pins.iter().find(|p| p.id == conn.target_pin));
        let source_node = state
            .graph
            .nodes
            .values()
            .find(|n| n.pins.iter().any(|p| p.id == conn.source_pin));
        let target_node = state
            .graph
            .nodes
            .values()
            .find(|n| n.pins.iter().any(|p| p.id == conn.target_pin));

        if let (Some(source_node), Some(target_node), Some(_source_pin), Some(_target_pin)) =
            (source_node, target_node, source_pin, target_pin)
        {
            let source_pos = world_to_canvas(
                Pos2::new(source_node.position.0, source_node.position.1),
                canvas_rect,
                state,
            );
            let target_pos = world_to_canvas(
                Pos2::new(target_node.position.0, target_node.position.1),
                canvas_rect,
                state,
            );

            let source_pin_idx = source_node
                .pins
                .iter()
                .position(|p| p.id == conn.source_pin)
                .unwrap_or(0);
            let target_pin_idx = target_node
                .pins
                .iter()
                .position(|p| p.id == conn.target_pin)
                .unwrap_or(0);

            let start = Pos2::new(
                source_pos.x + NODE_WIDTH * state.zoom,
                source_pos.y + 24.0 * state.zoom + source_pin_idx as f32 * 20.0 * state.zoom,
            );
            let end = Pos2::new(
                target_pos.x,
                target_pos.y + 24.0 * state.zoom + target_pin_idx as f32 * 20.0 * state.zoom,
            );

            // Draw curved bezier connection
            let mid_x = (start.x + end.x) * 0.5;
            let ctrl1 = Pos2::new(mid_x, start.y);
            let ctrl2 = Pos2::new(mid_x, end.y);

            let path = egui::Shape::line_segment(
                [start, ctrl1],
                Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
            );
            painter.add(path);
            painter.add(egui::Shape::line_segment(
                [ctrl1, ctrl2],
                Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
            ));
            painter.add(egui::Shape::line_segment(
                [ctrl2, end],
                Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
            ));

            // Draw arrow at target
            let dir = (end - start).normalized();
            let arrow_size = 8.0 * state.zoom;
            let arrow_base = end - dir * arrow_size;
            let perp = Pos2::new(-dir.y, dir.x) * arrow_size * 0.5;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    end,
                    arrow_base + perp.to_vec2(),
                    arrow_base - perp.to_vec2(),
                ],
                CONNECTION_COLOR,
                egui::Stroke::NONE,
            ));
        }
    }

    node_rects
}

/// Draw all nodes.
fn draw_nodes(
    ui: &mut egui::Ui,
    state: &mut GraphEditorState,
    _canvas_rect: egui::Rect,
    node_rects: &[(uuid::Uuid, egui::Rect)],
) {
    let painter = ui.painter();
    for (node_id, rect) in node_rects {
        if let Some(node) = state.graph.nodes.get(node_id) {
            let is_selected = state.selected_node == Some(*node_id);
            let border_color = if is_selected {
                NODE_SELECTED_BORDER
            } else {
                NODE_BORDER_COLOR
            };

            // Node background
            painter.rect_filled(*rect, 4.0, NODE_BG_COLOR);
            painter.rect_stroke(*rect, 4.0, Stroke::new(1.5 * state.zoom, border_color));

            // Title bar
            let title_rect = egui::Rect::from_min_max(
                rect.min,
                Pos2::new(rect.max.x, rect.min.y + 24.0 * state.zoom),
            );
            painter.rect_filled(title_rect, 4.0, Color32::from_rgb(60, 60, 75));
            painter.rect_stroke(title_rect, 4.0, Stroke::new(1.0_f32, border_color));

            // Node type name
            let type_name = node_type_name(&node.node_type);
            painter.text(
                title_rect.center(),
                egui::Align2::CENTER_CENTER,
                type_name,
                TextStyle::Button.resolve(ui.style()),
                Color32::WHITE,
            );

            // Draw pins
            draw_node_pins(painter, node, *rect, state.zoom);
        }
    }
}

/// Draw pins on a node.
fn draw_node_pins(painter: &egui::Painter, node: &Node, rect: egui::Rect, zoom: f32) {
    let mut y = rect.top() + 24.0 * zoom;
    for pin in &node.pins {
        let pin_pos = match pin.direction {
            PinDirection::Input => Pos2::new(rect.left(), y),
            PinDirection::Output => Pos2::new(rect.right(), y),
        };

        // Pin circle
        let pin_color = pin_type_color(pin.pin_type);
        painter.circle_filled(pin_pos, PIN_RADIUS * zoom, pin_color);
        painter.circle_stroke(
            pin_pos,
            PIN_RADIUS * zoom,
            Stroke::new(1.0 * zoom, NODE_BORDER_COLOR),
        );

        // Pin label
        let label_pos = match pin.direction {
            PinDirection::Input => Pos2::new(pin_pos.x + PIN_RADIUS * zoom + 4.0, y),
            PinDirection::Output => Pos2::new(pin_pos.x - PIN_RADIUS * zoom - 4.0, y),
        };
        let galign = match pin.direction {
            PinDirection::Input => egui::Align2::LEFT_CENTER,
            PinDirection::Output => egui::Align2::RIGHT_CENTER,
        };
        painter.text(
            label_pos,
            galign,
            &pin.name,
            egui::FontId::proportional(11.0 * zoom),
            Color32::LIGHT_GRAY,
        );

        y += 20.0 * zoom;
    }
}

/// Get color for pin type.
fn pin_type_color(pin_type: PinType) -> Color32 {
    match pin_type {
        PinType::Value => Color32::from_rgb(200, 200, 200),
        PinType::Event => Color32::from_rgb(255, 200, 100),
        PinType::Integer => Color32::from_rgb(100, 200, 255),
        PinType::Float => Color32::from_rgb(150, 255, 150),
        PinType::Boolean => Color32::from_rgb(255, 150, 255),
        PinType::String => Color32::from_rgb(255, 255, 150),
    }
}

/// Draw preview connection being created.
fn draw_preview_connection(
    painter: &egui::Painter,
    state: &GraphEditorState,
    canvas_rect: egui::Rect,
    source_pin_id: uuid::Uuid,
    mouse_pos: Option<Pos2>,
) {
    if let Some(source_node) = state
        .graph
        .nodes
        .values()
        .find(|n| n.pins.iter().any(|p| p.id == source_pin_id))
    {
        if let Some(_source_pin) = source_node.pins.iter().find(|p| p.id == source_pin_id) {
            let source_pos = world_to_canvas(
                Pos2::new(source_node.position.0, source_node.position.1),
                canvas_rect,
                state,
            );
            let source_pin_idx = source_node
                .pins
                .iter()
                .position(|p| p.id == source_pin_id)
                .unwrap_or(0);
            let start = Pos2::new(
                source_pos.x + NODE_WIDTH * state.zoom,
                source_pos.y + 24.0 * state.zoom + source_pin_idx as f32 * 20.0 * state.zoom,
            );

            if let Some(mouse_pos) = mouse_pos {
                let mid_x = (start.x + mouse_pos.x) * 0.5;
                let ctrl1 = Pos2::new(mid_x, start.y);
                let ctrl2 = Pos2::new(mid_x, mouse_pos.y);

                painter.add(egui::Shape::line_segment(
                    [start, ctrl1],
                    Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
                ));
                painter.add(egui::Shape::line_segment(
                    [ctrl1, ctrl2],
                    Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
                ));
                painter.add(egui::Shape::line_segment(
                    [ctrl2, mouse_pos],
                    Stroke::new(CONNECTION_WIDTH, CONNECTION_COLOR),
                ));
            }
        }
    }
}

/// Handle node interactions (selection, dragging, connection creation).
fn handle_node_interactions(
    ui: &mut egui::Ui,
    state: &mut GraphEditorState,
    _canvas_rect: egui::Rect,
    node_rects: &[(uuid::Uuid, egui::Rect)],
    response: &Response,
) {
    let mut clicked_node: Option<uuid::Uuid> = None;

    for (node_id, rect) in node_rects {
        let node_response = ui.interact(*rect, ui.id().with(node_id), Sense::click_and_drag());

        if node_response.clicked() {
            clicked_node = Some(*node_id);
        }
        if node_response.dragged() && !ui.input(|input| input.modifiers.shift) {
            state.dragging = Some((*node_id, Pos2::ZERO));
            if let Some(node) = state.graph.nodes.get_mut(node_id) {
                node.position.0 = (node.position.0 + node_response.drag_delta().x / state.zoom)
                    .clamp(-1_000_000.0, 1_000_000.0);
                node.position.1 = (node.position.1 + node_response.drag_delta().y / state.zoom)
                    .clamp(-1_000_000.0, 1_000_000.0);
                state.dirty = true;
            }
        }

        // Double-click to edit node parameters
        if node_response.double_clicked() {
            state.editing_node = Some(*node_id);
            state.editing_draft = state.graph.nodes.get(node_id).cloned();
        }

        // Pin interaction
        let mut connection_to_add: Option<(Uuid, Uuid)> = None;
        if let Some(node) = state.graph.nodes.get(node_id) {
            for pin in &node.pins {
                let pin_pos = match pin.direction {
                    PinDirection::Input => Pos2::new(
                        rect.left(),
                        rect.top()
                            + 24.0 * state.zoom
                            + node.pins.iter().position(|p| p.id == pin.id).unwrap_or(0) as f32
                                * 20.0
                                * state.zoom,
                    ),
                    PinDirection::Output => Pos2::new(
                        rect.right(),
                        rect.top()
                            + 24.0 * state.zoom
                            + node.pins.iter().position(|p| p.id == pin.id).unwrap_or(0) as f32
                                * 20.0
                                * state.zoom,
                    ),
                };
                let pin_rect = egui::Rect::from_center_size(
                    pin_pos,
                    Vec2::splat(PIN_RADIUS * 2.0 * state.zoom),
                );
                let pin_response =
                    ui.interact(pin_rect, ui.id().with(pin.id), Sense::click_and_drag());

                if pin_response.clicked() {
                    if pin.direction == PinDirection::Output {
                        state.creating_connection = Some(pin.id);
                    } else if let Some(source_pin_id) = state.creating_connection {
                        if state.can_connect(source_pin_id, pin.id) {
                            connection_to_add = Some((source_pin_id, pin.id));
                        }
                        state.creating_connection = None;
                    }
                }
                if pin_response.drag_started() && pin.direction == PinDirection::Output {
                    state.creating_connection = Some(pin.id);
                }
            }
        }

        if let Some((source_pin_id, pin_id)) = connection_to_add {
            state.add_connection(source_pin_id, pin_id);
        }
    }
    if let Some(id) = clicked_node {
        state.selected_node = Some(id);
    }

    // If clicked on empty space, deselect
    if response.clicked() && clicked_node.is_none() && state.creating_connection.is_none() {
        state.selected_node = None;
    }

    // Cancel connection creation on escape or right click elsewhere
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.creating_connection = None;
    }
}

/// Show context menu for adding nodes (right-click on canvas).
fn show_add_node_menu(ctx: &egui::Context, state: &mut GraphEditorState, canvas_rect: egui::Rect) {
    egui::Window::new("Add Node")
        .fixed_pos(state.add_menu_pos)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            // Search filter
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut state.search_filter);
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    let filter = state.search_filter.to_lowercase();

                    for (category, nodes) in node_types_by_category() {
                        let nodes = nodes
                            .into_iter()
                            .filter(|(kind, _)| {
                                matches!(
                                    kind,
                                    NodeType::Event { .. }
                                        | NodeType::Constant { .. }
                                        | NodeType::Arithmetic { .. }
                                        | NodeType::Comparison { .. }
                                        | NodeType::Branch
                                        | NodeType::HostCall { .. }
                                        | NodeType::Output
                                )
                            })
                            .collect::<Vec<_>>();
                        if nodes.is_empty() {
                            continue;
                        }

                        let mut has_match = false;
                        for (_, name) in &nodes {
                            if filter.is_empty() || name.to_lowercase().contains(&filter) {
                                has_match = true;
                                break;
                            }
                        }
                        if !has_match && !filter.is_empty() {
                            continue;
                        }

                        ui.collapsing(category_name(category), |ui| {
                            for (node_type, name) in nodes {
                                if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                                    continue;
                                }
                                if ui.button(name).clicked() {
                                    let world_pos =
                                        canvas_to_world(state.add_menu_pos, canvas_rect, state);
                                    state.add_node(node_type, world_pos);
                                    state.show_add_menu = false;
                                }
                            }
                        });
                    }
                });

            ui.separator();
            if ui.button("Close").clicked() {
                state.show_add_menu = false;
            }
        });

    // Close menu if clicked elsewhere
    if ctx.input(|i| i.pointer.any_click())
        && !ctx.memory(|m| m.area_rect(egui::Id::new("Add Node")).is_some())
    {
        state.show_add_menu = false;
    }
}

/// Show node parameter editor (double-click node).
fn show_node_editor(ctx: &egui::Context, state: &mut GraphEditorState, node_id: uuid::Uuid) {
    let Some(mut node) = state
        .editing_draft
        .clone()
        .or_else(|| state.graph.nodes.get(&node_id).cloned())
    else {
        return;
    };

    let mut action = None;
    egui::Window::new(format!("Edit {}", node_type_name(&node.node_type)))
        .id(egui::Id::new(("graph_node_editor", node_id)))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            match &mut node.node_type {
                NodeType::Event { name } => {
                    ui.label("Event Name:");
                    ui.text_edit_singleline(name);
                }
                NodeType::Constant { value } => {
                    let previous = match value {
                        ConstantValue::Integer(_) => 0,
                        ConstantValue::Float(_) => 1,
                        ConstantValue::Boolean(_) => 2,
                        ConstantValue::String(_) => 3,
                    };
                    let mut kind = previous;
                    egui::ComboBox::from_label("Type")
                        .selected_text(["Integer", "Float", "Boolean", "String"][kind])
                        .show_ui(ui, |ui| {
                            for (index, name) in
                                ["Integer", "Float", "Boolean", "String"].iter().enumerate()
                            {
                                ui.selectable_value(&mut kind, index, *name);
                            }
                        });
                    if kind != previous {
                        *value = match kind {
                            0 => ConstantValue::Integer(0),
                            1 => ConstantValue::Float(0.0),
                            2 => ConstantValue::Boolean(false),
                            _ => ConstantValue::String(String::new()),
                        };
                    }
                    ui.label("Value:");
                    match value {
                        ConstantValue::Integer(v) => {
                            ui.add(egui::DragValue::new(v).speed(1));
                        }
                        ConstantValue::Float(v) => {
                            ui.add(egui::DragValue::new(v).speed(0.1));
                        }
                        ConstantValue::Boolean(v) => {
                            ui.checkbox(v, "");
                        }
                        ConstantValue::String(v) => {
                            ui.text_edit_singleline(v);
                        }
                    }
                }
                NodeType::Arithmetic { op } => {
                    ui.label("Operation:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", op))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(op, ArithmeticOp::Add, "Add");
                            ui.selectable_value(op, ArithmeticOp::Sub, "Subtract");
                            ui.selectable_value(op, ArithmeticOp::Mul, "Multiply");
                            ui.selectable_value(op, ArithmeticOp::Div, "Divide");
                        });
                }
                NodeType::Comparison { op } => {
                    ui.label("Comparison:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", op))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(op, ComparisonOp::Equal, "Equal");
                            ui.selectable_value(op, ComparisonOp::LessThan, "Less Than");
                        });
                }
                NodeType::HostCall { name } => {
                    ui.label("Function:");
                    egui::ComboBox::from_label("")
                        .selected_text(name.as_str())
                        .show_ui(ui, |ui| {
                            for (fname, _, _) in kiln_runtime::slice::BUILTINS {
                                ui.selectable_value(name, fname.to_string(), *fname);
                            }
                        });
                }
                NodeType::AnimationClip { clip_name } => {
                    ui.label("Clip Name:");
                    ui.text_edit_singleline(clip_name);
                }
                NodeType::AnimationState { state_name } => {
                    ui.label("State Name:");
                    ui.text_edit_singleline(state_name);
                }
                NodeType::BehaviorAction { name } => {
                    ui.label("Action Name:");
                    ui.text_edit_singleline(name);
                }
                NodeType::AIState { state_name } => {
                    ui.label("State Name:");
                    ui.text_edit_singleline(state_name);
                }
                NodeType::Dialogue { dialogue_id } => {
                    ui.label("Dialogue ID:");
                    ui.text_edit_singleline(dialogue_id);
                }
                NodeType::QuestStart { quest_id }
                | NodeType::QuestComplete { quest_id }
                | NodeType::QuestObjectiveUpdate { quest_id, .. } => {
                    ui.label("Quest ID:");
                    ui.text_edit_singleline(quest_id);
                }
                NodeType::DataTableLookup { table_id, .. }
                | NodeType::DataTableRowGet { table_id } => {
                    ui.label("Table ID:");
                    ui.text_edit_singleline(table_id);
                    if let NodeType::DataTableLookup { column_name, .. } = &mut node.node_type {
                        ui.label("Column Name:");
                        ui.text_edit_singleline(column_name);
                    }
                }
                _ => {
                    ui.label("No editable parameters for this node type.");
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    action = Some(0);
                }
                if ui.button("Cancel").clicked() {
                    action = Some(1);
                }
                if ui.button("Delete Node").clicked() {
                    action = Some(2);
                }
            });
        });
    match action {
        Some(0) => {
            node.refresh_pins();
            state.graph.nodes.insert(node_id, node.clone());
            let pins = state
                .graph
                .nodes
                .values()
                .flat_map(|n| n.pins.iter().map(|p| p.id))
                .collect::<std::collections::HashSet<_>>();
            state
                .graph
                .connections
                .retain(|c| pins.contains(&c.source_pin) && pins.contains(&c.target_pin));
            state.dirty = true;
        }
        Some(2) => {
            state.graph.remove_node(node_id);
            state.dirty = true;
        }
        _ => {}
    }
    if action.is_some() {
        state.editing_node = None;
        state.editing_draft = None;
    } else {
        state.editing_draft = Some(node);
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

#[cfg(test)]
mod parameter_window_tests {
    use super::*;

    #[test]
    fn node_parameters_stay_above_the_floating_graph_when_its_parent_is_raised() {
        let ctx = egui::Context::default();
        crate::editor_shell::apply_theme(&ctx);
        let mut state = GraphEditorState::new();
        let node = state.add_node(
            NodeType::Constant {
                value: ConstantValue::Integer(5),
            },
            Pos2::new(100.0, 200.0),
        );
        state.editing_node = Some(node);
        let parent = egui::Id::new("floating_graph_test");
        for _ in 0..4 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(1440.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::Window::new("Graph")
                        .id(parent)
                        .fixed_rect(egui::Rect::from_min_max(
                            egui::pos2(20.0, 20.0),
                            egui::pos2(1400.0, 880.0),
                        ))
                        .show(ctx, |ui| render_graph_editor(ui, &mut state));
                    // Model the containing dock regaining focus after a click.
                    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, parent));
                },
            );
        }
        let layer = ctx.layer_id_at(egui::pos2(720.0, 450.0)).unwrap();
        assert_eq!(layer.id, egui::Id::new(("graph_node_editor", node)));
        assert_eq!(state.editing_node, Some(node));
    }
}
