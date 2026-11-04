//! Textured multilayer tilemap authoring with bounded fill and terrain brushes.
use crate::image_preview::{PreviewImage, PreviewImages};
use kiln_core::tilemap::EMPTY_TILE;
use kiln_core::{Project, TileMap, TileSet};
use std::collections::{HashSet, VecDeque};

const SIDE: i32 = 16;
const CELL: f32 = 22.0;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileTool {
    #[default]
    Brush,
    Erase,
    Fill,
    Terrain,
}
#[derive(Debug, Default)]
pub struct TilemapEditorState {
    pub selected_map: Option<String>,
    pub target_map: Option<String>,
    pub selected_layer: usize,
    pub brush_tile: u16,
    pub scroll: [i32; 2],
    pub tool: TileTool,
    pub terrain: Option<String>,
    pub show_collision: bool,
    pub tileset_path: String,
    pub tile_pixels: [u32; 2],
    pub palette_page: u32,
    images: PreviewImages,
    last_tile: Option<[i32; 2]>,
    stroke: Option<TileMap>,
    undo: Vec<TileMap>,
    redo: Vec<TileMap>,
}
impl TilemapEditorState {
    pub fn new() -> Self {
        Self {
            show_collision: true,
            tile_pixels: [16; 2],
            ..Default::default()
        }
    }
    pub fn current_map<'a>(&self, p: &'a Project) -> Option<&'a TileMap> {
        p.tilemaps.get(self.selected_map.as_ref()?)
    }
    pub fn current_map_mut<'a>(&self, p: &'a mut Project) -> Option<&'a mut TileMap> {
        p.tilemaps.get_mut(self.selected_map.as_ref()?)
    }
    pub fn select_map(&mut self, id: Option<String>) {
        self.selected_map = id;
        self.selected_layer = 0;
        self.undo.clear();
        self.redo.clear();
        self.stroke = None;
        self.last_tile = None;
    }
    pub fn paint(&mut self, p: &mut Project, x: i32, y: i32, tile: u16) {
        let layer = self.selected_layer;
        if let Some(layer) = self
            .current_map_mut(p)
            .and_then(|m| m.layers.get_mut(layer))
        {
            layer.set_tile_at(x, y, tile);
        }
    }
    pub fn paint_brush(&mut self, p: &mut Project, x: i32, y: i32) {
        self.paint(p, x, y, self.brush_tile);
    }
    pub fn toggle_brush_solid(&mut self, p: &mut Project, id: &str) {
        if let Some(set) = p.tilesets.get_mut(id) {
            let index = self.brush_tile as usize;
            if index < set.tile_count() as usize {
                set.solid.resize(set.tile_count() as usize, false);
                set.solid[index] = !set.solid[index];
            }
        }
    }
    pub fn brush_is_solid(&self, p: &Project, id: &str) -> bool {
        p.tilesets
            .get(id)
            .is_some_and(|set| set.is_solid(self.brush_tile))
    }
    /// Fill a finite connected region atomically; never expand unbounded empty space.
    pub fn fill(
        &mut self,
        p: &mut Project,
        start: [i32; 2],
        bounds: [i32; 4],
        tile: u16,
    ) -> Result<usize, String> {
        let [left, bottom, right, top] = bounds;
        if left < 0
            || bottom < 0
            || right <= left
            || top <= bottom
            || (right as i64 - left as i64) * (top as i64 - bottom as i64) > 65_536
        {
            return Err("Fill bounds must cover 1–65536 nonnegative cells".into());
        }
        if start[0] < left || start[1] < bottom || start[0] >= right || start[1] >= top {
            return Ok(0);
        }
        let layer_index = self.selected_layer;
        let layer = self
            .current_map_mut(p)
            .and_then(|m| m.layers.get_mut(layer_index))
            .ok_or("Select a map layer")?;
        let old = layer.tile_at(start[0], start[1]);
        if old == tile {
            return Ok(0);
        }
        let mut queue = VecDeque::from([start]);
        let mut visited = HashSet::new();
        let mut changed = 0;
        while let Some([x, y]) = queue.pop_front() {
            if x < left
                || y < bottom
                || x >= right
                || y >= top
                || !visited.insert([x, y])
                || layer.tile_at(x, y) != old
            {
                continue;
            }
            layer.set_tile_at(x, y, tile);
            changed += 1;
            queue.extend([[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]]);
        }
        Ok(changed)
    }
    /// Four-neighbor terrain masks: up=1, right=2, down=4, left=8.
    pub fn paint_terrain(
        &mut self,
        p: &mut Project,
        x: i32,
        y: i32,
        erase: bool,
    ) -> Result<(), String> {
        let rule = p
            .terrains
            .get(self.terrain.as_ref().ok_or("Select terrain")?)
            .cloned()
            .ok_or("Missing terrain")?;
        let count = p
            .tilesets
            .get(&rule.tileset)
            .ok_or("Terrain tileset is missing")?
            .tile_count();
        if rule
            .tiles
            .iter()
            .any(|tile| *tile == EMPTY_TILE || *tile as u32 >= count)
        {
            return Err("Terrain variants must refer to tiles in the selected tileset".into());
        }
        let layer_index = self.selected_layer;
        let layer = self
            .current_map_mut(p)
            .and_then(|m| m.layers.get_mut(layer_index))
            .ok_or("Select a map layer")?;
        if layer.tileset != rule.tileset {
            return Err("Terrain belongs to another tileset".into());
        }
        if x < 0 || y < 0 {
            return Ok(());
        }
        layer.set_tile_at(x, y, if erase { EMPTY_TILE } else { rule.tiles[0] });
        let neighbors = |x, y| [[x, y + 1], [x + 1, y], [x, y - 1], [x - 1, y]];
        let mut cells = vec![[x, y]];
        cells.extend(neighbors(x, y));
        for [x, y] in cells {
            if !rule.tiles.contains(&layer.tile_at(x, y)) || layer.tile_at(x, y) == EMPTY_TILE {
                continue;
            }
            let mut mask = 0;
            for (bit, [nx, ny]) in neighbors(x, y).into_iter().enumerate() {
                let tile = layer.tile_at(nx, ny);
                if tile != EMPTY_TILE && rule.tiles.contains(&tile) {
                    mask |= 1 << bit;
                }
            }
            layer.set_tile_at(x, y, rule.tiles[mask]);
        }
        Ok(())
    }
    pub fn begin_stroke(&mut self, p: &Project) {
        if self.stroke.is_none() {
            self.stroke = self.current_map(p).cloned();
        }
    }
    pub fn end_stroke(&mut self, p: &Project) {
        if let Some(before) = self.stroke.take() {
            if self.current_map(p) != Some(&before) {
                self.undo.push(before);
                if self.undo.len() > 64 {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
        self.last_tile = None;
    }
    pub fn undo(&mut self, p: &mut Project) {
        if let Some(previous) = self.undo.pop() {
            if let Some(current) = self.current_map_mut(p) {
                let old = std::mem::replace(current, previous);
                self.redo.push(old);
            }
        }
    }
    pub fn redo(&mut self, p: &mut Project) {
        if let Some(next) = self.redo.pop() {
            if let Some(current) = self.current_map_mut(p) {
                let old = std::mem::replace(current, next);
                self.undo.push(old);
            }
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut TilemapEditorState, p: &mut Project) {
    show_with_root(ui, state, p, None);
}
pub fn show_with_root(
    ui: &mut egui::Ui,
    state: &mut TilemapEditorState,
    p: &mut Project,
    root: Option<&std::path::Path>,
) {
    ui.heading("Tilemap Editor");
    let mut ids: Vec<_> = p.tilemaps.keys().cloned().collect();
    ids.sort();
    let before = state.selected_map.clone();
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("map_select")
            .selected_text(state.selected_map.as_deref().unwrap_or("Select map"))
            .show_ui(ui, |ui| {
                for id in ids {
                    ui.selectable_value(&mut state.selected_map, Some(id.clone()), id);
                }
            });
        if ui.button("New Map").clicked() {
            let mut map = TileMap::default();
            map.layers.push(Default::default());
            let id = map.id.clone();
            p.tilemaps.insert(id.clone(), map);
            state.selected_map = Some(id);
        }
    });
    if before != state.selected_map {
        state.select_map(state.selected_map.clone());
    }
    ui.collapsing("Create Tileset", |ui| {
        ui.horizontal(|ui| {
            ui.label("Image path");
            ui.text_edit_singleline(&mut state.tileset_path);
        });
        ui.horizontal(|ui| {
            for value in &mut state.tile_pixels {
                ui.add(egui::DragValue::new(value).range(1..=8192));
            }
        });
        if ui.button("Create from Image").clicked() {
            match state.images.load(ui.ctx(), root, &state.tileset_path) {
                Ok(image) => {
                    let [w, h] = state.tile_pixels;
                    let columns = image.size[0] / w;
                    let rows = image.size[1] / h;
                    if columns > 0
                        && rows > 0
                        && columns * rows < EMPTY_TILE as u32
                        && image.size[0] % w == 0
                        && image.size[1] % h == 0
                    {
                        let id = format!("tileset_{}", kiln_core::next_id());
                        p.tilesets.insert(
                            id.clone(),
                            TileSet {
                                texture: state.tileset_path.clone(),
                                tile_size: [w, h],
                                columns,
                                rows,
                                solid: vec![],
                            },
                        );
                        let index = state.selected_layer;
                        if let Some(layer) = state
                            .current_map_mut(p)
                            .and_then(|m| m.layers.get_mut(index))
                        {
                            layer.tileset = id;
                        }
                    } else {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Use a tight grid with 1–65534 tiles",
                        );
                    }
                }
                Err(error) => {
                    ui.label(error);
                }
            }
        }
    });
    let Some(id) = state.selected_map.clone() else {
        return;
    };
    ui.horizontal(|ui| {
        if ui.button("Add Layer").clicked() {
            if let Some(map) = p.tilemaps.get_mut(&id) {
                map.layers.push(kiln_core::TileMapLayer {
                    name: format!("Layer {}", map.layers.len()),
                    tileset: p.tilesets.keys().min().cloned().unwrap_or_default(),
                    ..Default::default()
                });
                state.selected_layer = map.layers.len() - 1;
            }
        }
        if ui.button("Undo Paint").clicked() {
            state.undo(p);
        }
        if ui.button("Redo Paint").clicked() {
            state.redo(p);
        }
    });
    let Some(map) = p.tilemaps.get_mut(&id) else {
        return;
    };
    ui.horizontal(|ui| {
        for (index, layer) in map.layers.iter().enumerate() {
            ui.selectable_value(&mut state.selected_layer, index, &layer.name);
        }
    });
    ui.horizontal(|ui| {
        ui.label("World tile size");
        for size in &mut map.tile_size {
            ui.add(egui::DragValue::new(size).range(0.01..=8192.0));
        }
    });
    let Some(layer) = map.layers.get_mut(state.selected_layer) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut layer.name);
        ui.add(egui::DragValue::new(&mut layer.z).prefix("Layer Z "));
        for offset in &mut layer.offset {
            ui.add(egui::DragValue::new(offset).prefix("Offset "));
        }
    });
    let mut sets: Vec<_> = p.tilesets.keys().cloned().collect();
    sets.sort();
    egui::ComboBox::from_id_salt("tileset_select")
        .selected_text(&layer.tileset)
        .show_ui(ui, |ui| {
            for id in &sets {
                ui.selectable_value(&mut layer.tileset, id.clone(), id);
            }
        });
    let set_id = layer.tileset.clone();
    ui.horizontal(|ui| {
        for tool in [
            TileTool::Brush,
            TileTool::Erase,
            TileTool::Fill,
            TileTool::Terrain,
        ] {
            ui.selectable_value(&mut state.tool, tool, format!("{tool:?}"));
        }
        ui.checkbox(&mut state.show_collision, "Collision preview");
    });
    if state.tool == TileTool::Terrain {
        let mut ids: Vec<_> = p
            .terrains
            .iter()
            .filter(|(_, rule)| rule.tileset == set_id)
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        egui::ComboBox::from_id_salt("terrain")
            .selected_text(state.terrain.as_deref().unwrap_or("Select terrain"))
            .show_ui(ui, |ui| {
                for id in ids {
                    ui.selectable_value(&mut state.terrain, Some(id.clone()), id);
                }
            });
        if ui.button("New Terrain").clicked() {
            let id = format!("terrain_{}", kiln_core::next_id());
            p.terrains.insert(
                id.clone(),
                kiln_core::tilemap::TerrainSet {
                    tileset: set_id.clone(),
                    tiles: [state.brush_tile; 16],
                },
            );
            state.terrain = Some(id);
        }
        if let Some(rule) = state.terrain.as_ref().and_then(|id| p.terrains.get_mut(id)) {
            ui.collapsing("Neighbor masks (up/right/down/left)", |ui| {
                for (mask, tile) in rule.tiles.iter_mut().enumerate() {
                    ui.add(
                        egui::DragValue::new(tile)
                            .prefix(format!("{mask:04b}: "))
                            .range(0..=65534),
                    );
                }
            });
        }
    }
    let set = p.tilesets.get(&set_id).cloned();
    let image = set
        .as_ref()
        .and_then(|set| state.images.load(ui.ctx(), root, &set.texture).ok());
    let (area, response) = ui.allocate_exact_size(
        egui::vec2(SIDE as f32 * CELL, SIDE as f32 * CELL),
        egui::Sense::click_and_drag(),
    );
    for row in 0..SIDE {
        for col in 0..SIDE {
            let x = state.scroll[0] + col;
            let y = state.scroll[1] + SIDE - 1 - row;
            let tile = p.tilemaps[&id].layers[state.selected_layer].tile_at(x, y);
            let cell = egui::Rect::from_min_size(
                area.min + egui::vec2(col as f32 * CELL, row as f32 * CELL),
                egui::Vec2::splat(CELL),
            );
            paint_cell(
                ui,
                cell,
                tile,
                set.as_ref(),
                image.as_ref(),
                state.show_collision,
            );
        }
    }
    let down = ui.input(|input| input.pointer.primary_down() || input.pointer.secondary_down());
    if response.hovered() && (down || response.clicked() || response.secondary_clicked()) {
        if let Some(point) = ui.input(|input| input.pointer.interact_pos()) {
            let x = state.scroll[0] + ((point.x - area.left()) / CELL) as i32;
            let y = state.scroll[1] + SIDE - 1 - ((point.y - area.top()) / CELL) as i32;
            let erase =
                ui.input(|input| input.pointer.secondary_down()) || state.tool == TileTool::Erase;
            state.begin_stroke(p);
            match state.tool {
                TileTool::Fill => {
                    if response.clicked() || response.secondary_clicked() {
                        let _ = state.fill(
                            p,
                            [x, y],
                            [
                                state.scroll[0],
                                state.scroll[1],
                                state.scroll[0] + SIDE,
                                state.scroll[1] + SIDE,
                            ],
                            if erase { EMPTY_TILE } else { state.brush_tile },
                        );
                    }
                }
                TileTool::Terrain => {
                    if let Err(error) = state.paint_terrain(p, x, y, erase) {
                        ui.label(error);
                    }
                }
                _ => {
                    let [sx, sy] = state.last_tile.unwrap_or([x, y]);
                    let steps = (x - sx).abs().max((y - sy).abs()).max(1);
                    for step in 0..=steps {
                        state.paint(
                            p,
                            sx + (x - sx) * step / steps,
                            sy + (y - sy) * step / steps,
                            if erase { EMPTY_TILE } else { state.brush_tile },
                        );
                    }
                }
            }
            state.last_tile = Some([x, y]);
        }
    }
    if !down {
        state.end_stroke(p);
    }
    ui.horizontal(|ui| {
        for (label, axis, delta) in [
            ("Left", 0, -SIDE),
            ("Right", 0, SIDE),
            ("Down", 1, -SIDE),
            ("Up", 1, SIDE),
        ] {
            if ui.button(label).clicked() {
                state.scroll[axis] = (state.scroll[axis] + delta).max(0);
            }
        }
        ui.label(format!("{}, {}", state.scroll[0], state.scroll[1]));
    });
    ui.horizontal(|ui| {
        ui.label(format!("Brush {}", state.brush_tile));
        if ui
            .selectable_label(state.brush_is_solid(p, &set_id), "Solid")
            .clicked()
        {
            state.toggle_brush_solid(p, &set_id);
        }
    });
    if let Some(set) = &set {
        let pages = set.tile_count().min(65535).div_ceil(64).max(1);
        state.palette_page = state.palette_page.min(pages - 1);
        ui.horizontal(|ui| {
            if ui.button("Previous Palette Page").clicked() {
                state.palette_page = state.palette_page.saturating_sub(1);
            }
            if ui.button("Next Palette Page").clicked() {
                state.palette_page = (state.palette_page + 1).min(pages - 1);
            }
            ui.label(format!("{} / {pages}", state.palette_page + 1));
        });
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("selected_layer_palette")
                    .num_columns(8)
                    .show(ui, |ui| {
                        for tile in state.palette_page * 64
                            ..((state.palette_page + 1) * 64)
                                .min(set.tile_count())
                                .min(65535)
                        {
                            let (cell, response) = ui
                                .allocate_exact_size(egui::Vec2::splat(28.0), egui::Sense::click());
                            paint_cell(
                                ui,
                                cell,
                                tile as u16,
                                Some(set),
                                image.as_ref(),
                                state.show_collision,
                            );
                            if response.clicked() {
                                state.brush_tile = tile as u16;
                            }
                            if tile as u16 == state.brush_tile {
                                ui.painter().rect_stroke(
                                    cell,
                                    0.0,
                                    egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
                                );
                            }
                            if tile % 8 == 7 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }
}
fn paint_cell(
    ui: &egui::Ui,
    cell: egui::Rect,
    tile: u16,
    set: Option<&TileSet>,
    image: Option<&PreviewImage>,
    collision: bool,
) {
    ui.painter()
        .rect_filled(cell, 0.0, egui::Color32::from_gray(25));
    if let (Some(set), Some(image)) = (set, image) {
        if let Some(uv) = set.tile_uv(tile) {
            ui.painter().image(
                image.texture.id(),
                cell,
                egui::Rect::from_min_max(egui::pos2(uv[0], uv[1]), egui::pos2(uv[2], uv[3])),
                egui::Color32::WHITE,
            );
        }
    }
    let solid = collision && set.is_some_and(|set| set.is_solid(tile));
    ui.painter().rect_stroke(
        cell,
        0.0,
        egui::Stroke::new(
            1.0_f32,
            if solid {
                egui::Color32::RED
            } else {
                egui::Color32::from_gray(60)
            },
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Project, TilemapEditorState) {
        let mut p = Project::default();
        let mut map = TileMap {
            id: "main".into(),
            ..Default::default()
        };
        map.layers.push(kiln_core::TileMapLayer {
            tileset: "terrain".into(),
            ..Default::default()
        });
        map.layers.push(Default::default());
        p.tilemaps.insert("main".into(), map);
        p.tilesets.insert(
            "terrain".into(),
            TileSet {
                columns: 4,
                rows: 4,
                ..Default::default()
            },
        );
        let mut state = TilemapEditorState::new();
        state.select_map(Some("main".into()));
        (p, state)
    }
    #[test]
    fn paint_writes_into_the_selected_layer() {
        let (mut p, mut state) = fixture();
        state.selected_layer = 1;
        state.brush_tile = 3;
        state.paint_brush(&mut p, 2, 5);
        assert_eq!(p.tilemaps["main"].layers[1].tile_at(2, 5), 3);
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(2, 5), EMPTY_TILE);
    }
    #[test]
    fn painting_without_a_map_is_a_no_op() {
        let mut p = Project::default();
        TilemapEditorState::new().paint_brush(&mut p, 1, 1);
        assert!(p.tilemaps.is_empty());
    }
    #[test]
    fn solid_flag_can_be_toggled() {
        let (mut p, mut state) = fixture();
        state.brush_tile = 1;
        assert!(!state.brush_is_solid(&p, "terrain"));
        state.toggle_brush_solid(&mut p, "terrain");
        assert!(state.brush_is_solid(&p, "terrain"));
    }
    #[test]
    fn terrain_reflows_adjacent_tiles_and_erase_without_touching_other_layers() {
        let (mut p, mut state) = fixture();
        p.terrains.insert(
            "grass".into(),
            kiln_core::tilemap::TerrainSet {
                tileset: "terrain".into(),
                tiles: std::array::from_fn(|index| index as u16),
            },
        );
        state.terrain = Some("grass".into());
        state.paint_terrain(&mut p, 1, 1, false).unwrap();
        state.paint_terrain(&mut p, 2, 1, false).unwrap();
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(1, 1), 2);
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(2, 1), 8);
        state.paint_terrain(&mut p, 1, 2, false).unwrap();
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(1, 1), 3);
        state.paint_terrain(&mut p, 2, 1, true).unwrap();
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(1, 1), 1);
        assert!(p.tilemaps["main"].layers[1].chunks.is_empty());
        let loaded = Project::from_toml(&p.to_toml().unwrap()).unwrap();
        assert_eq!(loaded.terrains, p.terrains);
        assert_eq!(loaded.tilemaps, p.tilemaps);
    }
    #[test]
    fn fill_crosses_chunks_stops_at_barriers_and_rejects_unbounded_edits() {
        let (mut p, mut state) = fixture();
        for y in 0..4 {
            state.paint(&mut p, 20, y, 1);
        }
        let before = p.tilemaps["main"].clone();
        assert!(state.fill(&mut p, [0, 0], [0, 0, i32::MAX, 5], 2).is_err());
        assert_eq!(p.tilemaps["main"], before);
        state.begin_stroke(&p);
        assert_eq!(state.fill(&mut p, [0, 0], [0, 0, 32, 4], 2).unwrap(), 80);
        state.end_stroke(&p);
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(19, 3), 2);
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(21, 3), EMPTY_TILE);
        state.undo(&mut p);
        assert_eq!(p.tilemaps["main"], before);
        state.redo(&mut p);
        assert_eq!(p.tilemaps["main"].layers[0].tile_at(17, 0), 2);
    }
}
