//! Scene hierarchy panel — tree view of all entities in the open scene.
//!
//! Displays entities as a tree, allows selection, renaming, and basic
//! component management. Selected entities are highlighted and their
//! components are shown in the inspector.

use kiln_core::project::{Component, Project, Scene};

/// Action requested by the hierarchy panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyAction {
    /// User wants to add a new entity with the given name.
    AddEntity(String),
    /// User wants to remove the entity with the given project ID.
    RemoveEntity(String),
    /// User wants to duplicate (copy) the entity with the given source id.
    DuplicateEntity(String),
    /// User wants to reparent `child_id` under `parent_id`.
    Reparent {
        child_id: String,
        parent_id: Option<String>,
    },
    /// User selected an entity (for syncing with inspector).
    SelectEntity(String),
}

/// A node in the hierarchy tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyNode {
    /// Entity id in the project data.
    pub entity_id: String,
    /// Display name.
    pub name: String,
    /// Whether the node is expanded (shows children).
    pub expanded: bool,
    /// Whether this node is selected.
    pub selected: bool,
    /// Whether the node has children (nested scenes).
    pub has_children: bool,
}

/// State of the hierarchy panel.
#[derive(Debug, Default)]
pub struct HierarchyPanel {
    /// All nodes in the hierarchy.
    pub nodes: Vec<HierarchyNode>,
    /// Currently selected entity id (if any).
    pub selected_id: Option<String>,
    /// All selected entity IDs, with `selected_id` remaining the inspector target.
    pub selected_ids: Vec<String>,
    /// Filter text for filtering nodes by name.
    pub filter: String,
    /// Whether to show the "add entity" dialog.
    pub show_add_dialog: bool,
    /// Name typed for the new entity.
    pub new_entity_name: String,
}

impl HierarchyPanel {
    /// Create a new empty hierarchy panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the hierarchy from a project scene.
    pub fn rebuild(&mut self, project: &Project, scene_id: &str) {
        self.nodes.clear();
        if let Some(scene) = project.scenes.get(scene_id) {
            for entity in &scene.entities {
                let has_children = self.scene_has_nested(project, scene, &entity.id);
                self.nodes.push(HierarchyNode {
                    entity_id: entity.id.clone(),
                    name: entity.name.clone(),
                    expanded: true,
                    selected: false,
                    has_children,
                });
            }
        }
    }

    /// Whether a scene entity has nested child scenes.
    fn scene_has_nested(&self, project: &Project, scene: &Scene, entity_id: &str) -> bool {
        for nested in &scene.nested_scenes {
            if let Some(child_scene) = project.scenes.get(&nested.scene_id) {
                if child_scene.entities.iter().any(|e| e.id == entity_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Get the selected node, if any.
    pub fn selected_node(&self) -> Option<&HierarchyNode> {
        self.nodes.iter().find(|n| n.selected)
    }

    /// Get the selected entity id, if any.
    pub fn selected_entity_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    /// Select an entity by id.
    pub fn select(&mut self, entity_id: &str) {
        self.selected_ids.clear();
        self.selected_ids.push(entity_id.to_owned());
        self.select_primary(entity_id);
    }

    /// Add or remove an entity from the current selection.
    pub fn toggle_select(&mut self, entity_id: &str) {
        if let Some(index) = self.selected_ids.iter().position(|id| id == entity_id) {
            self.selected_ids.remove(index);
            if self.selected_id.as_deref() == Some(entity_id) {
                self.selected_id = self.selected_ids.last().cloned();
            }
        } else {
            self.selected_ids.push(entity_id.to_owned());
            self.selected_id = Some(entity_id.to_owned());
        }
        for node in &mut self.nodes {
            node.selected = self.selected_ids.contains(&node.entity_id);
        }
    }

    fn select_primary(&mut self, entity_id: &str) {
        for node in &mut self.nodes {
            node.selected = node.entity_id == entity_id;
        }
        self.selected_id = Some(entity_id.to_string());
    }

    /// Clear selection.
    pub fn clear_selection(&mut self) {
        for node in &mut self.nodes {
            node.selected = false;
        }
        self.selected_id = None;
        self.selected_ids.clear();
    }

    /// Filter nodes by name (case-insensitive).
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
    }

    /// Get filtered nodes.
    pub fn filtered_nodes(&self) -> Vec<&HierarchyNode> {
        if self.filter.is_empty() {
            return self.nodes.iter().collect();
        }
        let filter = self.filter.to_lowercase();
        self.nodes
            .iter()
            .filter(|n| n.name.to_lowercase().contains(&filter))
            .collect()
    }
}

/// Render the hierarchy panel in an egui window.
pub fn render_hierarchy_panel(
    ui: &mut egui::Ui,
    state: &mut HierarchyPanel,
    project: &mut Project,
    scene_id: &str,
) -> Option<HierarchyAction> {
    let mut action = None;

    ui.add(
        egui::TextEdit::singleline(&mut state.filter)
            .hint_text("Search hierarchy")
            .desired_width(f32::INFINITY),
    );
    ui.weak(format!("SCENE  {scene_id}"));
    ui.separator();

    // Entity tree
    egui::ScrollArea::vertical().show(ui, |ui| {
        let nodes: Vec<HierarchyNode> = state.filtered_nodes().into_iter().cloned().collect();
        for node in nodes {
            let selected = state.selected_ids.contains(&node.entity_id);
            let _node_text = if node.has_children {
                format!("{} ▾", node.name)
            } else {
                node.name.clone()
            };

            ui.horizontal(|ui| {
                // Expand/collapse toggle (for nested scenes)
                if node.has_children {
                    let expanded = node.expanded;
                    let arrow = if expanded { "▼" } else { "▶" };
                    if ui.button(arrow).clicked() {
                        // Toggle expansion (handled by caller)
                    }
                    ui.label(arrow);
                } else {
                    ui.label("  ");
                }

                // Entity select button
                let response = ui
                    .add_sized(
                        [ui.available_width() - 50.0, 22.0],
                        egui::Button::new(&node.name)
                            .selected(selected)
                            .frame(false)
                            .sense(egui::Sense::click_and_drag()),
                    )
                    .on_hover_text(&node.entity_id);
                if response.drag_started() {
                    response.dnd_set_drag_payload(node.entity_id.clone());
                }

                // Reparent only when an entity ID is actually dropped here.
                if let Some(source) = response.dnd_release_payload::<String>() {
                    if source.as_str() != node.entity_id {
                        action = Some(HierarchyAction::Reparent {
                            child_id: source.as_str().to_owned(),
                            parent_id: Some(node.entity_id.clone()),
                        });
                    }
                }

                if response.clicked() {
                    let additive =
                        ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
                    if additive {
                        state.toggle_select(&node.entity_id);
                    } else {
                        state.select(&node.entity_id);
                    }
                    action = Some(HierarchyAction::SelectEntity(node.entity_id.clone()));
                }

                // Role badge
                if let Some(role) = get_entity_role(project, scene_id, &node.entity_id) {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 100),
                        format!("[{}]", role),
                    );
                }
            });
        }
    });

    ui.separator();

    // Add entity button
    ui.horizontal_wrapped(|ui| {
        if ui.button("+ Add Entity").clicked() {
            state.show_add_dialog = true;
        }
        if ui.button("Delete").clicked() {
            if let Some(id) = &state.selected_id {
                action = Some(HierarchyAction::RemoveEntity(id.clone()));
            }
        }
        if ui.button("Duplicate").clicked() {
            if let Some(id) = &state.selected_id {
                action = Some(HierarchyAction::DuplicateEntity(id.clone()));
            }
        }
    });

    // Add entity dialog
    if state.show_add_dialog {
        egui::Window::new("Add Entity")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Entity name:");
                ui.text_edit_singleline(&mut state.new_entity_name);
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if !state.new_entity_name.is_empty() {
                            action =
                                Some(HierarchyAction::AddEntity(state.new_entity_name.clone()));
                        }
                        state.show_add_dialog = false;
                        state.new_entity_name.clear();
                    }
                    if ui.button("Cancel").clicked() {
                        state.show_add_dialog = false;
                        state.new_entity_name.clear();
                    }
                });
            });
    }

    action
}

