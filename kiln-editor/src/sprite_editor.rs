//! Saved sprite geometry and grid-sheet slicing, using decoded image previews.
use crate::image_preview::PreviewImages;
use kiln_core::visual::SpriteLayout;
use kiln_core::{Clip, Component, Frame, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliceMode {
    #[default]
    Manual,
    Grid,
    AutoDetect,
}
#[derive(Debug)]
pub struct SpriteEditorState {
    pub selected_entity: Option<String>,
    pub spritesheet_path: String,
    pub spritesheet_size: (u32, u32),
    pub pivot: [f32; 2],
    pub slice_mode: SliceMode,
    pub grid_cell_size: (u32, u32),
    pub grid_spacing: (u32, u32),
    pub grid_offset: (u32, u32),
    pub new_clip_name: String,
    images: PreviewImages,
}
impl Default for SpriteEditorState {
    fn default() -> Self {
        Self::new()
    }
}
impl SpriteEditorState {
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            spritesheet_path: String::new(),
            spritesheet_size: (0, 0),
            pivot: [0.5; 2],
            slice_mode: SliceMode::Manual,
            grid_cell_size: (16, 16),
            grid_spacing: (0, 0),
            grid_offset: (0, 0),
            new_clip_name: "walk".into(),
            images: Default::default(),
        }
    }
    pub fn render(&mut self, ui: &mut egui::Ui, project: &mut Project) {
        self.render_with_root(ui, project, None);
    }
    pub fn render_with_root(
        &mut self,
        ui: &mut egui::Ui,
        p: &mut Project,
        root: Option<&std::path::Path>,
    ) {
        ui.heading("Sprite Editor");
        let mut entities: Vec<_> = p
            .scenes
            .iter()
            .flat_map(|(scene_id, scene)| {
                scene
                    .entities
                    .iter()
                    .filter(|entity| {
                        entity
                            .components
                            .iter()
                            .any(|c| matches!(c, Component::Sprite { .. }))
                    })
                    .map(|entity| (scene_id.clone(), entity.id.clone(), entity.name.clone()))
            })
            .collect();
        entities.sort();
        egui::ComboBox::from_id_salt("sprite_entity")
            .selected_text(self.selected_entity.as_deref().unwrap_or("Select entity"))
            .show_ui(ui, |ui| {
                for (_, id, name) in &entities {
                    ui.selectable_value(&mut self.selected_entity, Some(id.clone()), name);
                }
            });
        let Some((scene_id, id, _)) = entities
            .into_iter()
            .find(|(_, id, _)| self.selected_entity.as_ref() == Some(id))
        else {
            return;
        };
        if let Err(error) = crate::scene_controls::require_editable(&p.scenes[&scene_id], &id) {
            ui.label(error);
            return;
        }
        let entity = p.scenes[&scene_id]
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap();
        let mut layout = entity
            .components
            .iter()
            .find_map(|c| match c {
                Component::SpriteLayout { settings } => Some(*settings),
                _ => None,
            })
            .unwrap_or_default();
        let mut layer = entity
            .components
            .iter()
            .find_map(|c| match c {
                Component::Sprite { layer, .. } => Some(*layer),
                _ => None,
            })
            .unwrap_or_default();
        let mut path = entity
            .components
            .iter()
            .find_map(|c| match c {
                Component::Sprite { texture, .. } => texture.clone(),
                _ => None,
            })
            .unwrap_or_default();
        let mut changed = layout_ui(ui, &mut layout);
        ui.horizontal(|ui| {
            ui.label("Texture");
            changed |= ui.text_edit_singleline(&mut path).changed();
            changed |= ui
                .add(egui::DragValue::new(&mut layer).prefix("Sorting layer "))
                .changed();
        });
        self.pivot = layout.pivot;
        self.spritesheet_path = path.clone();
        if changed && layout.validate().is_ok() && layer.is_finite() {
            let entity = p
                .scenes
                .get_mut(&scene_id)
                .unwrap()
                .entities
                .iter_mut()
                .find(|entity| entity.id == id)
                .unwrap();
            set_layout(entity, layout);
            if let Some(Component::Sprite {
                texture,
                layer: sorting,
                ..
            }) = entity
                .components
                .iter_mut()
                .find(|c| matches!(c, Component::Sprite { .. }))
            {
                *texture = (!path.is_empty()).then_some(path.clone());
                *sorting = layer;
            }
        }
        let image = self.images.load(ui.ctx(), root, &path);
        if let Ok(image) = &image {
            self.spritesheet_size = (image.size[0], image.size[1]);
            let scale = (ui.available_width().min(360.0) / image.size[0] as f32)
                .min(240.0 / image.size[1] as f32)
                .min(8.0);
            let (area, _) = ui.allocate_exact_size(
                egui::vec2(image.size[0] as f32 * scale, image.size[1] as f32 * scale),
                egui::Sense::hover(),
            );
            ui.painter().image(
                image.texture.id(),
                area,
                egui::Rect::from_min_max(
                    egui::pos2(
                        if layout.flip_x { 1.0 } else { 0.0 },
                        if layout.flip_y { 1.0 } else { 0.0 },
                    ),
                    egui::pos2(
                        if layout.flip_x { 0.0 } else { 1.0 },
                        if layout.flip_y { 0.0 } else { 1.0 },
                    ),
                ),
                egui::Color32::WHITE,
            );
            let pivot = area.min
                + egui::vec2(
                    area.width() * layout.pivot[0],
                    area.height() * layout.pivot[1],
                );
            ui.painter()
                .circle_stroke(pivot, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::RED));
        } else if let Err(error) = &image {
            ui.label(error);
        }
        ui.separator();
        ui.heading("Grid to Animation");
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.new_clip_name);
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.grid_cell_size.0)
                    .prefix("Cell W ")
                    .range(1..=8192),
            );
            ui.add(
                egui::DragValue::new(&mut self.grid_cell_size.1)
                    .prefix("H ")
                    .range(1..=8192),
            );
        });
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.grid_spacing.0).prefix("Spacing X "));
            ui.add(egui::DragValue::new(&mut self.grid_spacing.1).prefix("Y "));
        });
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.grid_offset.0).prefix("Offset X "));
            ui.add(egui::DragValue::new(&mut self.grid_offset.1).prefix("Y "));
        });
        if ui.button("Create Clip and Attach").clicked() {
            let result = grid_clip(
                &self.new_clip_name,
                &path,
                [self.spritesheet_size.0, self.spritesheet_size.1],
                [self.grid_cell_size.0, self.grid_cell_size.1],
                [self.grid_spacing.0, self.grid_spacing.1],
                [self.grid_offset.0, self.grid_offset.1],
            );
            match result {
                Ok(clip) if !p.animation_clips.contains_key(&clip.name) => {
                    p.animation_clips.insert(clip.name.clone(), clip.clone());
                    let entity = p
                        .scenes
                        .get_mut(&scene_id)
                        .unwrap()
                        .entities
                        .iter_mut()
                        .find(|entity| entity.id == id)
                        .unwrap();
                    if let Some(Component::Sprite {
                        animation_clip,
                        width,
                        height,
                        ..
                    }) = entity
                        .components
                        .iter_mut()
                        .find(|c| matches!(c, Component::Sprite { .. }))
                    {
                        *animation_clip = Some(clip.name);
                        *width = self.grid_cell_size.0 as f32;
                        *height = self.grid_cell_size.1 as f32;
                    }
                }
                Ok(_) => {
                    ui.colored_label(egui::Color32::YELLOW, "Choose a new clip name");
                }
                Err(error) => {
                    ui.label(error);
                }
            }
        }
    }
}
pub fn set_layout(entity: &mut kiln_core::Entity, settings: SpriteLayout) {
    if let Some(component) = entity
        .components
        .iter_mut()
        .find(|c| matches!(c, Component::SpriteLayout { .. }))
    {
        *component = Component::SpriteLayout { settings };
    } else {
        entity.components.push(Component::SpriteLayout { settings });
    }
}
pub fn layout_ui(ui: &mut egui::Ui, layout: &mut SpriteLayout) -> bool {
    let before = *layout;
    ui.horizontal(|ui| {
        for (value, label) in layout.pivot.iter_mut().zip(["Pivot X ", "Y "]) {
            ui.add(
                egui::DragValue::new(value)
                    .prefix(label)
                    .speed(0.01)
                    .range(0.0..=1.0),
            );
        }
    });
    ui.horizontal(|ui| {
        for (label, pivot) in [
            ("Top Left", [0.0, 0.0]),
            ("Center", [0.5, 0.5]),
            ("Feet", [0.5, 1.0]),
        ] {
            if ui.button(label).clicked() {
                layout.pivot = pivot;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut layout.flip_x, "Flip X");
        ui.checkbox(&mut layout.flip_y, "Flip Y");
        ui.add(
            egui::DragValue::new(&mut layout.pixels_per_unit)
                .prefix("Pixels / unit ")
                .range(0.001..=8192.0),
        );
    });
    *layout != before
}
/// Generate tight or spaced grid frames without touching scene files.
pub fn grid_clip(
    name: &str,
    path: &str,
    sheet: [u32; 2],
    cell: [u32; 2],
    spacing: [u32; 2],
    offset: [u32; 2],
) -> Result<Clip, String> {
    if cell.contains(&0) || name.trim().is_empty() || path.is_empty() {
        return Err("Choose clip name, sheet and positive cell size".into());
    }
    let mut clip = Clip::new(name.into(), path.into());
    let [w, h] = cell;
    let step_x = w.checked_add(spacing[0]).ok_or("Grid spacing overflow")?;
    let step_y = h.checked_add(spacing[1]).ok_or("Grid spacing overflow")?;
    let mut y = offset[1];
    while y.checked_add(h).is_some_and(|end| end <= sheet[1]) {
        let mut x = offset[0];
        while x.checked_add(w).is_some_and(|end| end <= sheet[0]) {
            if clip.frames.len() == 4096 {
                return Err("Sheet contains more than 4096 frames".into());
            }
            clip.add_frame(Frame {
                rect: [x, y, w, h],
                pivot: [0.5; 2],
                duration: 0.1,
                events: vec![],
            });
            let Some(next) = x.checked_add(step_x) else {
                break;
            };
            x = next;
        }
        let Some(next) = y.checked_add(step_y) else {
            break;
        };
        y = next;
    }
    clip.validate(sheet)?;
    Ok(clip)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sprite_editor_new() {
        let state = SpriteEditorState::new();
        assert!(state.selected_entity.is_none());
        assert_eq!(state.pivot, [0.5; 2]);
    }
    #[test]
    fn test_slice_mode_default() {
        assert_eq!(SliceMode::default(), SliceMode::Manual);
    }
}
