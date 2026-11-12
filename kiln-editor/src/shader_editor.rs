//! Custom Shader Editor panel — create and edit WGSL shaders for sprite materials.
//!
//! Shaders are WGSL fragment programs that take position, size, color, uv_rect
//! as inputs and output a color. Materials reference shaders with parameter
//! overrides.

use kiln_core::project::{MaterialParam, Shader};
use std::collections::HashMap;

/// State of the shader editor panel.
#[derive(Debug, Default)]
pub struct ShaderEditorState {
    /// All shader definitions, keyed by their ID.
    pub shaders: HashMap<String, Shader>,
    /// Name of the currently selected shader.
    pub selected_shader: Option<String>,
    /// Name for a new shader.
    pub new_shader_name: String,
    /// Show new shader dialog.
    pub show_new_shader_dialog: bool,
    /// Currently editing shader source.
    pub edit_source: String,
    /// Entry point name for the shader.
    pub edit_entry_point: String,
    pub selected_material: Option<String>,
    pub new_material_name: String,
    pub new_parameter_name: String,
    pub new_parameter_kind: usize,
}

impl ShaderEditorState {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            selected_shader: None,
            new_shader_name: String::new(),
            show_new_shader_dialog: false,
            edit_source: String::new(),
            edit_entry_point: "fs_main".to_string(),
            selected_material: None,
            new_material_name: String::new(),
            new_parameter_name: String::new(),
            new_parameter_kind: 0,
        }
    }

    pub fn selected_shader(&self) -> Option<&Shader> {
        self.selected_shader
            .as_ref()
            .and_then(|name| self.shaders.get(name))
    }

    pub fn selected_shader_mut(&mut self) -> Option<&mut Shader> {
        self.selected_shader
            .as_deref()
            .and_then(|name| self.shaders.get_mut(name))
    }

    pub fn add_shader(&mut self, name: String, source: String) {
        let shader = Shader {
            parameters: Default::default(),
            id: name.clone(),
            name,
            source,
            entry_point: "fs_main".to_string(),
        };
        self.selected_shader = Some(shader.id.clone());
        self.shaders.insert(shader.id.clone(), shader);
    }

    pub fn remove_shader(&mut self, name: &str) {
        self.shaders.remove(name);
        if self.selected_shader.as_deref() == Some(name) {
            self.selected_shader = None;
        }
    }
}
pub fn render_shader_editor(
    ui: &mut egui::Ui,
    state: &mut ShaderEditorState,
    project: &mut kiln_core::Project,
) {
    state.shaders = project.shaders.clone();
    if state
        .selected_shader
        .as_ref()
        .is_some_and(|id| !project.shaders.contains_key(id))
    {
        state.selected_shader = None;
    }
    ui.heading("Shader Editor");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Shaders:");
        egui::ComboBox::from_label("")
            .selected_text(
                state
                    .selected_shader
                    .as_deref()
                    .unwrap_or("Select a shader..."),
            )
            .show_ui(ui, |ui| {
                for name in state.shaders.keys().cloned().collect::<Vec<_>>() {
                    if ui
                        .selectable_label(state.selected_shader.as_deref() == Some(&name), &name)
                        .clicked()
                    {
                        state.selected_shader = Some(name);
                    }
                }
            });

        ui.separator();

        if ui.button("+ New Shader").clicked() {
            state.show_new_shader_dialog = true;
            state.new_shader_name.clear();
        }

        if ui.button("Delete Shader").clicked() {
            if let Some(name) = state.selected_shader.take() {
                state.remove_shader(&name);
                project.shaders.remove(&name);
            }
        }
    });

    ui.add_space(8.0);

    if state.show_new_shader_dialog {
        egui::Window::new("New Shader")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.label("Shader name:");
                ui.text_edit_singleline(&mut state.new_shader_name);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.show_new_shader_dialog = false;
                        state.new_shader_name.clear();
                    }
                    if ui.button("Create").clicked() && !state.new_shader_name.is_empty() {
                        let default_source = kiln_render::materials::DEFAULT_FRAGMENT.to_string();
                        state.add_shader(state.new_shader_name.clone(), default_source);
                        state.show_new_shader_dialog = false;
                        state.new_shader_name.clear();
                    }
                });
            });
    }

    if let Some(shader) = state.selected_shader_mut() {
        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                ui.heading("Shader Properties");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut shader.name);
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Entry Point:");
                    ui.text_edit_singleline(&mut shader.entry_point);
                });

                ui.add_space(8.0);

                ui.heading("Parameters");
                ui.add_space(4.0);
                for (param_name, param) in shader.parameters.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.label(param_name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            match param {
                                MaterialParam::Float(v) => {
                                    ui.add(egui::DragValue::new(v).speed(0.1));
                                }
                                MaterialParam::Vec2(v) => {
                                    ui.add(egui::DragValue::new(&mut v[0]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[1]).speed(0.1));
                                }
                                MaterialParam::Vec3(v) => {
                                    ui.add(egui::DragValue::new(&mut v[0]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[1]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[2]).speed(0.1));
                                }
                                MaterialParam::Vec4(v) => {
                                    ui.add(egui::DragValue::new(&mut v[0]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[1]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[2]).speed(0.1));
                                    ui.add(egui::DragValue::new(&mut v[3]).speed(0.1));
                                }
                                MaterialParam::Color(v) => {
                                    ui.color_edit_button_rgba_unmultiplied(v);
                                }
                                MaterialParam::Texture(t) => {
                                    ui.text_edit_singleline(t);
                                }
                            }
                        });
                    });
                }

                ui.add_space(8.0);

                ui.heading("Shader Source (WGSL)");
                ui.add_space(4.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::TextEdit::multiline(&mut shader.source)
                        .id_source("shader_source")
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .show(ui);
                });
            });

            cols[1].vertical(|ui| {
                ui.heading("Preview");
                ui.add_space(4.0);

                ui.label("Assign a material to a sprite and view it in Scene or Game.");
                ui.label(
                    "The scene viewport shows the compiled fragment, textures and parameters.",
                );
                ui.label(format!("Shader ID: {}", shader.id));
                ui.label(format!("Entry point: {}", shader.entry_point));
                ui.label(format!("Parameters: {}", shader.parameters.len()));
            });
        });
    }

    create_parameter(ui, state);
    project.shaders = state.shaders.clone();
    render_materials(ui, state, project);
}

