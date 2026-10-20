//! Editor panel for authoring tilemaps.
//!
//! The panel edits the tilemap data of the open project directly: it lists
//! the tilemaps and their layers, lets the user pick a tileset, paint tiles
//! on a chunk-sized grid, and marks tiles as solid.
//!
//! Painting works on the chunk grid, so the panel never has to allocate one
//! widget per tile of a large map.

use egui::{Color32, Rect, Sense, Stroke, Ui, Vec2};
use kiln_core::tilemap::EMPTY_TILE;
use kiln_core::{Project, TileMap};

/// How many tile columns the painting grid shows at once.
const VISIBLE_COLS: u32 = 16;
/// How many tile rows the painting grid shows at once.
const VISIBLE_ROWS: u32 = 16;
/// Size of one tile on screen, in points.
const TILE_PIXELS: f32 = 16.0;

/// The state of the tilemap authoring panel.
#[derive(Debug, Default)]
pub struct TilemapEditorState {
    /// Id of the tilemap being edited.
    pub selected_map: Option<String>,
    /// Index of the layer being edited inside the selected tilemap.
    pub selected_layer: usize,
    /// Tile id painted with the brush.
    pub brush_tile: u16,
    /// Scroll offset of the painting grid, in tiles.
    pub scroll: [i32; 2],
}

impl TilemapEditorState {
    /// Create an empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// The tilemap that is currently being edited.
    pub fn current_map<'a>(&self, project: &'a Project) -> Option<&'a TileMap> {
        let id = self.selected_map.as_ref()?;
        project.tilemaps.get(id)
    }

    /// The tilemap that is currently being edited, mutably.
    pub fn current_map_mut<'a>(&mut self, project: &'a mut Project) -> Option<&'a mut TileMap> {
        let id = self.selected_map.clone()?;
        project.tilemaps.get_mut(&id)
    }

    /// Select a tilemap by id and reset the layer selection.
    pub fn select_map(&mut self, id: Option<String>) {
        self.selected_map = id;
        self.selected_layer = 0;
    }

    /// Paint a tile into the selected layer of the selected tilemap.
    pub fn paint(&mut self, project: &mut Project, tx: i32, ty: i32, tile: u16) {
        let Some(map) = self.current_map_mut(project) else {
            return;
        };
        let Some(layer) = map.layers.get_mut(self.selected_layer) else {
            return;
        };
        layer.set_tile_at(tx, ty, tile);
    }

    /// Paint the brush tile into the selected layer.
    pub fn paint_brush(&mut self, project: &mut Project, tx: i32, ty: i32) {
        let tile = self.brush_tile;
        self.paint(project, tx, ty, tile);
    }

    /// Toggle the solid flag of the brush tile in the selected tileset.
    pub fn toggle_brush_solid(&mut self, project: &mut Project, tileset_id: &str) {
        let tile = self.brush_tile as usize;
        let Some(tileset) = project.tilesets.get_mut(tileset_id) else {
            return;
        };
        if tileset.solid.len() <= tile {
            tileset.solid.resize(tile + 1, false);
        }
        tileset.solid[tile] = !tileset.solid[tile];
    }

    /// Whether the brush tile is solid in the given tileset.
    pub fn brush_is_solid(&self, project: &Project, tileset_id: &str) -> bool {
        let Some(tileset) = project.tilesets.get(tileset_id) else {
            return false;
        };
        tileset
            .solid
            .get(self.brush_tile as usize)
            .copied()
            .unwrap_or(false)
    }
}

