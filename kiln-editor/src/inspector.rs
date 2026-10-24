use kiln_core::project::{Component, Project};
use kiln_core::Transform;

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
) -> Option<String> {
    let Some(entity_id) = state.selected_entity_id.as_ref() else {
        ui.heading("Inspector");
        ui.add_space(8.0);
        ui.label("Select an entity in the Hierarchy to edit it.");
        return None;
    };

    let Some(scene) = project.scenes.get_mut(scene_id) else {
        return None;
    };

    let Some(entity) = scene.entities.iter_mut().find(|e| e.id == *entity_id) else {
        return None;
    };

    ui.heading("Inspector");
    ui.add_space(8.0);

    ui.label("Entity:");
    ui.text_edit_singleline(&mut entity.name);
    ui.add_space(8.0);

    for component in &mut entity.components {
        ui.separator();
        render_component(ui, component);
    }

    ui.separator();
    ui.add_space(4.0);
    if ui.button("+ Add Component").clicked() {
        // Component addition handled by caller
    }

    None
}

fn render_component(ui: &mut egui::Ui, component: &mut Component) {
    match component {
        Component::Transform { x, y, rotation, scale } => {
            ui.label("Transform");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Position:");
                ui.add(egui::DragValue::new(x).speed(1.0));
                ui.add(egui::DragValue::new(y).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Rotation:");
                ui.add(egui::DragValue::new(rotation).speed(0.1));
                ui.label("Scale:");
                ui.add(egui::DragValue::new(scale).speed(0.1).clamp_range(0.001..=100.0));
            });
        }
        Component::Sprite { color, texture } => {
            ui.label("Sprite");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color = [color[0], color[1], color[2], color[3]];
                if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                    color[0] = color[0].clamp(0.0, 1.0);
                    color[1] = color[1].clamp(0.0, 1.0);
                    color[2] = color[2].clamp(0.0, 1.0);
                    color[3] = color[3].clamp(0.0, 1.0);
                }
            });
            ui.label(format!("Texture: {:?}", texture));
        }
        Component::Role { role } => {
            ui.label("Role");
            ui.add_space(4.0);
            ui.text_edit_singleline(role);
        }
        Component::Tilemap { map } => {
            ui.label("Tilemap");
            ui.add_space(4.0);
            ui.label(format!("Map: {}", map));
        }
    }
}
