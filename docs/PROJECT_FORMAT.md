# Kiln Project Format (`kiln.toml`)

The Kiln project manifest is a single TOML file (`kiln.toml`) at the project root. It declares your game's identity, window settings, scenes, entities, components, tilemaps, and tilesets. Human-readable, git-friendly, and authored by hand or through the editor.

> Parsed by `kiln_core::Project` (`kiln-core/src/project.rs`) with TOML/RON round-trip support.

## Top-level fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | String | — | Unique project ID |
| `name` | String | — | Human-readable project name |
| `version` | String | — | Version string |
| `publisher` | String | `""` | Publisher/author |
| `icon` | String | `""` | Icon asset path |
| `features` | `Vec<String>` | `[]` | Feature flags (e.g. `audio`, `physics`) |
| `window` | `WindowSettings` | defaults | Window configuration |
| `mobile` | `MobileSettings` | defaults | Mobile export settings |
| `scenes` | `HashMap<String, Scene>` | `{}` | Scenes keyed by id |
| `tilemaps` | `HashMap<String, TileMap>` | `{}` | Tilemaps keyed by id |
| `tilesets` | `HashMap<String, TileSet>` | `{}` | Tilesets keyed by id |

## `[window]` — WindowSettings

```toml
[window]
width = 1280
height = 720
fullscreen = false
borderless = false
vsync = true
high_dpi = true
resizable = true
monitor = 0
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | u32 | `1280` | Window width in pixels |
| `height` | u32 | `720` | Window height in pixels |
| `fullscreen` | bool | `false` | Fullscreen mode |
| `borderless` | bool | `false` | Borderless window |
| `vsync` | bool | `true` | Vertical sync |
| `high_dpi` | bool | `true` | High-DPI scaling |
| `resizable` | bool | `true` | User-resizable window |
| `monitor` | u32 | `0` | Monitor index |

## `[scenes.<id>]` — Scene

A scene contains entities, nested scenes, and an optional KilnScript.

```toml
[scenes.main]
id = "main"
name = "Main"
script = "scripts/main.ks"

[[scenes.main.entities]]
id = "player"
name = "Player"
components = [
  { type = "Transform", x = 120.0, y = 200.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/hero.png" },
  { type = "Role", role = "player" },
]

[[scenes.main.nested_scenes]]
scene_id = "overlay"
offset = [10.0, 20.0]
additive = false
```

### Scene fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | String | — | Unique scene id |
| `name` | String | — | Display name |
| `script` | `Option<String>` | `None` | Path to KilnScript file (`.ks`) |
| `entities` | `Vec<Entity>` | `[]` | Entities in this scene |
| `nested_scenes` | `Vec<NestedScene>` | `[]` | Nested scenes with offsets |

### Entity fields

Each entity in `[[scenes.<id>.entities]]` has:

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique entity id within the scene |
| `name` | String | Display name |
| `components` | `Vec<Component>` | List of component tables |

### Component types

Components are inline tables with `type` plus field-specific keys.

#### `Transform`

```toml
{ type = "Transform", x = 120.0, y = 200.0, rotation = 0.0, scale = 1.0 }
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x` | f32 | `0.0` | World X position |
| `y` | f32 | `0.0` | World Y position |
| `rotation` | f32 | `0.0` | Rotation in radians |
| `scale` | f32 | `1.0` | Uniform scale |

#### `Sprite`

```toml
{ type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/hero.png" }
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `color` | `[f32; 4]` | `[1,1,1,1]` | RGBA tint (0-1) |
| `texture` | `Option<String>` | `None` | Project-relative asset path (e.g. `"art/hero.png"`). Omitted = solid colour quad. |

Texture paths are project-relative with `/` separators (ADR-0028). They work identically on Windows, macOS, and Linux.

#### `Role`

```toml
{ type = "Role", role = "player" }
```

| Field | Type | Description |
|-------|------|-------------|
| `role` | String | Gameplay name (e.g. `"player"`, `"coin"`, `"ground"`). Scripts find entities by role instead of by index. |

#### `Tilemap`

```toml
{ type = "Tilemap", map = "overworld" }
```

| Field | Type | Description |
|-------|------|-------------|
| `map` | String | Id of the tilemap in `[[tilemaps]]` to bind to this entity |

## `[[tilemaps]]` — TileMap

```toml
[[tilemaps]]
id = "overworld"
name = "Overworld"

[[tilemaps.layers]]
name = "ground"
tileset = "terrain"
z = 0
offset = [0.0, 0.0]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | String | — | Unique map id |
| `name` | String | — | Display name |
| `layers` | `Vec<TileMapLayer>` | `[]` | Rendering layers |

### TileMapLayer fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | String | — | Layer name |
| `tileset` | String | — | Id of the tileset in `[[tilesets]]` |
| `z` | i32 | `0` | Render order (lower = behind) |
| `offset` | `[f32; 2]` | `[0, 0]` | World-space offset |

## `[[tilesets]]` — TileSet

A tileset is a texture atlas of tiles with per-tile solid flags.

```toml
[[tilesets]]
id = "terrain"
texture = "tiles/terrain.png"
tile_size = [16, 16]
columns = 4
rows = 4
solid = [false, true, false, false]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | String | — | Unique tileset id |
| `texture` | String | `""` | Project-relative path to the tileset image |
| `tile_size` | `[u32; 2]` | `[16, 16]` | Size of one tile in pixels |
| `columns` | u32 | `1` | Tile columns in the atlas |
| `rows` | u32 | `1` | Tile rows in the atlas |
| `solid` | `Vec<bool>` | `[]` | Per-tile solid flags, indexed by tile id |

## `[mobile]` — MobileSettings

```toml
[mobile]
orientation = "landscape"

[mobile.ios]
bundle_identifier = "com.example.game"
permissions = ["NSCameraUsageDescription"]
supports_notch = true

[mobile.ios.privacy_descriptions]
camera = "This game uses the camera"
microphone = "This game uses the microphone"

[mobile.android]
package_name = "com.example.game"
permissions = []
target_sdk = 33
```

## Example: Complete Minimal Project

```toml
id = "mygame"
name = "My Game"
version = "0.1.0"
publisher = "Me"
icon = ""
features = []

[window]
width = 640
height = 480
fullscreen = false
borderless = false
vsync = true
high_dpi = true
resizable = true
monitor = 0

[scenes.main]
id = "main"
name = "Main"
script = "scripts/main.ks"

[[scenes.main.entities]]
id = "player"
name = "Player"
components = [
  { type = "Transform", x = 120.0, y = 200.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/hero.png" },
  { type = "Role", role = "player" },
]

[[scenes.main.entities]]
id = "coin1"
name = "Coin"
components = [
  { type = "Transform", x = 240.0, y = 260.0, rotation = 0.0, scale = 1.0 },
  { type = "Sprite", color = [1.0, 1.0, 1.0, 1.0], texture = "art/coin.png" },
  { type = "Role", role = "coin" },
]

[[tilesets]]
id = "terrain"
texture = "tiles/terrain.png"
tile_size = [16, 16]
columns = 4
rows = 4
solid = [false, true, false, false]

[[tilemaps]]
id = "overworld"
name = "Overworld"

[[tilemaps.layers]]
name = "ground"
tileset = "terrain"
z = 0
offset = [0.0, 0.0]
```

## See Also

- `docs/API_REFERENCE.md` — Complete API reference for all crates
- `docs/KILNSCRIPT.md` — KilnScript language reference
- `docs/TILEMAPS.md` — Chunked tilemaps with collision
- `kiln-core/src/project.rs` — Source implementation
