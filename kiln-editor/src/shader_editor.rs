//! Custom Shader Editor panel — create and edit WGSL shaders for sprite materials.
//!
//! Shaders are WGSL fragment programs that take position, size, color, uv_rect
//! as inputs and output a color. Materials reference shaders with parameter
//! overrides.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2};
use kiln_core::project::{Material, MaterialParam, Shader};
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
}

impl ShaderEditorState {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            selected_shader: None,
            new_shader_name: String::new(),
            show_new_shader_dialog: false,
            edit_source: String::new(),
            edit_entry_point: "main".to_string(),
        }
    }

    pub fn selected_shader(&self) -> Option<&Shader> {
        self.selected_shader.as_ref().and_then(|name| self.shaders.get(name))
    }

    pub fn selected_shader_mut(&mut self) -> Option<&mut Shader> {
        self.selected_shader.as_deref().and_then(|name| self.shaders.get_mut(name))
    }

    pub fn add_shader(&mut self, name: String, source: String) {
        let shader = Shader {
            id: name.clone(),
            name,
            source,
            entry_point: "main".to_string(),
        };
        self.shaders.insert(shader.id.clone(), shader);
        self.selected_shader = Some(shader.id.clone());
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
    ui.heading("Shader Editor");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Shaders:");
        egui::ComboBox::from_label("")
            .selected_text(state.selected_shader.as_deref().unwrap_or("Select a shader..."))
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
                    if ui.button("Create").clicked() {
                        if !state.new_shader_name.is_empty() {
                            let default_source = r#"// Custom sprite shader
// Inputs:
//   position: vec2<f32> - center of sprite
//   size: vec2<f32> - size of sprite
//   color: vec4<f32> - tint color
//   uv_rect: vec4<f32> - UV rect (u0, v0, u1, v1)
//   material_id: u32 - material ID (0 = default)
// Outputs:
//   @location(0) vec4<f32> - final color
@fragment
fn fs_main(
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv_rect: vec4<f32>,
    @location(4) material_id: u32,
) -> @location(0) vec4<f32> {
    let tex_coord = position / size + vec2<f32>(0.5, 0.5);
    let uv = mix(uv_rect.xy, uv_rect.zw, tex_coord);
    let texel = textureSample(sprite_tex, sprite_sampler, uv);
    return texel * color;
}
"#.to_string();
                            state.add_shader(state.new_shader_name.clone(), default_source);
                            state.show_new_shader_dialog = false;
                            state.new_shader_name.clear();
                        }
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
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
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
                });

                ui.add_space(8.0);

                ui.heading("Shader Source (WGSL)");
                ui.add_space(4.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::TextEdit::multiline(&mut shader.source)
                        .id_source("shader_source")
                        .desired_width(f32::INFINITY)
                        .desired_height(400.0)
                        .show(ui);
                });
            });

            cols[1].vertical(|ui| {
                ui.heading("Preview");
                ui.add_space(4.0);

                let preview_size = 128.0;
                let (rect, _response) = ui.allocate_exact_size(
                    Vec2::new(preview_size, preview_size),
                    egui::Sense::hover(),
                );

                ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(20, 20, 25));

                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Color32::from_rgb(100, 200, 255),
                );

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Shader Preview",
                    egui::TextStyle::Small,
                    Color32::from_rgb(200, 200, 200),
                );

                ui.add_space(8.0);
                ui.label(format!("Shader ID: {}", shader.id));
                ui.label(format!("Entry point: {}", shader.entry_point));
                ui.label(format!("Parameters: {}", shader.parameters.len()));
            });
        });
    }

    if let Some(shader) = state.selected_shader() {
        if let Some(project_shader) = project.shaders.get_mut(&shader.id) {
            *project_shader = shader.clone();
        } else {
            project.shaders.insert(shader.id.clone(), shader.clone());
        }
    }
}
