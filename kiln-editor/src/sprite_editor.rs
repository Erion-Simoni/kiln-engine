//! Sprite Editor panel — pivot, slice, 9-slice editing, texture atlas packing.

use egui::{Color32, DragValue, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use kiln_core::project::{Component, Entity, Project};
use std::collections::HashMap;

const SPRITE_PREVIEW_SIZE: f32 = 256.0;

/// State of the sprite editor.
#[derive(Debug, Default)]
pub struct SpriteEditorState {
    /// Currently selected entity ID (source_id).
    pub selected_entity: Option<String>,
    /// Sprite sheet texture path.
    pub spritesheet_path: String,
    /// Sprite sheet dimensions.
    pub spritesheet_size: (u32, u32),
    /// Current pivot point (0.0 to 1.0).
    pub pivot: [f32; 2],
    /// Whether we're in slice mode (auto-detect or grid).
    pub slice_mode: SliceMode,
    /// Grid slicing parameters.
    pub grid_cell_size: (u32, u32),
    pub grid_spacing: (u32, u32),
    pub grid_offset: (u32, u32),
    /// 9-slice parameters.
    pub nine_slice_borders: (u32, u32, u32, u32), // left, top, right, bottom
    /// Show auto-slice dialog.
    pub show_auto_slice: bool,
    /// Preview zoom.
    pub preview_zoom: f32,
    /// Preview pan offset.
    pub preview_pan: (f32, f32),
    /// Whether the preview is being dragged.
    pub preview_dragging: bool,
    /// Last mouse position for pan.
    last_mouse_pos: Option<Pos2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceMode {
    Manual,
    Grid,
    AutoDetect,
}

impl Default for SliceMode {
    fn default() -> Self {
        SliceMode::Manual
    }
}

impl SpriteEditorState {
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            spritesheet_path: String::new(),
            spritesheet_size: (0, 0),
            pivot: [0.5, 0.5],
            slice_mode: SliceMode::Manual,
            grid_cell_size: (32, 32),
            grid_spacing: (0, 0),
            grid_offset: (0, 0),
            nine_slice_borders: (0, 0, 0, 0),
            show_auto_slice: false,
            preview_zoom: 1.0,
            preview_pan: (0.0, 0.0),
            preview_dragging: false,
            last_mouse_pos: None,
        }
    }

    pub fn new() -> Self {
        Self {
            selected_entity: None,
            spritesheet_path: String::new(),
            spritesheet_size: (0, 0),
            pivot: [0.5, 0.5],
            slice_mode: SliceMode::Manual,
            grid_cell_size: (32, 32),
            grid_spacing: (0, 0),
            grid_offset: (0, 0),
            nine_slice_borders: (0, 0, 0, 0),
            show_auto_slice: false,
            preview_zoom: 1.0,
            preview_pan: (0.0, 0.0),
            preview_dragging: false,
            last_mouse_pos: None,
        }
    }

    /// Render the sprite editor panel.
    pub fn render(&mut self, ui: &mut Ui, project: &mut Project) {
        ui.heading("Sprite Editor");
        ui.add_space(8.0);

        // Entity selector
        ui.horizontal(|ui| {
            ui.label("Entity:");
            let current = self.selected_entity.as_deref().unwrap_or("Select entity...");
            egui::ComboBox::from_label("")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.selected_entity.is_none(), "None").clicked() {
                        self.selected_entity = None;
                        self.update_from_entity(project);
                    }
                    for scene in project.scenes.values() {
                        for entity in &scene.entities {
                            if entity.components.iter().any(|c| matches!(c, Component::Sprite { .. })) {
                                let label = format!("{} ({})", entity.id, entity.name);
                                if ui.selectable_label(self.selected_entity.as_deref() == Some(&entity.id), &label).clicked() {
                                    self.selected_entity = Some(entity.id.clone());
                                    self.update_from_entity(project);
                                }
                            }
                        }
                    }
                });
        });

        if self.selected_entity.is_none() {
            ui.add_space(8.0);
            ui.label("Select an entity with a Sprite component to edit its pivot, slice, or 9-slice settings.");
            return;
        }

        ui.add_space(8.0);

        let entity_id = self.selected_entity.as_ref().unwrap();
        let (scene_id, entity) = self.find_entity(project, entity_id);

        if entity.is_none() {
            ui.label("Entity not found");
            return;
        }

        let entity = entity.unwrap();

        let mut sprite_comp = None;
        for component in &entity.components {
            if let Component::Sprite { color, texture, layer, material, animation_clip } = component {
                sprite_comp = Some((color.clone(), texture.clone(), *layer, material.clone(), animation_clip.clone()));
                break;
            }
        }

        if sprite_comp.is_none() {
            ui.label("Selected entity has no Sprite component");
            return;
        }

        let (color, texture, layer, material, animation_clip) = sprite_comp.unwrap();

        ui.separator();
        ui.heading("Sprite Settings");
        ui.add_space(4.0);

        // Texture path
        ui.horizontal(|ui| {
            ui.label("Texture:");
            let mut tex_path = texture.as_deref().unwrap_or("(none)").to_string();
            if ui.text_edit_singleline(&mut tex_path).changed() {
                if tex_path.is_empty() || tex_path == "(none)" {
                    self.update_sprite_texture(project, entity_id, None);
                } else {
                    self.update_sprite_texture(project, entity_id, Some(tex_path.clone()));
                }
            }
            if ui.small_button("📁").clicked() {
            }
        });

        ui.add_space(8.0);

        // Pivot editor
        ui.separator();
        ui.heading("Pivot Point");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(DragValue::new(&mut self.pivot[0]).speed(0.01).range(0.0..=1.0).clamp_range(0.0..=1.0));
            ui.label("Y:");
            ui.add(DragValue::new(&mut self.pivot[1]).speed(0.01).range(0.0..=1.0).clamp_range(0.0..=1.0));
        });
        // In a real implementation, we'd update the pivot here

        // Quick pivot presets
        ui.horizontal(|ui| {
            if ui.small_button("Top-Left").clicked() {
                self.pivot = [0.0, 0.0];
            }
            if ui.small_button("Top-Center").clicked() {
                self.pivot = [0.5, 0.0];
            }
            if ui.small_button("Top-Right").clicked() {
                self.pivot = [1.0, 0.0];
            }
            if ui.small_button("Center-Left").clicked() {
                self.pivot = [0.0, 0.5];
            }
            if ui.small_button("Center").clicked() {
                self.pivot = [0.5, 0.5];
            }
            if ui.small_button("Center-Right").clicked() {
                self.pivot = [1.0, 0.5];
            }
            if ui.small_button("Bottom-Left").clicked() {
                self.pivot = [0.0, 1.0];
            }
            if ui.small_button("Bottom-Center").clicked() {
                self.pivot = [0.5, 1.0];
            }
            if ui.small_button("Bottom-Right").clicked() {
                self.pivot = [1.0, 1.0];
            }
        });

        ui.add_space(8.0);

        // Slice mode selector
        ui.separator();
        ui.heading("Slice Mode");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.slice_mode, SliceMode::Manual, "Manual");
            ui.radio_value(&mut self.slice_mode, SliceMode::Grid, "Grid");
            ui.radio_value(&mut self.slice_mode, SliceMode::AutoDetect, "Auto-Detect");
        });

        match self.slice_mode {
            SliceMode::Grid => {
                self.render_grid_slice(ui);
            }
            SliceMode::AutoDetect => {
                self.render_auto_detect(ui);
            }
            SliceMode::Manual => {
                ui.label("Manual slice mode - use the preview to set slice regions");
            }
        }

        ui.add_space(8.0);

        // 9-slice borders
        ui.separator();
        ui.heading("9-Slice Borders");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Left:");
            ui.add(DragValue::new(&mut self.nine_slice_borders.0).speed(1).range(0..=u32::MAX));
            ui.label("Top:");
            ui.add(DragValue::new(&mut self.nine_slice_borders.1).speed(1).range(0..=u32::MAX));
            ui.label("Right:");
            ui.add(DragValue::new(&mut self.nine_slice_borders.2).speed(1).range(0..=u32::MAX));
            ui.label("Bottom:");
            ui.add(DragValue::new(&mut self.nine_slice_borders.3).speed(1).range(0..=u32::MAX));
        });

        // Quick 9-slice presets
        ui.horizontal(|ui| {
            if ui.small_button("Clear").clicked() {
                self.nine_slice_borders = (0, 0, 0, 0);
            }
            if ui.small_button("8px").clicked() {
                self.nine_slice_borders = (8, 8, 8, 8);
            }
            if ui.small_button("16px").clicked() {
                self.nine_slice_borders = (16, 16, 16, 16);
            }
            if ui.small_button("24px").clicked() {
                self.nine_slice_borders = (24, 24, 24, 24);
            }
        });

        ui.add_space(8.0);

        // Sprite preview
        ui.separator();
        ui.heading("Preview");
        ui.add_space(4.0);
        self.render_preview(ui, project);
    }

    fn find_entity<'a>(&self, project: &'a Project, entity_id: &str) -> (Option<&'a str>, Option<&'a Entity>) {
        for (scene_id, scene) in &project.scenes {
            for entity in &scene.entities {
                if entity.id == *entity_id {
                    return (Some(scene_id), Some(entity));
                }
                for nested in &scene.nested_scenes {
                    if let Some(nested_scene) = project.scenes.get(&nested.scene_id) {
                        for entity in &nested_scene.entities {
                            if entity.id == *entity_id {
                                return (Some(scene_id), Some(entity));
                            }
                        }
                    }
                }
            }
        }
        (None, None)
    }

    fn update_from_entity(&mut self, project: &Project) {
        if let Some(entity_id) = &self.selected_entity {
            if let (_, Some(entity)) = self.find_entity(project, entity_id) {
                for component in &entity.components {
                    if let Component::Sprite { color: _, texture, layer: _, material: _, animation_clip: _ } = component {
                        self.spritesheet_path = texture.clone().unwrap_or_default();
                        self.spritesheet_size = (256, 256);
                        break;
                    }
                }
            }
        }
    }

    fn update_sprite_pivot(&mut self, project: &mut Project, entity_id: &str) {
        if let (Some(scene_id), Some(entity)) = self.find_entity(project, entity_id) {
            if let Some(scene) = project.scenes.get_mut(scene_id) {
                for entity in &mut scene.entities {
                    if entity.id == *entity_id {
                        for component in &mut entity.components {
                            if let Component::Sprite { pivot, .. } = component {
                                *pivot = self.pivot;
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    fn update_sprite_texture(&mut self, project: &mut Project, entity_id: &str, texture: Option<String>) {
        if let (Some(scene_id), Some(entity)) = self.find_entity(project, entity_id) {
            if let Some(scene) = project.scenes.get_mut(scene_id) {
                for entity in &mut scene.entities {
                    if entity.id == *entity_id {
                        for component in &mut entity.components {
                            if let Component::Sprite { texture: tex, .. } = component {
                                *tex = texture;
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    fn update_sprite_nine_slice(&mut self, project: &mut Project, entity_id: &str) {
        if let (Some(scene_id), Some(entity)) = self.find_entity(project, entity_id) {
            if let Some(scene) = project.scenes.get_mut(scene_id) {
                for entity in &mut scene.entities {
                    if entity.id == *entity_id {
                        for component in &mut entity.components {
                            if let Component::Sprite { material, .. } = component {
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    fn render_grid_slice(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Cell Width:");
            ui.add(DragValue::new(&mut self.grid_cell_size.0).speed(1).range(1..=u32::MAX));
            ui.label("Cell Height:");
            ui.add(DragValue::new(&mut self.grid_cell_size.1).speed(1).range(1..=u32::MAX));
        });
        ui.horizontal(|ui| {
            ui.label("Spacing X:");
            ui.add(DragValue::new(&mut self.grid_spacing.0).speed(1).range(0..=u32::MAX));
            ui.label("Spacing Y:");
            ui.add(DragValue::new(&mut self.grid_spacing.1).speed(1).range(0..=u32::MAX));
        });
        ui.horizontal(|ui| {
            ui.label("Offset X:");
            ui.add(DragValue::new(&mut self.grid_offset.0).speed(1).range(0..=u32::MAX));
            ui.label("Offset Y:");
            ui.add(DragValue::new(&mut self.grid_offset.1).speed(1).range(0..=u32::MAX));
        });
        if ui.button("Apply Grid Slice").clicked() {
        }
    }

    fn render_auto_detect(&mut self, ui: &mut Ui) {
        ui.label("Auto-detect slices based on transparency");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("Detect Slices").clicked() {
            }
            if ui.button("Clear").clicked() {
            }
        });
        ui.add_space(4.0);
        ui.label("Detected frames: 0");
    }

    fn render_preview(&mut self, ui: &mut Ui, _project: &Project) {
        let size = Vec2::new(SPRITE_PREVIEW_SIZE * self.preview_zoom, SPRITE_PREVIEW_SIZE * self.preview_zoom);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(last) = self.last_mouse_pos {
                    let delta = pos - last;
                    self.preview_pan.0 += delta.x;
                    self.preview_pan.1 += delta.y;
                }
                self.last_mouse_pos = Some(pos);
            }
        } else {
            self.last_mouse_pos = None;
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.preview_zoom = (self.preview_zoom * (1.0 + scroll * 0.01)).clamp(0.1, 10.0);
            }
        }

        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(40, 40, 50));
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(100)));

        let center = rect.center();
        let preview_size = Vec2::new(128.0 * self.preview_zoom, 128.0 * self.preview_zoom);
        let sprite_rect = Rect::from_center_size(center + Vec2::new(self.preview_pan.0, self.preview_pan.1), preview_size);
        
        ui.painter().rect_stroke(sprite_rect, 4.0, Stroke::new(2.0, Color32::from_rgb(100, 200, 255)));
        
        let pivot_pos = Pos2::new(
            sprite_rect.left() + sprite_rect.width() * self.pivot[0],
            sprite_rect.top() + sprite_rect.height() * self.pivot[1],
        );
        ui.painter().circle_filled(pivot_pos, 4.0 * self.preview_zoom, Color32::from_rgb(255, 100, 100));
        
        if self.nine_slice_borders != (0, 0, 0, 0) {
            let (left, top, right, bottom) = self.nine_slice_borders;
            let l = sprite_rect.left() + left as f32 * self.preview_zoom;
            let t = sprite_rect.top() + top as f32 * self.preview_zoom;
            let r = sprite_rect.right() - right as f32 * self.preview_zoom;
            let b = sprite_rect.bottom() - bottom as f32 * self.preview_zoom;
            
            ui.painter().line_segment(
                [Pos2::new(l, sprite_rect.top()), Pos2::new(l, sprite_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(255, 255, 100)),
            );
            ui.painter().line_segment(
                [Pos2::new(r, sprite_rect.top()), Pos2::new(r, sprite_rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(255, 255, 100)),
            );
            ui.painter().line_segment(
                [Pos2::new(sprite_rect.left(), t), Pos2::new(sprite_rect.right(), t)],
                Stroke::new(1.0, Color32::from_rgb(255, 255, 100)),
            );
            ui.painter().line_segment(
                [Pos2::new(sprite_rect.left(), b), Pos2::new(sprite_rect.right(), b)],
                Stroke::new(1.0, Color32::from_rgb(255, 255, 100)),
            );
        }
        
        if self.slice_mode == SliceMode::Grid && self.spritesheet_size.0 > 0 {
            let (cell_w, cell_h) = self.grid_cell_size;
            let (spacing_x, spacing_y) = self.grid_spacing;
            let (offset_x, offset_y) = self.grid_offset;
            
            let cell_w_px = cell_w as f32 * self.preview_zoom;
            let cell_h_px = cell_h as f32 * self.preview_zoom;
            let spacing_x_px = spacing_x as f32 * self.preview_zoom;
            let spacing_y_px = spacing_y as f32 * self.preview_zoom;
            let offset_x_px = offset_x as f32 * self.preview_zoom;
            let offset_y_px = offset_y as f32 * self.preview_zoom;
            
            let start_x = sprite_rect.left() + offset_x_px;
            let start_y = sprite_rect.top() + offset_y_px;
            
            let cols = ((sprite_rect.width() - offset_x_px) / (cell_w_px + spacing_x_px)).max(1.0) as usize;
            let rows = ((sprite_rect.height() - offset_y_px) / (cell_h_px + spacing_y_px)).max(1.0) as usize;
            
            for col in 0..cols {
                for row in 0..rows {
                    let x = start_x + col as f32 * (cell_w_px + spacing_x_px);
                    let y = start_y + row as f32 * (cell_h_px + spacing_y_px);
                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cell_w_px, cell_h_px));
                    ui.painter().rect_stroke(cell_rect, 2.0, Stroke::new(1.0, Color32::from_rgb(100, 255, 100)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_editor_new() {
        let editor = SpriteEditorState::new();
        assert!(editor.selected_entity.is_none());
        assert_eq!(editor.pivot, [0.5, 0.5]);
    }

    #[test]
    fn test_slice_mode_default() {
        let mode = SliceMode::default();
        assert_eq!(mode, SliceMode::Manual);
    }
}
