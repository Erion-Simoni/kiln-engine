//! Prefab Editor panel — create, edit, and instantiate prefabs with override view.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2, ComboBox, Stroke, Sense};
use kiln_core::{Entity, Prefab, PrefabOverride, Component, ComponentOverride};
use kiln_editor::PrefabEditorState;
use std::collections::HashMap;

/// State of the prefab editor panel.
#[derive(Debug, Default)]
pub struct PrefabEditorPanel {
    /// The prefab editor state.
    pub state: PrefabEditorState,
    /// Whether to show the new prefab dialog.
    pub show_new_prefab_dialog: bool,
    /// New prefab name.
    pub new_prefab_name: String,
    /// Whether to show the override editor.
    pub show_override_editor: bool,
    /// Currently selected override entity.
    pub override_entity_id: Option<String>,
    /// Current override being edited.
    pub current_override: Option<PrefabOverride>,
    /// Search filter for prefabs.
    filter: String,
}

impl PrefabEditorPanel {
    pub fn new() -> Self {
        Self {
            state: PrefabEditorState::new(),
            show_new_prefab_dialog: false,
            new_prefab_name: String::new(),
            show_override_editor: false,
            override_entity_id: None,
            current_override: None,
            filter: String::new(),
        }
    }


    /// Render the prefab editor panel.
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("Prefab Editor");
        ui.add_space(8.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ New Prefab").clicked() {
                self.show_new_prefab_dialog = true;
                self.new_prefab_name.clear();
            }
            if ui.button("🗑 Delete Selected").clicked() {
                if let Some(id) = self.state.selected_prefab_id() {
                    self.state.remove_prefab(id);
                }
            }
            ui.separator();
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
        );

        ui.add_space(8.0);

        // Prefab list
        ui.heading("Prefabs");
        ui.add_space(4.0);

        let mut selected_prefab_id = self.state.selected_prefab_id().map(|s| s.to_string());

        ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            let prefabs: Vec<_> = self.state.prefab_list()
                .into_iter()
                .filter(|p| self.filter.is_empty() || p.name.to_lowercase().contains(&self.filter.to_lowercase()))
                .collect();

            if prefabs.is_empty() {
                ui.label("No prefabs. Click 'New Prefab' to create one.");
            } else {
                for prefab in prefabs {
                    let is_selected = selected_prefab_id.as_deref() == Some(&prefab.id);
                    let mut selected = is_selected;
                    
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &prefab.name).clicked() {
                            self.state.select_prefab(&prefab.id);
                        }
                        
                        if ui.small_button("✏️").on_hover_text("Edit overrides").clicked() {
                            self.show_override_editor = true;
                            self.override_entity_id = Some(prefab.root_entity.id.clone());
                            self.current_override = Some(PrefabOverride::new(prefab.root_entity.id.clone()));
                        }
                        
                        if ui.small_button("🗑").clicked() {
                            self.state.remove_prefab(&prefab.id);
                        }
                    });
                }
            }
            
            if prefabs.is_empty() {
                ui.label("No prefabs. Click 'New Prefab' to create one.");
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);


        // Override editor
        if self.show_override_editor {
            ui.heading("Override Editor");
            ui.add_space(8.0);

            if let Some(override_data) = &mut self.current_override {
                if let Some(entity_id) = &self.override_entity_id {
                    ui.label(format!("Editing overrides for: {}", entity_id));
                    ui.add_space(4.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("➕ Add Override").clicked() {
                            // Add a new override
                        }
                        if ui.button("✅ Apply & Close").clicked() {
                            self.show_override_editor = false;
                        }
                    });

                    ui.add_space(8.0);
                    
                    // Override list
                    let override_keys: Vec<String> = override_data.overrides.keys().cloned().collect();
                    
                    for entity_id in override_keys {
                        if let Some(override_val) = override_data.overrides.get_mut(&entity_id) {
                            ui.horizontal(|ui| {
                                ui.label(format!("Entity: {}", entity_id));
                                
                                match override_val {
                                    ComponentOverride::Transform { x, y, rotation, scale } => {
                                        ui.label("Transform:");
                                        if let Some(x) = x {
                                            ui.add(DragValue::new(x).speed(0.1).suffix(" x"));
                                        } else {
                                            if ui.button("Add X").clicked() {
                                                // Would add x override
                                            }
                                        }
                                        if let Some(y) = y {
                                            ui.add(DragValue::new(y).speed(0.1).suffix(" y"));
                                        } else {
                                            if ui.button("Add Y").clicked() {
                                                // Would add y override
                                            }
                                        }
                                        if let Some(rotation) = rotation {
                                            ui.add(DragValue::new(rotation).speed(0.01).suffix(" rad"));
                                        }
                                        if let Some(scale) = scale {
                                            ui.add(DragValue::new(scale).speed(0.01).suffix(" s"));
                                        }
                                    }
                                    ComponentOverride::Sprite { color, texture } => {
                                        ui.label("Sprite:");
                                        if let Some(color) = color {
                                            let mut color_arr = *color;
                                            if ui.color_edit_button_rgba_unmultiplied(&mut color_arr).changed() {
                                                *color = color_arr;
                                            }
                                        } else {
                                            if ui.button("Add Color").clicked() {
                                                // Would add color override
                                            }
                                        }
                                        if let Some(texture) = texture {
                                            ui.label(format!("Texture: {}", texture));
                                        } else {
                                            if ui.button("Add Texture").clicked() {
                                                // Would add texture override
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            } else {
                ui.label("No overrides yet.");
            }
            
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Done").clicked() {
                    self.show_override_editor = false;
                }
            });
        }


        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Prefab creation dialog
        if self.show_new_prefab_dialog {
            egui::Window::new("New Prefab")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Prefab name:");
                    ui.text_edit_singleline(&mut self.new_prefab_name);

                    ui.add_space(8.0);
                    ui.label("Note: This creates a prefab from the currently selected entity in the scene.");

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_new_prefab_dialog = false;
                        }
                        if ui.button("Create").clicked() {
                            if !self.new_prefab_name.is_empty() {
                                // TODO: Create prefab from selected entity
                                self.show_new_prefab_dialog = false;
                            }
                        }
                    });
            });
        }
    }
}


