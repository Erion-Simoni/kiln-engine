//! Input rebinding panel — allows users to rebind keyboard/gamepad actions.

use egui::{ComboBox, Color32, RichText, ScrollArea, Sense, Ui, Vec2, Widget};
use kiln_input::{Action, ActionMap, Binding, BindingType};
use kiln_core::Project;

/// State for the input rebinding panel.
#[derive(Debug, Default)]
pub struct InputRebindPanel {
    /// The action currently being rebound.
    rebinding_action: Option<String>,
    /// The index of the binding being replaced.
    rebinding_binding_idx: Option<usize>,
    /// Whether we're waiting for a new key press.
    waiting_for_key: bool,
}

impl InputRebindPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the input rebinding panel.
    pub fn render(&mut self, ui: &mut Ui, project: &mut Project) {
        ui.heading("Input Rebinding");
        ui.add_space(8.0);

        let action_map = &mut project.action_map;

        if self.waiting_for_key {
            self.render_waiting_for_key(ui, action_map);
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            for action in action_map.actions() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(&action.name);
                    ui.label(format!("({})", action.id));
                });

                if action.bindings.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("No bindings");
                        if ui.small_button("+ Add Binding").clicked() {
                            self.start_rebind(&action.id, action.bindings.len(), action_map);
                        }
                    });
                } else {
                    for (idx, binding) in action.bindings.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let label = format_binding(binding);
                            let button_text = if self.rebinding_action.as_deref() == Some(&action.id)
                                && self.rebinding_binding_idx == Some(idx)
                            {
                                "Press a key..."
                            } else {
                                &label
                            };

                            let btn = ui.button(button_text);
                            if btn.clicked() && !self.waiting_for_key {
                                self.start_rebind(&action.id, idx, action_map);
                            }

                            if ui.small_button("✕").on_hover_text("Remove binding").clicked() {
                                if let Some(action_mut) = action_map.get_action_mut(&action.id) {
                                    action_mut.bindings.remove(idx);
                                }
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        if ui.small_button("+ Add Binding").clicked() {
                            self.start_rebind(&action.id, action.bindings.len(), action_map);
                        }
                    });
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add New Action").clicked() {
                    let new_id = format!("action_{}", action_map.actions.len());
                    let action = Action::new(&new_id, &new_id);
                    action_map.add_action(action);
                }
            });
        });
    }
    fn start_rebind(&mut self, action_id: &str, binding_idx: usize, action_map: &mut ActionMap) {
        self.rebinding_action = Some(action_id.to_string());
        self.rebinding_binding_idx = Some(binding_idx);
        self.waiting_for_key = true;
    }

    fn render_waiting_for_key(&mut self, ui: &mut Ui, action_map: &mut ActionMap) {
        ui.add_space(16.0);
        ui.label(RichText::new("Press a key or gamepad button...").color(Color32::YELLOW).size(18.0));
        ui.add_space(8.0);
        ui.label("Press ESC to cancel");

        let ctx = ui.ctx();
        let input = ctx.input(|i| i.clone());

        for key in &input.keys_down {
            let key_name = format!("{:?}", key);
            self.apply_rebinding(action_map, BindingType::Key(key_name));
            return;
        }

        if input.key_pressed(egui::Key::Escape) {
            self.cancel_rebinding();
        }
    }

    fn apply_rebinding(&mut self, action_map: &mut ActionMap, binding_type: BindingType) {
        if let (Some(action_id), Some(binding_idx)) = (
            self.rebinding_action.take(),
            self.rebinding_binding_idx.take(),
        ) {
            if let Some(action) = action_map.get_action_mut(&action_id) {
                let binding = Binding::new(binding_type);
                if binding_idx >= action.bindings.len() {
                    action.bindings.push(binding);
                } else {
                    action.bindings[binding_idx] = binding;
                }
            }
        }
        self.waiting_for_key = false;
    }

    fn cancel_rebinding(&mut self) {
        self.rebinding_action = None;
        self.rebinding_binding_idx = None;
        self.waiting_for_key = false;
    }

}

/// Format a binding for display.
fn format_binding(binding: &Binding) -> String {
    let mut parts = Vec::new();
    if binding.ctrl {
        parts.push("Ctrl".to_string());
    }
    if binding.shift {
        parts.push("Shift".to_string());
    }
    if binding.alt {
        parts.push("Alt".to_string());
    }
    parts.push(binding.binding_type.to_string());
    parts.join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_input::{Binding, BindingType};

    #[test]
    fn test_format_binding() {
        let binding = Binding::new(BindingType::Key("Space".to_string()));
        assert_eq!(format_binding(&binding), "Space");

        let binding = Binding::new(BindingType::Key("Space".to_string())).with_ctrl();
        assert_eq!(format_binding(&binding), "Ctrl + Space");
    }
}
