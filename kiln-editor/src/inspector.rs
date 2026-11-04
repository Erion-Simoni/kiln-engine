use kiln_core::project::{Component, Project};

/// Action requested by the inspector panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorAction {
    /// Add a new component of the given type.
    AddComponent(String),
    /// Remove the component at the given index.
    RemoveComponent(usize),
}

#[derive(Debug, Default)]
pub struct InspectorPanel {
    pub selected_entity_id: Option<String>,
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn render_inspector_panel(
    ui: &mut egui::Ui,
    state: &mut InspectorPanel,
    project: &mut Project,
    scene_id: &str,
) -> Option<InspectorAction> {
    let Some(entity_id) = state.selected_entity_id.as_ref() else {
        ui.heading("Inspector");
        ui.add_space(8.0);
        ui.label("Select an entity in the Hierarchy to edit it.");
        return None;
    };

    let metadata = project.clone();
    let scene = project.scenes.get_mut(scene_id)?;

    let entity = scene.entities.iter_mut().find(|e| e.id == *entity_id)?;

    ui.heading("Inspector");
    ui.add_space(8.0);

    // Entity name
    ui.horizontal(|ui| {
        ui.label("Entity:");
        ui.text_edit_singleline(&mut entity.name);
    });
    ui.add_space(8.0);

    // Components
    let mut remove_idx: Option<usize> = None;
    for (idx, component) in entity.components.iter_mut().enumerate() {
        ui.separator();
        let mut changed = false;

        egui::CollapsingHeader::new(component_header_name(component))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    changed |= render_component(ui, component, &metadata);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("✕")
                            .on_hover_text("Remove component")
                            .clicked()
                        {
                            remove_idx = Some(idx);
                        }
                    });
                });
            });
    }

    if let Some(idx) = remove_idx {
        return Some(InspectorAction::RemoveComponent(idx));
    }

    ui.separator();
    ui.add_space(4.0);

    // Add Component dropdown
    let mut add_component_action = None;
    ui.horizontal(|ui| {
        ui.label("Add Component:");
        egui::ComboBox::from_label("")
            .selected_text("Select component type...")
            .show_ui(ui, |ui| {
                for name in available_component_types() {
                    if ui.selectable_label(false, name).clicked() {
                        add_component_action =
                            Some(InspectorAction::AddComponent(name.to_string()));
                    }
                }
            });
    });
    if let Some(action) = add_component_action {
        return Some(action);
    }

    None
}

fn component_header_name(component: &Component) -> String {
    match component {
        Component::Script { .. } => "Script".into(),
        Component::Transform { .. } => "📍 Transform".to_string(),
        Component::Sprite { .. } => "🖼 Sprite".to_string(),
        Component::Tilemap { .. } => "🗺 Tilemap".to_string(),
        Component::Role { .. } => "🎭 Role".to_string(),
        Component::Parent { .. } => "🔗 Parent".to_string(),
        Component::PrefabInstance { .. } => "📦 Prefab Instance".to_string(),
    }
}

fn available_component_types() -> Vec<&'static str> {
    vec!["Transform", "Sprite", "Tilemap", "Role", "Parent"]
}

