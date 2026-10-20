//! Tilemap data model (tilesets, layers, chunked tile storage).
//!
//! The model is plain serializable data so it can live inside a project
//! manifest ([`crate::Project`]) and be authored in the editor. Runtime
//! behaviour (culling, drawing, collision queries) lives in
//! `kiln_runtime::tilemap`.
//!
//! Tiles are stored in fixed-size chunks so a large map does not allocate one
//! entity per tile.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Width and height of a chunk, in tiles.
pub const CHUNK_SIZE: u32 = 16;

/// Tile value meaning "no tile here".
pub const EMPTY_TILE: u16 = u16::MAX;

/// A collection of tiles packed into a single image (a tileset).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileSet {
    /// Asset path of the tileset image (e.g. `"tiles/terrain.png"`).
    pub texture: String,
    /// Size of a single tile inside the image, in pixels.
    pub tile_size: [u32; 2],
    /// Number of tile columns in the image.
    pub columns: u32,
    /// Number of tile rows in the image.
    pub rows: u32,
    /// Per-tile solid flags, indexed by tile id. Missing entries mean
    /// "not solid".
    #[serde(default)]
    pub solid: Vec<bool>,
}

impl Default for TileSet {
    fn default() -> Self {
        Self {
            texture: String::new(),
            tile_size: [16, 16],
            columns: 1,
            rows: 1,
            solid: Vec::new(),
        }
    }
}

impl TileSet {
    /// Number of tiles in the set.
    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }

    /// Size of the whole tileset image, in pixels.
    pub fn image_size(&self) -> [u32; 2] {
        [
            self.tile_size[0] * self.columns,
            self.tile_size[1] * self.rows,
        ]
    }

    /// Normalised UV rect (`u0, v0, u1, v1`) of a tile, or `None` when the
    /// tile id is outside the set.
    pub fn tile_uv(&self, tile: u16) -> Option<[f32; 4]> {
        if tile == EMPTY_TILE {
            return None;
        }
        let index = tile as u32;
        if index >= self.tile_count() {
            return None;
        }
        let [tw, th] = self.tile_size;
        let [iw, ih] = self.image_size();
        let col = index % self.columns;
        let row = index / self.columns;
        let x = col * tw;
        let y = row * th;
        let iw = iw.max(1) as f32;
        let ih = ih.max(1) as f32;
        Some([
            x as f32 / iw,
            y as f32 / ih,
            (x + tw) as f32 / iw,
            (y + th) as f32 / ih,
        ])
    }

    /// Whether a tile blocks movement.
    pub fn is_solid(&self, tile: u16) -> bool {
        self.solid.get(tile as usize).copied().unwrap_or(false)
    }
}

/// A rectangular block of tiles, addressed in chunk coordinates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileChunk {
    /// Chunk column in the map.
    pub x: i32,
    /// Chunk row in the map.
    pub y: i32,
    /// `CHUNK_SIZE * CHUNK_SIZE` tile ids, row-major.
    pub tiles: Vec<u16>,
}

impl TileChunk {
    /// An empty chunk at the given chunk coordinate.
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            tiles: vec![EMPTY_TILE; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    /// Index of a tile inside the chunk's flat array.
    pub fn index(lx: u32, ly: u32) -> usize {
        (ly as usize) * CHUNK_SIZE as usize + lx as usize
    }

    /// Tile id at a local coordinate, or [`EMPTY_TILE`].
    pub fn tile(&self, lx: u32, ly: u32) -> u16 {
        if lx >= CHUNK_SIZE || ly >= CHUNK_SIZE {
            return EMPTY_TILE;
        }
        self.tiles
            .get(Self::index(lx, ly))
            .copied()
            .unwrap_or(EMPTY_TILE)
    }

    /// Set a tile at a local coordinate. Out-of-range coordinates are ignored.
    pub fn set_tile(&mut self, lx: u32, ly: u32, tile: u16) {
        if lx >= CHUNK_SIZE || ly >= CHUNK_SIZE {
            return;
        }
        let idx = Self::index(lx, ly);
        if idx < self.tiles.len() {
            self.tiles[idx] = tile;
        }
    }
}

/// One drawable layer of a tilemap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileMapLayer {
    /// Layer name, shown in the editor.
    pub name: String,
    /// Id of the tileset used by this layer.
    pub tileset: String,
    /// Z-order; higher layers are drawn on top.
    #[serde(default)]
    pub z: i32,
    /// World-space offset of the layer, in pixels.
    #[serde(default)]
    pub offset: [f32; 2],
    /// Chunks of tiles, keyed by `"cx,cy"`.
    #[serde(default)]
    pub chunks: HashMap<String, TileChunk>,
}