fn edit_parameter(ui: &mut egui::Ui, value: &mut MaterialParam) {
    match value {
        MaterialParam::Float(v) => {
            ui.add(egui::DragValue::new(v).speed(0.01));
        }
        MaterialParam::Vec2(v) => {
            for n in v {
                ui.add(egui::DragValue::new(n).speed(0.01));
            }
        }
        MaterialParam::Vec3(v) => {
            for n in v {
                ui.add(egui::DragValue::new(n).speed(0.01));
            }
        }
        MaterialParam::Vec4(v) => {
            for n in v {
                ui.add(egui::DragValue::new(n).speed(0.01));
            }
        }
        MaterialParam::Color(v) => {
            ui.color_edit_button_rgba_unmultiplied(v);
        }
        MaterialParam::Texture(v) => {
            ui.text_edit_singleline(v);
        }
    }
}
fn render_materials(
    ui: &mut egui::Ui,
    state: &mut ShaderEditorState,
    project: &mut kiln_core::Project,
) {
    ui.separator();
    ui.heading("Sprite materials");
    let mut ids: Vec<_> = project.materials.keys().cloned().collect();
    ids.sort();
    egui::ComboBox::from_id_salt("sprite_material_selection")
        .selected_text(
            state
                .selected_material
                .as_deref()
                .unwrap_or("Choose material…"),
        )
        .show_ui(ui, |ui| {
            for id in ids {
                ui.selectable_value(&mut state.selected_material, Some(id.clone()), id);
            }
        });
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut state.new_material_name);
        if ui
            .add_enabled(
                state.selected_shader.is_some() && !state.new_material_name.trim().is_empty(),
                egui::Button::new("Create material"),
            )
            .clicked()
        {
            let id = state.new_material_name.trim().to_owned();
            project.materials.entry(id.clone()).or_insert_with(|| {
                kiln_core::project::SpriteMaterial {
                    id: id.clone(),
                    name: id.clone(),
                    shader: state.selected_shader.clone().unwrap(),
                    parameters: Default::default(),
                }
            });
            state.selected_material = Some(id);
            state.new_material_name.clear();
        }
    });
    let Some(id) = state.selected_material.clone() else {
        return;
    };
    if ui.button("Delete material").clicked() {
        project.materials.remove(&id);
        state.selected_material = None;
        return;
    }
    let Some(material) = project.materials.get_mut(&id) else {
        return;
    };
    let mut shader_ids: Vec<_> = project.shaders.keys().cloned().collect();
    shader_ids.sort();
    egui::ComboBox::from_id_salt("material_shader")
        .selected_text(&material.shader)
        .show_ui(ui, |ui| {
            for shader in shader_ids {
                ui.selectable_value(&mut material.shader, shader.clone(), shader);
            }
        });
    if ui.button("Reset overrides").clicked() {
        material.parameters.clear();
    }
    if let Some(shader) = project.shaders.get(&material.shader) {
        let defaults: std::collections::BTreeMap<_, _> = shader.parameters.iter().collect();
        for (name, default) in defaults {
            ui.horizontal(|ui| {
                let mut overridden = material.parameters.contains_key(name);
                if ui.checkbox(&mut overridden, name).changed() {
                    if overridden {
                        material.parameters.insert(name.clone(), default.clone());
                    } else {
                        material.parameters.remove(name);
                    }
                }
                if let Some(value) = material.parameters.get_mut(name) {
                    edit_parameter(ui, value);
                } else {
                    ui.label("Shader default");
                }
            });
        }
    }
}

