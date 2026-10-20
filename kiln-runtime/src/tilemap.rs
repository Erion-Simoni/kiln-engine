//! Runtime tilemap component.
//!
//! A [`Tilemap`] binds a [`kiln_core::TileMap`] to a GPU tileset texture and
//! turns it into drawable sprite instances. Tiles are stored in chunks
//! ([`kiln_core::CHUNK_SIZE`] tiles per side) and only the chunks that
//! intersect the camera viewport are submitted each frame, so a large map
//! does not allocate one entity per tile and does not pay for off-screen
//! tiles.

use kiln_core::tilemap::{CHUNK_SIZE, EMPTY_TILE};
use kiln_core::{TileMap as TileMapData, TileSet};
use kiln_render::{SpriteInstance, TextureId};
use std::collections::HashMap;

/// A drawable tilemap instance in the world.
#[derive(Debug, Clone)]
pub struct Tilemap {
    /// The tile data of this map.
    pub map: TileMapData,
    /// Tilesets by id, used to resolve each layer's tileset.
    pub tilesets: HashMap<String, TileSet>,
    /// GPU texture of each tileset, keyed by tileset id.
    pub textures: HashMap<String, TextureId>,
    /// Tint applied to every tile.
    pub color: [f32; 4],
    /// World-space offset of the whole map.
    pub offset: [f32; 2],
}

impl Tilemap {
    /// Create a runtime tilemap from data.
    pub fn new(
        map: TileMapData,
        tilesets: HashMap<String, TileSet>,
        textures: HashMap<String, TextureId>,
    ) -> Self {
        Self {
            map,
            tilesets,
            textures,
            color: [1.0; 4],
            offset: [0.0, 0.0],
        }
    }

    /// World-space rectangle of the map, as `[min_x, min_y, max_x, max_y]`.
    pub fn world_bounds(&self) -> [f32; 4] {
        let [w, h] = self.map.size_in_tiles();
        let ts = self.map.tile_size;
        [
            self.offset[0],
            self.offset[1],
            self.offset[0] + w as f32 * ts[0],
            self.offset[1] + h as f32 * ts[1],
        ]
    }

    /// Whether the given world-space rect overlaps a solid tile.
    ///
    /// Returns the world position of the first solid tile found.
    pub fn first_solid_hit(&self, rect: [f32; 4]) -> Option<[f32; 2]> {
        let [min_x, min_y, max_x, max_y] = rect;
        let shifted = [
            min_x - self.offset[0],
            min_y - self.offset[1],
            max_x - self.offset[0],
            max_y - self.offset[1],
        ];
        self.map.first_solid_hit(&self.tilesets, shifted)
    }

