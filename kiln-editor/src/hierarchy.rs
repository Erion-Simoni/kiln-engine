//! Scene hierarchy panel — tree view of all entities in the open scene.
//!
//! Displays entities as a tree, allows selection, renaming, and basic
//! component management. Selected entities are highlighted and their
//! components are shown in the inspector.

use kiln_core::project::{Component, Project};
use std::collections::HashMap;

/// Action requested by the hierarchy panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyAction {
    SetEditorFlags {
        entity_id: String,
        locked: bool,
        hidden: bool,
    },
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
    /// Whether the node has parented entities below it.
    pub has_children: bool,
    /// Entity-parent depth in the displayed preorder tree.
    pub depth: usize,
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
    scene_key: Option<(String, String)>,
}

impl HierarchyPanel {
    /// Create a new empty hierarchy panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the hierarchy from a project scene.
    pub fn rebuild(&mut self, project: &Project, scene_id: &str) {
        let key = (project.id.clone(), scene_id.to_owned());
        let expanded: HashMap<_, _> = if self.scene_key.as_ref() == Some(&key) {
            self.nodes
                .iter()
                .map(|n| (n.entity_id.clone(), n.expanded))
                .collect()
        } else {
            HashMap::new()
        };
        self.scene_key = Some(key);
        self.nodes.clear();
        if let Some(scene) = project.scenes.get(scene_id) {
            let indices: HashMap<_, _> = scene
                .entities
                .iter()
                .enumerate()
                .map(|(index, entity)| (entity.id.as_str(), index))
                .collect();
            let mut children = vec![Vec::new(); scene.entities.len()];
            let mut roots = Vec::new();
            for (index, entity) in scene.entities.iter().enumerate() {
                let parent = entity
                    .components
                    .iter()
                    .find_map(|component| match component {
                        Component::Parent { parent } => indices.get(parent.as_str()).copied(),
                        _ => None,
                    })
                    .filter(|parent| *parent != index);
                if let Some(parent) = parent {
                    children[parent].push(index);
                } else {
                    roots.push(index);
                }
            }
            let mut visited = vec![false; scene.entities.len()];
            // Also visit cyclic components without recursion, so malformed data
            // cannot hide entities or cause an unbounded recursive traversal.
            for root in roots.into_iter().chain(0..scene.entities.len()) {
                let mut stack = vec![(root, 0)];
                while let Some((index, depth)) = stack.pop() {
                    if visited[index] {
                        continue;
                    }
                    visited[index] = true;
                    let entity = &scene.entities[index];
                    self.nodes.push(HierarchyNode {
                        entity_id: entity.id.clone(),
                        name: entity.name.clone(),
                        expanded: expanded.get(&entity.id).copied().unwrap_or(true),
                        selected: self.selected_ids.contains(&entity.id),
                        has_children: !children[index].is_empty(),
                        depth,
                    });
                    stack.extend(
                        children[index]
                            .iter()
                            .rev()
                            .map(|index| (*index, depth + 1)),
                    );
                }
            }
        }
    }

