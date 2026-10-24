# Tilemaps — Chunked 2D Tilemaps with Collision

Kiln's tilemap system stores tiles in fixed-size chunks (16×16 tiles per chunk) so a large map never allocates one entity per tile. Only chunks that intersect the camera viewport are submitted each frame, and solid-tile collision is queryable by gameplay code.

## Data Model (`kiln-core`)

Tilemaps live in the project manifest (`kiln.toml`) as plain serialisable data:

```toml
[[tilemaps]]
id = "overworld"
name = "Overworld"

[[tilemaps.layers]]
name = "ground"
tileset = "terrain"
z = 0
offset = [0.0, 0.0]

[[tilesets]]
id = "terrain"
texture = "tiles/terrain.png"
tile_size = [16, 16]
columns = 4
rows = 4
solid = [false, true, false, false]   # tile 1 is solid
```

- **`TileSet`** — a texture atlas of tiles, with per-tile `solid` flags.
- **`TileMapLayer`** — one layer references a tileset, has a z-order and an offset.
- **`TileMap`** — layers sorted by z; chunked tile storage.
- **`CHUNK_SIZE = 16`** — tiles per chunk side; `EMPTY_TILE = u16::MAX` means no tile.

Layers render in z-order (lowest first). The map's world bounds cover all chunks across all layers.

## Runtime (`kiln-runtime`)

### Spawning

A scene entity with a `Component::Tilemap { map }` component is turned into a runtime `Tilemap` bound to the project's tilesets and their GPU textures:

```rust
use kiln_runtime::{spawn_tilemaps, GameWorld};
use kiln_core::Project;

// `project` is your loaded Project, `scene` is the scene to spawn
let mut textures: HashMap<String, TextureId> = HashMap::new();
for tileset_id in project.tilesets.keys() {
    textures.insert(tileset_id.clone(), gpu_texture_for(tileset_id));
}

spawn_tilemaps(&mut world, &project, &scene, SceneOffset::default(),
    &mut |tileset_id| textures.get(tileset_id).copied());
```

The editor and the player use the same spawning path, so a tilemap looks identical in both.

### Visible Tile Collection (`collect_visible`)

Given a camera viewport rectangle `[min_x, min_y, max_x, max_y]`, `collect_visible` returns `(TextureId, SpriteInstance)` pairs for every visible tile. Chunks outside the view are skipped entirely — no per-tile iteration over the whole map.

### Solid-Tile Collision

`first_solid_hit(rect)` returns the world position of the first solid tile overlapping a rectangle. Use it for gameplay collision:

```rust
use kiln_runtime::tilemap_collision::{entity_box, first_solid_hit, move_with_collision};

let box_rect = entity_box(position, sprite_size);   // centred on entity
if let Some(hit) = first_solid_hit(&world, box_rect) {
    // hit = world position of the solid tile
}

// Axis-split movement: slide along walls instead of sticking
let new_pos = move_with_collision(&world, position, sprite_size, delta);
```

Movement is split into horizontal and vertical axes. Each axis is taken only when the destination is free, so an entity walking into a wall slides along it.

### Tilemap Component

```rust
pub struct Tilemap {
    pub map_id: String,
    pub map: TileMapData,
    pub tilesets: HashMap<String, TileSet>,
    pub textures: HashMap<String, TextureId>,
    pub color: [f32; 4],
    pub offset: [f32; 2],
}
```

`world_bounds()` returns the map's world-space rectangle. `first_solid_hit(rect)` queries solid tiles.

## Editor Authoring (`kiln-editor`)

The `TilemapEditorState` panel lets you:

- List the project's tilemaps and their layers
- Pick a tileset and paint tiles on a 16×16 chunk grid
- Toggle the `solid` flag of the brush tile (solid tiles show red in the grid)
- Scroll the painting grid by chunk

Paint writes into the selected layer of the selected map. Scrolling moves the visible grid by one chunk at a time.

## Example: Vertical Slice Tilemap

See `samples/vertical_slice/kiln.toml` for a complete project with a ground layer and a tileset.

## See Also

- `kiln-core/src/tilemap.rs` — data model
- `kiln-runtime/src/tilemap.rs` — runtime component and visibility culling
- `kiln-runtime/src/tilemap_collision.rs` — solid-tile collision and axis-split movement
- `kiln-runtime/src/tilemap_scene.rs` — spawning tilemaps from scene data
- `kiln-editor/src/tilemap_editor.rs` — editor painting panel