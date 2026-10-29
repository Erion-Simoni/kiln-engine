//! Scene hierarchy panel — tree view of all entities in the open scene.
//!
//! Displays entities as a tree, allows selection, renaming, and basic
//! component management. Selected entities are highlighted and their
//! components are shown in the inspector.

use hecs::Entity;
use kiln_core::project::{Component, Project, Scene};
use kiln_runtime::world::Role;

/// Action requested by the hierarchy panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyAction {
    /// User wants to add a new entity with the given name.
    AddEntity(String),
    /// User wants to remove the entity with the given project ID.
    RemoveEntity(String),
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
    }

    /// Filter nodes by name (case-insensitive substring match).
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

    ui.heading("Hierarchy");

    // Filter bar
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.filter);
        if ui.button("Clear").clicked() {
            state.filter = "".to_string();
        }
    });

    ui.separator();

    // Entity tree
    egui::ScrollArea::vertical().show(ui, |ui| {
        let nodes: Vec<HierarchyNode> = state.filtered_nodes().iter().cloned().cloned().collect();
        for node in nodes {
            let selected = state.selected_id.as_deref() == Some(&node.entity_id);
            let node_text = if node.has_children {
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
                if ui
                    .selectable_label(selected, &node.name)
                    .clicked()
                {
                    state.select(&node.entity_id);
                    action = Some(HierarchyAction::SelectEntity(node.entity_id.clone()));
                }

                // Role badge
                if let Some(role) = get_entity_role(project, scene_id, &node.entity_id) {
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 100), format!("[{}]", role));
                }
            });
        }
    });

    ui.separator();

    // Add entity button
    ui.horizontal(|ui| {
        if ui.button("+ Add Entity").clicked() {
            state.show_add_dialog = true;
        }
        if ui.button("- Remove Selected").clicked() {
            if let Some(id) = &state.selected_id {
                action = Some(HierarchyAction::RemoveEntity(id.clone()));
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
                            action = Some(HierarchyAction::AddEntity(state.new_entity_name.clone()));
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