//! Preferences panel — editor theme, key bindings, external tool paths.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2, ComboBox};
use std::collections::HashMap;
use std::path::PathBuf;

/// Editor theme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EditorTheme {
    Dark,
    Light,
    System,
}

/// Key binding entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyBinding {
    pub action: String,
    pub key: String,
    pub modifiers: Vec<String>,
}

/// External tool configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalTool {
    pub name: String,
    pub path: PathBuf,
    pub args: String,
    pub enabled: bool,
}

/// Preferences state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Preferences {
    pub theme: EditorTheme,
    pub font_size: f32,
    pub key_bindings: Vec<KeyBinding>,
    pub external_tools: Vec<ExternalTool>,
    pub auto_save: bool,
    pub auto_save_interval: f32,
    pub show_tooltips: bool,
    pub confirm_on_close: bool,
    pub recent_projects_max: usize,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: EditorTheme::System,
            font_size: 13.0,
            key_bindings: vec![
                KeyBinding { action: "save".to_string(), key: "S".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "open".to_string(), key: "O".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "new".to_string(), key: "N".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "undo".to_string(), key: "Z".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "redo".to_string(), key: "Y".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "play".to_string(), key: "F5".to_string(), modifiers: vec![] },
                KeyBinding { action: "stop".to_string(), key: "F5".to_string(), modifiers: vec!["Shift".to_string()] },
                KeyBinding { action: "find".to_string(), key: "F".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "replace".to_string(), key: "H".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "duplicate".to_string(), key: "D".to_string(), modifiers: vec!["Ctrl".to_string()] },
                KeyBinding { action: "delete".to_string(), key: "Delete".to_string(), modifiers: vec![] },
            ],
            external_tools: vec![
                ExternalTool { name: "Image Editor".to_string(), path: PathBuf::new(), args: "".to_string(), enabled: false },
                ExternalTool { name: "Code Editor".to_string(), path: PathBuf::new(), args: "".to_string(), enabled: false },
                ExternalTool { name: "Audio Editor".to_string(), path: PathBuf::new(), args: "".to_string(), enabled: false },
            ],
            auto_save: true,
            auto_save_interval: 30.0,
            show_tooltips: true,
            confirm_on_close: true,
            recent_projects_max: 10,
        }
    }


/// State of the preferences panel.
#[derive(Debug, Default)]
pub struct PreferencesPanel {
    preferences: Preferences,
    show_key_binding_dialog: bool,
    editing_key_binding: Option<usize>,
    new_key_binding_action: String,
    new_key_binding_key: String,
    new_key_binding_modifiers: Vec<String>,
    show_external_tool_dialog: bool,
    editing_external_tool: Option<usize>,
    new_tool_name: String,
    new_tool_path: String,
    new_tool_args: String,
}

impl PreferencesPanel {
    pub fn new() -> Self {
        Self {
            preferences: Preferences::default(),
            ..Default::default()
        }
    }

    pub fn load(&mut self, prefs: Preferences) {
        self.preferences = prefs;
    }

    pub fn get_preferences(&self) -> &Preferences {
        &self.preferences
    }


    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Preferences");
        ui.add_space(8.0);