impl Default for TileMapLayer {
    fn default() -> Self {
        Self {
            name: "Layer".to_string(),
            tileset: String::new(),
            z: 0,
            offset: [0.0, 0.0],
            chunks: HashMap::new(),
        }
    }
}

impl TileMapLayer {
    /// Key used to store a chunk in the chunk map.
    pub fn chunk_key(cx: i32, cy: i32) -> String {
        format!("{cx},{cy}")
    }

    /// Chunk at the given chunk coordinate, if it exists.
    pub fn chunk(&self, cx: i32, cy: i32) -> Option<&TileChunk> {
        self.chunks.get(&Self::chunk_key(cx, cy))
    }

    /// Mutable chunk at the given chunk coordinate, if it exists.
    pub fn chunk_mut(&mut self, cx: i32, cy: i32) -> Option<&mut TileChunk> {
        self.chunks.get_mut(&Self::chunk_key(cx, cy))
    }

    /// Get or create the chunk at the given chunk coordinate.
    pub fn chunk_or_insert(&mut self, cx: i32, cy: i32) -> &mut TileChunk {
        self.chunks
            .entry(Self::chunk_key(cx, cy))
            .or_insert_with(|| TileChunk::new(cx, cy))
    }

    /// Tile id at map coordinates (in tiles).
    pub fn tile_at(&self, tx: i32, ty: i32) -> u16 {
        if tx < 0 || ty < 0 {
            return EMPTY_TILE;
        }
        let (cx, cy) = (tx / CHUNK_SIZE as i32, ty / CHUNK_SIZE as i32);
        let lx = (tx % CHUNK_SIZE as i32) as u32;
        let ly = (ty % CHUNK_SIZE as i32) as u32;
        match self.chunk(cx, cy) {
            Some(chunk) => chunk.tile(lx, ly),
            None => EMPTY_TILE,
        }
    }

    /// Set a tile at map coordinates (in tiles). Missing chunks are created.
    pub fn set_tile_at(&mut self, tx: i32, ty: i32, tile: u16) {
        if tx < 0 || ty < 0 {
            return;
        }
        let (cx, cy) = (tx / CHUNK_SIZE as i32, ty / CHUNK_SIZE as i32);
        let lx = (tx % CHUNK_SIZE as i32) as u32;
        let ly = (ty % CHUNK_SIZE as i32) as u32;
        self.chunk_or_insert(cx, cy).set_tile(lx, ly, tile);
    }

    /// Remove all tiles of a chunk.
    pub fn clear_chunk(&mut self, cx: i32, cy: i32) {
        if let Some(chunk) = self.chunks.get_mut(&Self::chunk_key(cx, cy)) {
            chunk.tiles.fill(EMPTY_TILE);
        }
    }
}

/// A tilemap: one or more layers drawn over a shared tile size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileMap {
    /// Id of the tilemap.
    pub id: String,
    /// Human readable name.
    pub name: String,
    /// Size of one tile in world units (pixels by default).
    #[serde(default = "default_tile_world_size")]
    pub tile_size: [f32; 2],
    /// Layers, ordered by their `z` value when drawn.
    pub layers: Vec<TileMapLayer>,
}

fn default_tile_world_size() -> [f32; 2] {
    [16.0, 16.0]
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Tilemap".to_string(),
            tile_size: default_tile_world_size(),
            layers: Vec::new(),
        }
    }
}

impl TileMap {
    /// Layers in draw order (ascending `z`).
    pub fn sorted_layers(&self) -> Vec<&TileMapLayer> {
        let mut layers: Vec<&TileMapLayer> = self.layers.iter().collect();
        layers.sort_by_key(|l| l.z);
        layers
    }

    /// Layer with the given name.
    pub fn layer(&self, name: &str) -> Option<&TileMapLayer> {
        self.layers.iter().find(|l| l.name == name)
    }