/// Get the role of an entity in a scene, if any.
fn get_entity_role(project: &Project, scene_id: &str, entity_id: &str) -> Option<String> {
    let scene = project.scenes.get(scene_id)?;
    let entity = scene.entities.iter().find(|e| e.id == entity_id)?;
    for component in &entity.components {
        if let Component::Role { role } = component {
            return Some(role.clone());
        }
    }
    None
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn hierarchy_click_selects_and_drag_reparents_through_the_real_widgets() {
        let ctx = egui::Context::default();
        crate::editor_shell::apply_theme(&ctx);
        let mut project = Project::default();
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![],
                entities: ["Player", "Box"]
                    .into_iter()
                    .map(|name| kiln_core::project::Entity {
                        id: name.into(),
                        name: name.into(),
                        components: vec![],
                    })
                    .collect(),
            },
        );
        let mut panel = HierarchyPanel::new();
        panel.rebuild(&project, "main");
        let mut frame = |events| {
            let mut action = None;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(240.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = render_hierarchy_panel(ui, &mut panel, &mut project, "main");
                    });
                },
            );
            (action, output)
        };
        fn text_center(shapes: &[egui::epaint::ClippedShape], name: &str) -> egui::Pos2 {
            fn find(shape: &egui::Shape, name: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.job.text == name => {
                        Some(text.pos + text.galley.size() * 0.5)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, name)),
                    _ => None,
                }
            }
            shapes
                .iter()
                .find_map(|shape| find(&shape.shape, name))
                .expect("Entity row text is visible")
        }
        frame(vec![]);
        let (_, output) = frame(vec![]);
        let player = text_center(&output.shapes, "Player");
        let target = text_center(&output.shapes, "Box");
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![egui::Event::PointerMoved(player)]);
        frame(vec![button(player, true)]);
        let (action, _) = frame(vec![button(player, false)]);
        assert_eq!(action, Some(HierarchyAction::SelectEntity("Player".into())));
        frame(vec![button(player, true)]);
        frame(vec![egui::Event::PointerMoved(target)]);
        frame(vec![]);
        let (action, _) = frame(vec![button(target, false)]);
        assert_eq!(
            action,
            Some(HierarchyAction::Reparent {
                child_id: "Player".into(),
                parent_id: Some("Box".into())
            })
        );
        assert_eq!(panel.selected_entity_id(), Some("Player"));
    }

    #[test]
    fn additive_selection_tracks_members_and_primary() {
        let mut panel = HierarchyPanel::new();
        panel.select("first");
        panel.toggle_select("second");
        assert_eq!(panel.selected_ids, ["first", "second"]);
        assert_eq!(panel.selected_entity_id(), Some("second"));

        panel.toggle_select("second");
        assert_eq!(panel.selected_ids, ["first"]);
        assert_eq!(panel.selected_entity_id(), Some("first"));

        panel.toggle_select("first");
        assert!(panel.selected_ids.is_empty());
        assert_eq!(panel.selected_entity_id(), None);
    }
}