/// Draw the tilemap authoring panel.
pub fn show(ui: &mut Ui, state: &mut TilemapEditorState, project: &mut Project) {
    ui.heading("Tilemap Editor");
    ui.add_space(6.0);

    // --- Tilemap selection -------------------------------------------------
    ui.horizontal(|ui| {
        ui.label("Tilemap:");
        let ids: Vec<String> = {
            let mut ids: Vec<String> = project.tilemaps.keys().cloned().collect();
            ids.sort();
            ids
        };
        if ids.is_empty() {
            ui.label("none");
        } else {
            egui::ComboBox::from_id_salt("tilemap_select")
                .selected_text(state.selected_map.clone().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for id in &ids {
                        let selected = state.selected_map.as_deref() == Some(id.as_str());
                        if ui.selectable_label(selected, id).clicked() {
                            state.select_map(Some(id.clone()));
                        }
                    }
                });
        }
        if ui.button("New").clicked() {
            let map = TileMap::default();
            let id = map.id.clone();
            project.tilemaps.insert(id.clone(), map);
            state.select_map(Some(id));
        }
    });

    ui.add_space(6.0);

    let Some(map_id) = state.selected_map.clone() else {
        ui.label("Create or select a tilemap to start painting.");
        return;
    };

    // --- Layer list --------------------------------------------------------
    let layer_count = project
        .tilemaps
        .get(&map_id)
        .map(|m| m.layers.len())
        .unwrap_or(0);

    ui.horizontal(|ui| {
        ui.label("Layer:");
        for index in 0..layer_count {
            let name = project.tilemaps[&map_id].layers[index].name.clone();
            ui.selectable_value(&mut state.selected_layer, index, name);
        }
        if ui.button("+ Layer").clicked() {
            let tileset = project.tilesets.keys().next().cloned().unwrap_or_default();
            if let Some(map) = project.tilemaps.get_mut(&map_id) {
                map.layers.push(kiln_core::TileMapLayer {
                    name: format!("Layer {}", map.layers.len()),
                    tileset,
                    ..Default::default()
                });
            }
        }
    });

    // --- Layer properties --------------------------------------------------
    let mut tileset_id = {
        let Some(map) = project.tilemaps.get_mut(&map_id) else {
            return;
        };
        let Some(layer) = map.layers.get_mut(state.selected_layer) else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut layer.name);
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut layer.z).speed(1.0));
        });
        layer.tileset.clone()
    };

    ui.horizontal(|ui| {
        ui.label("Tileset:");
        let ids: Vec<String> = {
            let mut ids: Vec<String> = project.tilesets.keys().cloned().collect();
            ids.sort();
            ids
        };
        egui::ComboBox::from_id_salt("tileset_select")
            .selected_text(tileset_id.clone())
            .show_ui(ui, |ui| {
                for id in &ids {
                    ui.selectable_value(&mut tileset_id, id.clone(), id.clone());
                }
            });
        if let Some(map) = project.tilemaps.get_mut(&map_id) {
            if let Some(layer) = map.layers.get_mut(state.selected_layer) {
                layer.tileset = tileset_id.clone();
            }
        }
    });

    ui.add_space(6.0);

    // --- Brush -------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.label("Tile:");
        ui.add(egui::DragValue::new(&mut state.brush_tile).speed(1.0));
        let solid = state.brush_is_solid(project, &tileset_id);
        if ui
            .selectable_label(solid, "Solid")
            .on_hover_text("Toggle the solid flag of the brush tile")
            .clicked()
        {
            state.toggle_brush_solid(project, &tileset_id);
        }
    });

    ui.add_space(6.0);
    show_grid(ui, state, project, &map_id, &tileset_id);
}