    /// Mutable layer with the given name.
    pub fn layer_mut(&mut self, name: &str) -> Option<&mut TileMapLayer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }

    /// Size of the map in tiles.
    pub fn size_in_tiles(&self) -> [i32; 2] {
        let mut w = 0;
        let mut h = 0;
        for layer in &self.layers {
            for chunk in layer.chunks.values() {
                let x = chunk.x * CHUNK_SIZE as i32 + CHUNK_SIZE as i32;
                let y = chunk.y * CHUNK_SIZE as i32 + CHUNK_SIZE as i32;
                w = w.max(x);
                h = h.max(y);
            }
        }
        [w, h]
    }

    /// Whether the given world-space rect overlaps a solid tile.
    ///
    /// `tilesets` maps a layer's `tileset` id to its tileset definition.
    /// Returns the world position of the first solid tile found.
    pub fn first_solid_hit(
        &self,
        tilesets: &HashMap<String, TileSet>,
        rect: [f32; 4],
    ) -> Option<[f32; 2]> {
        let [min_x, min_y, max_x, max_y] = rect;
        let ts = self.tile_size;
        if ts[0] <= 0.0 || ts[1] <= 0.0 {
            return None;
        }
        let tx0 = (min_x / ts[0]).floor() as i32;
        let ty0 = (min_y / ts[1]).floor() as i32;
        let tx1 = (max_x / ts[0]).floor() as i32;
        let ty1 = (max_y / ts[1]).floor() as i32;

        for layer in &self.layers {
            let Some(tileset) = tilesets.get(&layer.tileset) else {
                continue;
            };
            for ty in ty0..=ty1 {
                for tx in tx0..=tx1 {
                    let tile = layer.tile_at(tx, ty);
                    if tileset.is_solid(tile) {
                        return Some([tx as f32 * ts[0], ty as f32 * ts[1]]);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tileset() -> TileSet {
        TileSet {
            texture: "tiles/terrain.png".to_string(),
            tile_size: [16, 16],
            columns: 2,
            rows: 2,
            solid: vec![false, true, false, false],
        }
    }

    #[test]
    fn tile_uv_is_normalised() {
        let ts = tileset();
        assert_eq!(ts.tile_uv(0), Some([0.0, 0.0, 0.5, 0.5]));
        assert_eq!(ts.tile_uv(1), Some([0.5, 0.0, 1.0, 0.5]));
        assert_eq!(ts.tile_uv(2), Some([0.0, 0.5, 0.5, 1.0]));
        assert_eq!(ts.tile_uv(3), Some([0.5, 0.5, 1.0, 1.0]));
    }

    #[test]
    fn out_of_range_tiles_have_no_uv() {
        let ts = tileset();
        assert_eq!(ts.tile_uv(4), None);
        assert_eq!(ts.tile_uv(EMPTY_TILE), None);
    }

    #[test]
    fn solid_flag_defaults_to_false() {
        let ts = tileset();
        assert!(ts.is_solid(1));
        assert!(!ts.is_solid(0));
        assert!(!ts.is_solid(99));
    }

    #[test]
    fn chunk_stores_tiles_by_local_coordinate() {
        let mut chunk = TileChunk::new(0, 0);
        chunk.set_tile(3, 4, 7);
        assert_eq!(chunk.tile(3, 4), 7);
        assert_eq!(chunk.tile(4, 3), EMPTY_TILE);
    }

    #[test]
    fn layer_maps_tile_coordinates_to_chunks() {
        let mut layer = TileMapLayer::default();
        layer.set_tile_at(0, 0, 1);
        layer.set_tile_at(CHUNK_SIZE as i32, 0, 2);
        assert_eq!(layer.tile_at(0, 0), 1);
        assert_eq!(layer.tile_at(CHUNK_SIZE as i32, 0), 2);
        assert_eq!(layer.tile_at(5, 5), EMPTY_TILE);
        assert_eq!(layer.chunks.len(), 2);
    }

    #[test]
    fn negative_coordinates_are_empty() {
        let mut layer = TileMapLayer::default();
        layer.set_tile_at(-1, -1, 3);
        assert_eq!(layer.tile_at(-1, -1), EMPTY_TILE);
        assert!(layer.chunks.is_empty());
    }

    #[test]
    fn layers_are_sorted_by_z() {
        let mut map = TileMap::default();
        map.layers.push(TileMapLayer {
            name: "top".to_string(),
            z: 10,
            ..Default::default()
        });
        map.layers.push(TileMapLayer {
            name: "bottom".to_string(),
            z: -5,
            ..Default::default()
        });
        let names: Vec<&str> = map
            .sorted_layers()
            .iter()
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(names, vec!["bottom", "top"]);
    }

    #[test]
    fn solid_query_finds_a_solid_tile() {
        let mut map = TileMap::default();
        let mut layer = TileMapLayer {
            tileset: "terrain".to_string(),
            ..Default::default()
        };
        layer.set_tile_at(4, 4, 1);
        map.layers.push(layer);

        let mut tilesets = HashMap::new();
        tilesets.insert("terrain".to_string(), tileset());

        let hit = map.first_solid_hit(&tilesets, [60.0, 60.0, 70.0, 70.0]);
        assert_eq!(hit, Some([64.0, 64.0]));
        assert!(map
            .first_solid_hit(&tilesets, [0.0, 0.0, 10.0, 10.0])
            .is_none());
    }

    #[test]
    fn map_size_covers_all_chunks() {
        let mut map = TileMap::default();
        let mut layer = TileMapLayer::default();
        layer.set_tile_at(CHUNK_SIZE as i32 - 1, CHUNK_SIZE as i32 - 1, 1);
        map.layers.push(layer);
        assert_eq!(map.size_in_tiles(), [16, 16]);
    }
}