    /// Toggle a branch without changing selection or the authored document.
    pub fn toggle_expanded(&mut self, entity_id: &str) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|n| n.entity_id == entity_id && n.has_children)
        {
            node.expanded = !node.expanded;
        }
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

    /// Select a pasted group while retaining the first entity as inspector target.
    pub fn select_many(&mut self, ids: &[String]) {
        self.selected_ids = ids.to_vec();
        self.selected_id = ids.first().cloned();
        for node in &mut self.nodes {
            node.selected = self.selected_ids.contains(&node.entity_id);
        }
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
            let mut hidden_below = None;
            return self
                .nodes
                .iter()
                .filter(|node| {
                    if hidden_below.is_some_and(|depth| node.depth > depth) {
                        return false;
                    }
                    hidden_below = (!node.expanded && node.has_children).then_some(node.depth);
                    true
                })
                .collect();
        }
        let filter = self.filter.to_lowercase();
        let mut visible: Vec<_> = self
            .nodes
            .iter()
            .map(|n| n.name.to_lowercase().contains(&filter))
            .collect();
        let mut ancestors = Vec::new();
        let mut parents = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            ancestors.truncate(node.depth);
            parents.push(ancestors.last().copied());
            ancestors.push(index);
        }
        for index in (0..self.nodes.len()).rev() {
            if visible[index] {
                if let Some(parent) = parents[index] {
                    visible[parent] = true;
                }
            }
        }
        self.nodes
            .iter()
            .zip(visible)
            .filter_map(|(node, visible)| visible.then_some(node))
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
    let flags = project
        .scenes
        .get(scene_id)
        .map(crate::scene_controls::resolved_flags)
        .unwrap_or_default();

    ui.add(
        egui::TextEdit::singleline(&mut state.filter)
            .hint_text("Search hierarchy")
            .desired_width(f32::INFINITY),
    );
    let root = ui.weak(format!("SCENE  {scene_id}")).on_hover_text(
        "Drop an entity here to move it to the scene root, preserving its world transform",
    );
    if let Some(source) = root.dnd_release_payload::<String>() {
        action = Some(HierarchyAction::Reparent {
            child_id: source.as_str().to_owned(),
            parent_id: None,
        });
    }
    ui.separator();

    // Entity tree
    egui::ScrollArea::vertical().show(ui, |ui| {
        let nodes: Vec<HierarchyNode> = state.filtered_nodes().into_iter().cloned().collect();
        for node in nodes {
            let selected = state.selected_ids.contains(&node.entity_id);
            ui.horizontal(|ui| {
                ui.add_space(node.depth.min(8) as f32 * 14.0);
                // Expand/collapse an entity's descendants.
                if node.has_children {
                    let expanded = node.expanded;
                    let response = ui.add_sized([16.0, 22.0], egui::Button::new("").frame(false));
                    let center = response.rect.center();
                    let offsets = if expanded {
                        [
                            egui::vec2(-4.0, -2.0),
                            egui::vec2(4.0, -2.0),
                            egui::vec2(0.0, 3.0),
                        ]
                    } else {
                        [
                            egui::vec2(-2.0, -4.0),
                            egui::vec2(3.0, 0.0),
                            egui::vec2(-2.0, 4.0),
                        ]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(
                        offsets.map(|offset| center + offset).to_vec(),
                        ui.visuals().text_color(),
                        egui::Stroke::NONE,
                    ));
                    if response.clicked() {
                        state.toggle_expanded(&node.entity_id);
                    }
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
                response.context_menu(|ui| {
                    let own = project
                        .scenes
                        .get(scene_id)
                        .map(|s| crate::scene_controls::own_flags(s, &node.entity_id))
                        .unwrap_or_default();
                    if ui
                        .button(if own.locked {
                            "Unlock entity"
                        } else {
                            "Lock entity"
                        })
                        .clicked()
                    {
                        action = Some(HierarchyAction::SetEditorFlags {
                            entity_id: node.entity_id.clone(),
                            locked: !own.locked,
                            hidden: own.hidden,
                        });
                        ui.close_menu();
                    }
                    if ui
                        .button(if own.hidden {
                            "Show in editor"
                        } else {
                            "Hide in editor"
                        })
                        .clicked()
                    {
                        action = Some(HierarchyAction::SetEditorFlags {
                            entity_id: node.entity_id.clone(),
                            locked: own.locked,
                            hidden: !own.hidden,
                        });
                        ui.close_menu();
                    }
                    if let Some(flags) = flags.get(&node.entity_id) {
                        if flags.locked {
                            ui.weak("Locked (including inherited locks)");
                        }
                        if flags.hidden {
                            ui.weak("Hidden in edit mode; visible in Play");
                        }
                    }
                });
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
    use kiln_core::Scene;

    fn tree_project() -> Project {
        let mut project = Project::default();
        let entities = [
            ("grandchild", Some("child")),
            ("parent", None),
            ("child", Some("parent")),
            ("other", None),
        ]
        .into_iter()
        .map(|(id, parent)| kiln_core::Entity {
            id: id.into(),
            name: id.into(),
            components: parent
                .map(|parent| Component::Parent {
                    parent: parent.into(),
                })
                .into_iter()
                .collect(),
        })
        .collect();
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![],
                entities,
            },
        );
        project
    }

    #[test]
    fn parent_order_fold_and_search_context_follow_authored_entity_links() {
        let project = tree_project();
        let mut panel = HierarchyPanel::new();
        panel.rebuild(&project, "main");
        let rows: Vec<_> = panel
            .nodes
            .iter()
            .map(|n| (n.entity_id.as_str(), n.depth, n.has_children))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("parent", 0, true),
                ("child", 1, true),
                ("grandchild", 2, false),
                ("other", 0, false)
            ]
        );
        panel.select("child");
        panel.toggle_expanded("parent");
        assert_eq!(
            panel
                .filtered_nodes()
                .iter()
                .map(|n| n.entity_id.as_str())
                .collect::<Vec<_>>(),
            ["parent", "other"]
        );
        panel.set_filter("GRAND");
        assert_eq!(
            panel
                .filtered_nodes()
                .iter()
                .map(|n| n.entity_id.as_str())
                .collect::<Vec<_>>(),
            ["parent", "child", "grandchild"]
        );
        panel.set_filter("");
        panel.rebuild(&project, "main");
        assert!(!panel.nodes[0].expanded);
        assert_eq!(panel.selected_node().unwrap().entity_id, "child");
        assert_eq!(panel.filtered_nodes().len(), 2);
        panel.toggle_expanded("parent");
        assert_eq!(panel.filtered_nodes().len(), 4);
    }

    #[test]
    fn switching_scene_does_not_inherit_a_matching_entitys_fold_state() {
        let mut project = tree_project();
        let mut other = project.scenes["main"].clone();
        other.id = "other".into();
        project.scenes.insert(other.id.clone(), other);
        let mut panel = HierarchyPanel::new();
        panel.rebuild(&project, "main");
        panel.toggle_expanded("parent");
        panel.rebuild(&project, "other");
        assert_eq!(panel.filtered_nodes().len(), 4);
    }

    #[test]
    fn cyclic_and_missing_parent_data_remains_inspectable_without_recursion() {
        let mut project = tree_project();
        let scene = project.scenes.get_mut("main").unwrap();
        scene.entities[1].components.push(Component::Parent {
            parent: "grandchild".into(),
        });
        scene.entities[3].components.push(Component::Parent {
            parent: "missing".into(),
        });
        let mut panel = HierarchyPanel::new();
        panel.rebuild(&project, "main");
        let ids: std::collections::HashSet<_> =
            panel.nodes.iter().map(|n| n.entity_id.as_str()).collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(panel.filtered_nodes().len(), 4);
    }

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