/// Draw the painting grid of the selected layer.
fn show_grid(
    ui: &mut Ui,
    state: &mut TilemapEditorState,
    project: &mut Project,
    map_id: &str,
    tileset_id: &str,
) {
    let (tile_size, start_x, start_y) = {
        let Some(map) = project.tilemaps.get(map_id) else {
            return;
        };
        (map.tile_size, state.scroll[0], state.scroll[1])
    };

    let size = Vec2::new(
        VISIBLE_COLS as f32 * TILE_PIXELS,
        VISIBLE_ROWS as f32 * TILE_PIXELS,
    );

    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();

    let mut pending: Option<(i32, i32, bool)> = None;

    for row in 0..VISIBLE_ROWS {
        for col in 0..VISIBLE_COLS {
            let tx = start_x + col as i32;
            let ty = start_y + row as i32;
            let cell = Rect::from_min_size(
                rect.min + Vec2::new(col as f32 * TILE_PIXELS, row as f32 * TILE_PIXELS),
                Vec2::splat(TILE_PIXELS),
            );

            let tile = project
                .tilemaps
                .get(map_id)
                .and_then(|m| m.layers.get(state.selected_layer))
                .map(|l| l.tile_at(tx, ty))
                .unwrap_or(EMPTY_TILE);

            painter.rect_filled(cell, 0.0, tile_color(project, tileset_id, tile));
            painter.rect_stroke(cell, 0.0_f32, Stroke::new(1.0_f32, Color32::from_gray(60)));

            if let Some(pos) = response.interact_pointer_pos() {
                if cell.contains(pos) {
                    if response.clicked() {
                        pending = Some((tx, ty, false));
                    } else if response.secondary_clicked() {
                        pending = Some((tx, ty, true));
                    }
                }
            }
        }
    }

    if let Some((tx, ty, erase)) = pending {
        if erase {
            state.paint(project, tx, ty, EMPTY_TILE);
        } else {
            state.paint_brush(project, tx, ty);
        }
    }

    ui.horizontal(|ui| {
        if ui.button("<").clicked() {
            state.scroll[0] -= VISIBLE_COLS as i32;
        }
        if ui.button(">").clicked() {
            state.scroll[0] += VISIBLE_COLS as i32;
        }
        if ui.button("^").clicked() {
            state.scroll[1] += VISIBLE_ROWS as i32;
        }
        if ui.button("v").clicked() {
            state.scroll[1] -= VISIBLE_ROWS as i32;
        }
        ui.label(format!("origin {}, {}", start_x, start_y));
        ui.label(format!("tile {} px", tile_size[0] as i32));
    });
}

/// Colour used to draw a tile in the painting grid.
///
/// Solid tiles are drawn red so they stand out; empty tiles are drawn dark.
fn tile_color(project: &Project, tileset_id: &str, tile: u16) -> Color32 {
    if tile == EMPTY_TILE {
        return Color32::from_rgb(28, 28, 32);
    }
    let solid = project
        .tilesets
        .get(tileset_id)
        .map(|ts| ts.is_solid(tile))
        .unwrap_or(false);
    if solid {
        Color32::from_rgb(120, 60, 60)
    } else {
        Color32::from_rgb(90, 110, 140)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_map() -> Project {
        let mut project = Project::default();
        let mut map = TileMap::default();
        map.layers.push(Default::default());
        project.tilemaps.insert(map.id.clone(), map);
        project
    }

    #[test]
    fn paint_writes_into_the_selected_layer() {
        let mut project = project_with_map();
        let mut state = TilemapEditorState::new();
        state.selected_map = project.tilemaps.keys().next().cloned();
        state.brush_tile = 3;

        state.paint_brush(&mut project, 2, 5);

        let map = project.tilemaps.values().next().expect("map");
        assert_eq!(map.layers[0].tile_at(2, 5), 3);
    }

    #[test]
    fn painting_without_a_map_is_a_no_op() {
        let mut project = Project::default();
        let mut state = TilemapEditorState::new();
        state.paint_brush(&mut project, 1, 1);
    }

    #[test]
    fn solid_flag_can_be_toggled() {
        let mut project = project_with_map();
        let ts_id = "terrain".to_string();
        project.tilesets.insert(
            ts_id.clone(),
            kiln_core::TileSet {
                texture: "tiles/t.png".to_string(),
                tile_size: [16, 16],
                columns: 2,
                rows: 2,
                solid: Vec::new(),
            },
        );

        let mut state = TilemapEditorState::new();
        state.brush_tile = 1;

        assert!(!state.brush_is_solid(&project, &ts_id));
        state.toggle_brush_solid(&mut project, &ts_id);
        assert!(state.brush_is_solid(&project, &ts_id));
    }
}