fn render_component(ui: &mut egui::Ui, component: &mut Component, project: &Project) -> bool {
    let mut changed = false;

    match component {
        Component::Script { path, properties } => {
            changed |= ui.text_edit_singleline(path).changed();
            for (key, value) in properties {
                ui.label(key);
                changed |= ui.add(egui::DragValue::new(value)).changed();
            }
        }
        Component::Transform {
            x,
            y,
            rotation,
            scale,
        } => {
            ui.add_space(4.0);
            ui.columns(2, |cols| {
                cols[0].label("Position X:");
                changed |= cols[1]
                    .add(egui::DragValue::new(x).speed(0.5).suffix(" px"))
                    .changed();
                cols[0].label("Position Y:");
                changed |= cols[1]
                    .add(egui::DragValue::new(y).speed(0.5).suffix(" px"))
                    .changed();
                cols[0].label("Rotation:");
                changed |= cols[1]
                    .add(egui::DragValue::new(rotation).speed(0.01).suffix(" rad"))
                    .changed();
                cols[0].label("Scale:");
                changed |= cols[1]
                    .add(egui::DragValue::new(scale).speed(0.01).range(0.001..=100.0))
                    .changed();
            });
        }
        Component::Sprite {
            color,
            texture,
            layer,
            material,
            animation_clip,
            ..
        } => {
            ui.add_space(4.0);

            // Color editor
            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color_arr = [color[0], color[1], color[2], color[3]];
                if ui
                    .color_edit_button_rgba_unmultiplied(&mut color_arr)
                    .changed()
                {
                    color[0] = color_arr[0].clamp(0.0, 1.0);
                    color[1] = color_arr[1].clamp(0.0, 1.0);
                    color[2] = color_arr[2].clamp(0.0, 1.0);
                    color[3] = color_arr[3].clamp(0.0, 1.0);
                    changed = true;
                }
            });
            // Sorting layer
            ui.horizontal(|ui| {
                ui.label("Layer:");
                changed |= ui.add(egui::DragValue::new(layer).speed(0.1)).changed();
            });

            // Texture picker
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Texture:");
                let current = texture.as_deref().unwrap_or("(none)");
                let mut tex_path = current.to_string();
                if ui.text_edit_singleline(&mut tex_path).changed() {
                    if tex_path.is_empty() || tex_path == "(none)" {
                        *texture = None;
                    } else {
                        *texture = Some(tex_path);
                    }
                    changed = true;
                }

                // Asset picker button
                if ui
                    .small_button("📁")
                    .on_hover_text("Browse assets")
                    .clicked()
                {
                    // TODO: Open asset browser modal
                }
            });

            // Animation clip picker
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Animation Clip:");
                let current = animation_clip.as_deref().unwrap_or("(none)");
                let mut clip_name = current.to_string();
                if ui.text_edit_singleline(&mut clip_name).changed() {
                    if clip_name.is_empty() || clip_name == "(none)" {
                        *animation_clip = None;
                    } else {
                        *animation_clip = Some(clip_name);
                    }
                    changed = true;
                }

                // Asset picker button
                if ui
                    .small_button("📁")
                    .on_hover_text("Browse animations")
                    .clicked()
                {
                    // TODO: Open animation clip browser
                }
            });

            // Material picker
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Material:");
                let current = material.as_deref().unwrap_or("(none)");
                let mut mat_name = current.to_string();
                if ui.text_edit_singleline(&mut mat_name).changed() {
                    if mat_name.is_empty() || mat_name == "(none)" {
                        *material = None;
                    } else {
                        *material = Some(mat_name);
                    }
                    changed = true;
                }

                if ui
                    .small_button("📁")
                    .on_hover_text("Browse materials")
                    .clicked()
                {
                    // TODO: Open material browser
                }
            });

            // Show texture preview if available
            if let Some(path) = texture {
                ui.add_space(4.0);
                ui.label(format!("Path: {}", path));
            }
        }
        Component::Tilemap { map } => {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Tilemap:");
                let mut map_name = map.clone();
                if ui.text_edit_singleline(&mut map_name).changed() {
                    *map = map_name;
                    changed = true;
                }
                if ui
                    .small_button("📁")
                    .on_hover_text("Browse tilemaps")
                    .clicked()
                {
                    // TODO: Open tilemap browser
                }
            });

            // Show tilemap info if it exists in project
            if let Some(tilemap) = project.tilemaps.get(map) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let total_chunks: usize = tilemap.layers.iter().map(|l| l.chunks.len()).sum();
                    let total_tiles: usize = tilemap
                        .layers
                        .iter()
                        .flat_map(|l| l.chunks.values())
                        .map(|c| c.tiles.len())
                        .sum();
                    ui.label("Size:");
                    ui.label(format!("{}×{} chunks", total_chunks, total_tiles));
                    ui.label("Tile size:");
                    ui.label(format!("{}×{}", tilemap.tile_size[0], tilemap.tile_size[1]));
                });
            }
        }
        Component::Role { role } => {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Role:");
                if ui.text_edit_singleline(role).changed() {
                    changed = true;
                }
                ui.label("(used by scripts to find entities)");
            });
        }
        Component::Parent { parent } => {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Parent:");
                let mut parent_id = parent.clone();
                if ui.text_edit_singleline(&mut parent_id).changed() {
                    *parent = parent_id;
                    changed = true;
                }
                ui.label("(parent entity source id)");
            });
        }
        Component::PrefabInstance {
            prefab_id,
            overrides,
        } => {
            let name = project
                .prefabs
                .get(prefab_id)
                .map(|prefab| prefab.name.as_str())
                .unwrap_or("Missing Prefab");
            ui.label(format!("Prefab: {name} ({prefab_id})"));
            ui.label(format!("Instance overrides: {}", overrides.overrides.len()));
            if !project.prefabs.contains_key(prefab_id) {
                ui.colored_label(egui::Color32::YELLOW, "Prefab definition is missing");
            }
        }
    }

    changed
}