fn create_parameter(ui: &mut egui::Ui, state: &mut ShaderEditorState) {
    if state.selected_shader.is_none() {
        return;
    }
    let kinds = ["Float", "Vec2", "Vec3", "Vec4", "Color", "Texture"];
    ui.horizontal(|ui| {
        ui.label("New parameter");
        ui.text_edit_singleline(&mut state.new_parameter_name);
        egui::ComboBox::from_id_salt("new_shader_parameter_type")
            .selected_text(kinds[state.new_parameter_kind])
            .show_ui(ui, |ui| {
                for (index, label) in kinds.iter().enumerate() {
                    ui.selectable_value(&mut state.new_parameter_kind, index, *label);
                }
            });
        if ui.button("Add").clicked() && !state.new_parameter_name.is_empty() {
            let value = match state.new_parameter_kind {
                0 => MaterialParam::Float(1.0),
                1 => MaterialParam::Vec2([0.0; 2]),
                2 => MaterialParam::Vec3([1.0; 3]),
                3 => MaterialParam::Vec4([1.0; 4]),
                4 => MaterialParam::Color([1.0; 4]),
                _ => MaterialParam::Texture(String::new()),
            };
            let name = std::mem::take(&mut state.new_parameter_name);
            state
                .selected_shader_mut()
                .unwrap()
                .parameters
                .entry(name)
                .or_insert(value);
        }
    });
    if let Some(shader) = state.selected_shader_mut() {
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for name in shader.parameters.keys() {
                if ui.small_button(format!("Remove {name}")).clicked() {
                    remove = Some(name.clone());
                }
            }
        });
        if let Some(name) = remove {
            shader.parameters.remove(&name);
        }
    }
}
#[cfg(test)]
mod template_tests {
    #[test]
    fn newly_created_shader_material_compiles_the_shared_fragment_contract() {
        let mut state = super::ShaderEditorState::new();
        state.add_shader(
            "new".into(),
            kiln_render::materials::DEFAULT_FRAGMENT.into(),
        );
        let mut project = kiln_core::Project {
            shaders: state.shaders,
            ..Default::default()
        };
        project.materials.insert(
            "material".into(),
            kiln_core::project::SpriteMaterial {
                id: "material".into(),
                name: "material".into(),
                shader: "new".into(),
                parameters: Default::default(),
            },
        );
        assert!(kiln_render::materials::plans(&project).is_ok());
    }
}