    /// Collect the visible tile instances for a camera viewport.
    ///
    /// `view` is the world-space rectangle of the camera
    /// (`[min_x, min_y, max_x, max_y]`). Chunks that do not intersect it are
    /// skipped entirely.
    ///
    /// Each entry is the tileset texture to draw with and the instance to
    /// draw, so the caller can push them straight into the sprite renderer.
    pub fn collect_visible(&self, view: [f32; 4]) -> Vec<(TextureId, SpriteInstance)> {
        let mut instances = Vec::new();
        let ts = self.map.tile_size;
        if ts[0] <= 0.0 || ts[1] <= 0.0 {
            return instances;
        }

        let [min_x, min_y, max_x, max_y] = view;
        let origin_x = min_x - self.offset[0];
        let origin_y = min_y - self.offset[1];
        let end_x = max_x - self.offset[0];
        let end_y = max_y - self.offset[1];

        let chunk_span = CHUNK_SIZE as f32 * ts[0];
        let chunk_span_y = CHUNK_SIZE as f32 * ts[1];

        for layer in self.map.sorted_layers() {
            let Some(tileset) = self.tilesets.get(&layer.tileset) else {
                continue;
            };
            let Some(&texture) = self.textures.get(&layer.tileset) else {
                continue;
            };

            let cx0 = (origin_x / chunk_span).floor() as i32;
            let cy0 = (origin_y / chunk_span_y).floor() as i32;
            let cx1 = (end_x / chunk_span).floor() as i32;
            let cy1 = (end_y / chunk_span_y).floor() as i32;

            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    let Some(chunk) = layer.chunk(cx, cy) else {
                        continue;
                    };
                    for ly in 0..CHUNK_SIZE {
                        for lx in 0..CHUNK_SIZE {
                            let tile = chunk.tile(lx, ly);
                            if tile == EMPTY_TILE {
                                continue;
                            }
                            let Some(uv_rect) = tileset.tile_uv(tile) else {
                                continue;
                            };
                            let tx = cx * CHUNK_SIZE as i32 + lx as i32;
                            let ty = cy * CHUNK_SIZE as i32 + ly as i32;
                            let px = self.offset[0] + layer.offset[0] + tx as f32 * ts[0];
                            let py = self.offset[1] + layer.offset[1] + ty as f32 * ts[1];
                            instances.push((
                                texture,
                                SpriteInstance {
                                    position: [px + ts[0] * 0.5, py + ts[1] * 0.5],
                                    size: ts,
                                    color: self.color,
                                    uv_rect,
                                },
                            ));
                        }
                    }
                }
            }
        }

        instances
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::{TileMapLayer, CHUNK_SIZE};

    fn tileset() -> TileSet {
        TileSet {
            texture: "tiles/terrain.png".to_string(),
            tile_size: [16, 16],
            columns: 2,
            rows: 2,
            solid: vec![false, true, false, false],
        }
    }

    fn tilemap() -> Tilemap {
        let mut map = TileMapData::default();
        let mut layer = TileMapLayer {
            name: "ground".to_string(),
            tileset: "terrain".to_string(),
            ..Default::default()
        };
        layer.set_tile_at(0, 0, 0);
        layer.set_tile_at(1, 0, 1);
        layer.set_tile_at(CHUNK_SIZE as i32, 0, 0);
        map.layers.push(layer);

        let mut tilesets = HashMap::new();
        tilesets.insert("terrain".to_string(), tileset());
        let mut textures = HashMap::new();
        textures.insert("terrain".to_string(), TextureId(7));

        Tilemap::new(map, tilesets, textures)
    }

    #[test]
    fn collects_visible_tiles() {
        let tm = tilemap();
        let instances = tm.collect_visible([0.0, 0.0, 64.0, 64.0]);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].0, TextureId(7));
        assert_eq!(instances[0].1.position, [8.0, 8.0]);
        assert_eq!(instances[0].1.uv_rect, [0.0, 0.0, 0.5, 0.5]);
    }

    #[test]
    fn culls_chunks_outside_the_view() {
        let tm = tilemap();
        // Only the second chunk is visible.
        let instances = tm.collect_visible([
            CHUNK_SIZE as f32 * 16.0,
            0.0,
            CHUNK_SIZE as f32 * 16.0 + 64.0,
            64.0,
        ]);
        assert_eq!(instances.len(), 1);
    }

    #[test]
    fn empty_view_collects_nothing() {
        let tm = tilemap();
        assert!(tm
            .collect_visible([10000.0, 10000.0, 11000.0, 11000.0])
            .is_empty());
    }

    #[test]
    fn solid_query_reports_tile_position() {
        let tm = tilemap();
        let hit = tm.first_solid_hit([16.0, 0.0, 20.0, 20.0]);
        assert_eq!(hit, Some([16.0, 0.0]));
        assert!(tm.first_solid_hit([0.0, 0.0, 4.0, 4.0]).is_none());
    }

    #[test]
    fn world_bounds_cover_the_map() {
        let tm = tilemap();
        let bounds = tm.world_bounds();
        // The map spans two chunks of 16 tiles each.
        assert_eq!(bounds[2], 2.0 * CHUNK_SIZE as f32 * 16.0);
    }
}