        // General tab
        ui.separator();
        ui.heading("General");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Theme:");
            ComboBox::from_label("")
                .selected_text(format!("{:?}", self.preferences.theme))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.preferences.theme, EditorTheme::Dark, "Dark");
                    ui.selectable_value(&mut self.preferences.theme, EditorTheme::Light, "Light");
                    ui.selectable_value(&mut self.preferences.theme, EditorTheme::System, "System");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Font Size:");
            ui.add(egui::DragValue::new(&mut self.preferences.font_size).speed(0.5).range(8.0..=24.0).suffix("px"));
        });

        ui.add_space(4.0);
        ui.checkbox(&mut self.preferences.auto_save, "Auto-save");
        if self.preferences.auto_save {
            ui.horizontal(|ui| {
                ui.label("Interval (seconds):");
                ui.add(egui::DragValue::new(&mut self.preferences.auto_save_interval).speed(1.0).range(5.0..=300.0).suffix("s"));
            });
        }

        ui.add_space(4.0);
        ui.checkbox(&mut self.preferences.show_tooltips, "Show tooltips");
        ui.checkbox(&mut self.preferences.confirm_on_close, "Confirm on close");

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Recent projects max:");
            ui.add(egui::DragValue::new(&mut self.preferences.recent_projects_max).speed(1).range(0..=50));
        });

        // Key Bindings tab
        ui.add_space(8.0);
        ui.separator();
        ui.heading("Key Bindings");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("➕ Add Binding").clicked() {
                self.show_key_binding_dialog = true;
                self.editing_key_binding = None;
                self.new_key_binding_action = String::new();
                self.new_key_binding_key = String::new();
                self.new_key_binding_modifiers = vec![];
            }
        });

        ui.add_space(4.0);

        egui::Grid::new("key_bindings_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                ui.label(RichText::new("Action").strong());
                ui.label(RichText::new("Key").strong());
                ui.label(RichText::new("Modifiers").strong());
                ui.label(RichText::new("Actions").strong());
                ui.end_row();

                let mut remove_idx = None;
                for (idx, binding) in self.preferences.key_bindings.iter().enumerate() {
                    ui.label(&binding.action);
                    ui.label(&binding.key);
                    ui.label(binding.modifiers.join(" + "));
                    if ui.button("🗑").clicked() {
                        remove_idx = Some(idx);
                    }
                    ui.end_row();
                }
                if let Some(idx) = remove_idx {
                    self.preferences.key_bindings.remove(idx);
                }
            });

        // Key binding dialog
        if self.show_key_binding_dialog {
            egui::Window::new(if self.editing_key_binding.is_some() { "Edit Key Binding" } else { "Add Key Binding" })
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Action:");
                    ui.text_edit_singleline(&mut self.new_key_binding_action);

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Key:");
                        ui.text_edit_singleline(&mut self.new_key_binding_key);
                    });

                    ui.add_space(4.0);
                    ui.label("Modifiers (comma-separated):");
                    ui.text_edit_singleline(&mut self.new_key_binding_modifiers.join(", "));

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_key_binding_dialog = false;
                            self.editing_key_binding = None;
                            self.new_key_binding_action.clear();
                            self.new_key_binding_key.clear();
                            self.new_key_binding_modifiers.clear();
                        }
                        if ui.button(if self.editing_key_binding.is_some() { "Save" } else { "Add" }).clicked() {
                            if !self.new_key_binding_action.is_empty() && !self.new_key_binding_key.is_empty() {
                                let modifiers = self.new_key_binding_modifiers.join(",").split(',').map(|s| s.trim().to_string()).collect();
                                let binding = KeyBinding {
                                    action: self.new_key_binding_action.clone(),
                                    key: self.new_key_binding_key.clone(),
                                    modifiers,
                                };
                                if let Some(idx) = self.editing_key_binding {
                                    self.preferences.key_bindings[idx] = binding;
                                } else {
                                    self.preferences.key_bindings.push(binding);
                                }
                                self.show_key_binding_dialog = false;
                                self.editing_key_binding = None;
                                self.new_key_binding_action.clear();
                                self.new_key_binding_key.clear();
                                self.new_key_binding_modifiers.clear();
                            }
                        });
                    });
            }
        }

        // External Tools tab
        ui.add_space(8.0);
        ui.separator();
        ui.heading("External Tools");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("➕ Add Tool").clicked() {
                self.show_external_tool_dialog = true;
                self.editing_external_tool = None;
                self.new_tool_name = String::new();
                self.new_tool_path = String::new();
                self.new_tool_args = String::new();
            }
        );

        ui.add_space(4.0);

        egui::Grid::new("external_tools_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("Path").strong());
                ui.label(RichText::new("Args").strong());
                ui.label(RichText::new("Enabled").strong());
                ui.label(RichText::new("Actions").strong());
                ui.end_row();

                let mut remove_idx = None;
                for (idx, tool) in self.preferences.external_tools.iter().enumerate() {
                    ui.label(&tool.name);
                    ui.label(tool.path.display().to_string());
                    ui.label(&tool.args);
                    ui.checkbox(&mut self.preferences.external_tools[idx].enabled, "");
                    if ui.small_button("🗑").clicked() {
                        remove_idx = Some(idx);
                    }
                    ui.end_row();
                }
                if let Some(idx) = remove_idx {
                    self.preferences.external_tools.remove(idx);
                }
            });

        if self.show_external_tool_dialog {
            egui::Window::new(if self.editing_external_tool.is_some() { "Edit External Tool" } else { "Add External Tool" })
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_tool_name);

                    ui.add_space(4.0);
                    ui.label("Path:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_tool_path);
                        if ui.button("Browse...").clicked() {
                            // TODO: Native file dialog
                        }
                    });

                    ui.add_space(4.0);
                    ui.label("Arguments:");
                    ui.text_edit_singleline(&mut self.new_tool_args);

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_external_tool_dialog = false;
                            self.editing_external_tool = None;
                            self.new_tool_name.clear();
                            self.new_tool_path.clear();
                            self.new_tool_args.clear();
                        }
                        if ui.button(if self.editing_external_tool.is_some() { "Save" } else { "Add" }).clicked() {
                            if !self.new_tool_name.is_empty() && !self.new_tool_path.is_empty() {
                                let tool = ExternalTool {
                                    name: self.new_tool_name.clone(),
                                    path: std::path::PathBuf::from(&self.new_tool_path),
                                    args: self.new_tool_args.clone(),
                                    enabled: true,
                                };
                                if let Some(idx) = self.editing_external_tool {
                                    self.preferences.external_tools[idx] = tool;
                                } else {
                                    self.preferences.external_tools.push(tool);
                                }
                                self.show_external_tool_dialog = false;
                                self.editing_external_tool = None;
                                self.new_tool_name.clear();
                                self.new_tool_path.clear();
                                self.new_tool_args.clear();
                            }
                        });
                    });
            }
        }

        ui.add_space(8.0);
        ui.separator();

        // Apply/Reset buttons
        ui.horizontal(|ui| {
            if ui.button("Reset to Defaults").clicked() {
                self.preferences = Preferences::default();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Apply").clicked() {
                    // Apply preferences - would save to config file
                }
                if ui.button("Cancel").clicked() {
                    // Revert to saved preferences
                }
            });
        });
    }
}


