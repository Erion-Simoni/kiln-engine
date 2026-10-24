# Kiln Engine API Reference

Complete public API reference for every workspace crate. All types are `pub` unless marked otherwise. This is the definitive reference for AI agents and contributors.

> **Note:** `cargo` is not available in this environment, so this reference was written from source. Verify signatures against the actual source files before relying on them for compilation.

---

## `kiln-core` — Math, time, shared types, ECS glue, feature-gated 3D

### Modules

| Module | Purpose |
|--------|---------|
| `access` | Declared component access for parallel ECS scheduling |
| `device_tier` | Device capability detection |
| `id` | Unique entity IDs |
| `math` | Re-exports from `glam` (`Mat4`, `Quat`, `Vec2`, `Vec3`, `Vec4`) |
| `mesh` | 3D mesh/material types, feature-gated behind `3d` |
| `paths` | Cross-platform paths |
| `prefab` | Prefab system |
| `project` | Project file format (`kiln.toml` schema) |
| `tilemap` | Tilemap data model |
| `time` | Frame clock |
| `transform` | 2D transform component |

### Key Types

#### `kiln_core::Clock`
Fixed-timestep frame clock.

```rust
pub struct Clock { /* private fields */ }
impl Clock {
    pub fn new() -> Self;
    pub fn tick(&mut self);
    pub fn delta(&self) -> std::time::Duration;
    pub fn delta_secs(&self) -> f32;
    pub fn elapsed(&self) -> std::time::Duration;
    pub fn elapsed_secs(&self) -> f32;
}
```

#### `kiln_core::Transform`
2D transform for entities.

```rust
pub struct Transform {
    pub position: glam::Vec2,
    pub rotation: f32,
    pub scale: glam::Vec2,
}
impl Transform {
    pub fn new(x: f32, y: f32) -> Self;
    pub fn with_rotation(mut self, rotation: f32) -> Self;
    pub fn with_scale(mut self, scale: glam::Vec2) -> Self;
}
impl Default for Transform { fn default() -> Self { Self::new(0.0, 0.0) } }
```

#### `kiln_core::AccessSet` / `AccessKind` / `ComponentAccess`
Declared component access for parallel ECS scheduling.

```rust
pub enum AccessKind { Read, Write }
pub struct ComponentAccess { pub type_id: std::any::TypeId, pub kind: AccessKind }
impl ComponentAccess {
    pub fn read<T: 'static>() -> Self;
    pub fn write<T: 'static>() -> Self;
}
pub struct AccessSet { /* reads/writes HashSet<TypeId> */ }
impl AccessSet {
    pub fn new() -> Self;
    pub fn read<T: 'static>(mut self) -> Self;
    pub fn write<T: 'static>(mut self) -> Self;
    pub fn conflicts_with(&self, other: &AccessSet) -> bool;
    pub fn is_read_only(&self) -> bool;
}
pub fn any_conflicts(sets: &[AccessSet]) -> bool;
```

#### `kiln_core::Project`
The project manifest. Serializes to TOML/RON.

```rust
pub struct Project {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub icon: String,
    pub features: Vec<String>,
    pub window: WindowSettings,
    pub mobile: MobileSettings,
    pub scenes: HashMap<String, Scene>,
    pub tilemaps: HashMap<String, TileMap>,
    pub tilesets: HashMap<String, TileSet>,
}
impl Project {
    pub fn default() -> Self;
    pub fn to_toml(&self) -> Result<String, toml::ser::Error>;
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error>;
}
```

#### `kiln_core::Component`
Entity components.

```rust
pub enum Component {
    Transform { x: f32, y: f32, rotation: f32, scale: f32 },
    Sprite { color: [f32; 4], texture: Option<String> },
    Role { role: String },
    Tilemap { map: String },
}
```

#### `kiln_core::Scene`, `Entity`, `NestedScene`, `SceneOffset`

```rust
pub struct Scene { pub id: String, pub name: String, pub entities: Vec<Entity>, pub nested_scenes: Vec<NestedScene>, pub script: Option<String> }
pub struct Entity { pub id: String, pub name: String, pub components: Vec<Component> }
pub struct NestedScene { pub scene_id: String, pub offset: SceneOffset, pub additive: bool }
pub struct SceneOffset { pub x: f32, pub y: f32 }
```

### Feature: `3d`
`kiln-core` has a `3d` Cargo feature (`3d = ["glam/serde"]`), off by default. When on, `mesh` module compiles real types: `Mesh`, `Material`, `Vertex`, `SubMesh`, `MeshBounds`, `PrimitiveTopology`, `VertexFormat`, `AlphaMode`, `Model`, `MeshHandle`. When off, transparent stubs exist so the public API surface is stable. A 2D build pulls in no 3D code.

---

## `kiln-render` — wgpu renderer, tiers, post, 2D batching → 3D, frame graph, GPU timestamps

### Modules

| Module | Purpose |
|--------|---------|
| `camera` | Orthographic camera (`Camera`) |
| `frame` | Default 2D frame graph (`Frame2dGraph`, `clear_view`, `execute_passes`) |
| `graph` | Render graph (`RenderGraph`, `PassDecl`, `ResourceDesc`) |
| `pass_config` | Data-driven per-tier pass enablement (`PassEnableConfig`) |
| `post` | Post-processing pipeline (`PostConfig`, `PostFramebuffer`, `PostPipeline`) |
| `post_gpu` | GPU implementation (`PostGpuPipeline`, `PostUniforms`) |
| `profiler` | GPU timestamp queries (`GpuProfiler`, `PassTiming`) |
| `renderer` | Main renderer (`Renderer`) |
| `sprite_renderer` | Sprite batch renderer (`SpriteRenderer`, `SpriteInstance`) |
| `texture` | GPU textures (`TextureId`, `create_rgba8_texture`) |
| `tier` | Render tiers (`RenderTier`) |

#### `kiln_render::Camera`
Orthographic camera for 2D.

```rust
pub struct Camera {
    pub position: [f32; 2],
    pub zoom: f32,
    pub viewport_size: [f32; 2],
    pub top_left_origin: bool,
}
impl Camera {
    pub fn new(width: f32, height: f32) -> Self;
    pub fn new_top_left(width: f32, height: f32) -> Self;
    pub fn projection_matrix(&self) -> glam::Mat4;
    pub fn view_bounds(&self) -> [f32; 4];
    pub fn pan(&mut self, dx: f32, dy: f32);
    pub fn zoom(&mut self, factor: f32);  // clamps to [0.1, 10.0]
}
```

#### `kiln_render::SpriteRenderer`
Batching 2D sprite renderer. Sprites sharing a `TextureId` are drawn in one instanced call.

```rust
pub const MAX_SPRITES: usize = 16_384;

pub struct SpriteInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub uv_rect: [f32; 4],
}
impl SpriteInstance {
    pub fn colored(position: [f32; 2], size: [f32; 2], color: [f32; 4]) -> Self;
    pub const STRIDE: u64;
}
impl SpriteRenderer {
    pub fn new(device: &Device, queue: &Queue, format: wgpu::TextureFormat) -> Self;
    pub fn begin_frame(&mut self);
    pub fn push(&mut self, inst: SpriteInstance);
    pub fn push_textured(&mut self, texture: TextureId, inst: SpriteInstance);
    pub fn create_texture_rgba(&mut self, device, queue, label, width, height, rgba) -> TextureId;
    pub fn flush(&mut self, pass: &mut RenderPass, camera: &Camera, queue: &Queue);
    pub fn queued_count(&self) -> usize;
    pub fn draw_call_count(&self) -> u32;
    pub fn upload_quad(&mut self, queue: &Queue);
}
```

#### `kiln_render::Renderer`
Main renderer with tier support, post-processing, GPU profiling.

```rust
impl Renderer {
    pub async fn new(instance: &Instance, surface: &Surface, width: u32, height: u32) -> Self;
    pub fn device(&self) -> &Device;
    pub fn queue(&self) -> &Queue;
    pub fn tier(&self) -> RenderTier;
    pub fn set_tier(&mut self, tier: RenderTier);
    pub fn has_post(&self) -> bool;
    pub fn scene_target(&self) -> Option<&wgpu::TextureView>;
    pub fn apply_post(&mut self, encoder: &mut CommandEncoder, output_view: &TextureView);
    pub fn resize(&mut self, surface: &Surface, width: u32, height: u32);
    pub fn profiler(&self) -> &GpuProfiler;
    pub fn pass_config(&self) -> &PassEnableConfig;
    pub fn set_pass_config(&mut self, config: PassEnableConfig);
}
```

#### `kiln_render::RenderTier`
```rust
pub enum RenderTier { Low, Medium, High }
impl RenderTier {
    pub fn auto_select(adapter: &Adapter) -> Self;
    pub fn max_bloom_passes(&self) -> u32;   // Low: 0, Medium: 3, High: 5
    pub fn enable_lut(&self) -> bool;        // High only
    pub fn enable_fog(&self) -> bool;        // Medium + High
    pub fn enable_outline(&self) -> bool;    // High only
}
```

#### `kiln_render::PostConfig`
Post-processing configuration (bloom, fog, outline, LUT).

```rust
pub struct PostConfig {
    pub bloom_threshold: f32,   // default 0.8
    pub bloom_intensity: f32,   // default 0.5
    pub fog_density: f32,       // default 0.0
    pub fog_color: [f32; 3],    // default [0.5, 0.5, 0.5]
    pub outline_color: [f32; 4],
    pub outline_thickness: f32, // default 1.0
    pub outline_threshold: f32, // default 0.1
}
impl PostConfig {
    pub fn for_tier(tier: RenderTier) -> Self;
}
```

#### `kiln_render::TextureId`
```rust
pub struct TextureId(pub u32);
impl TextureId { pub const WHITE: TextureId = TextureId(0); }
```

#### `kiln_render::GpuProfiler`
GPU timestamp queries for per-pass cost.

```rust
pub struct GpuProfiler { /* private */ }
impl GpuProfiler {
    pub fn new(device: &Device, queue: &Queue, max_passes: u32) -> Self;
    pub fn supported(&self) -> bool;
    pub fn timings(&self) -> &[PassTiming];
    pub fn begin_frame(&mut self);
    pub fn resolve(&mut self, encoder: &mut CommandEncoder);
    pub fn poll_timings(&mut self, device: &Device);
}
pub struct PassTiming { pub name: String, pub duration_ns: u64 }
```

---

## `kiln-runtime` — Frame loop, scene, input, schedule/jobs, optional audio/physics/threads

### Modules

| Module | Purpose |
|--------|---------|
| `input` | Input handling (`InputState`, `Key`, `InputEvent`) |
| `jobs` | Background task pool (`TaskPool`, `JobHandle`, `load_file_async`) |
| `loop_` | Fixed-timestep game loop (`GameLoop`) |
| `scene` | Spawning project scenes (`spawn_scene`, `SpriteFactory`, `SpawnedEntity`) |
| `schedule` | ECS schedule with parallel stages (`Schedule`, `SystemEntry`, `build_stages`) |
| `settings` | Graphics settings (`GraphicsSettings`, `Resolution`) |
| `slice` | Vertical slice runtime (`SliceSession`, `SliceState`, `BUILTINS`) |
| `sprite` | Sprite component (`Sprite`) |
| `tilemap` | Runtime tilemap (`Tilemap`) |
| `tilemap_collision` | Solid-tile collision (`move_with_collision`, `first_solid_hit`) |
| `tilemap_scene` | Spawning tilemaps from scene data (`spawn_tilemaps`) |
| `world` | ECS world wrapper (`GameWorld`, `Role`, `SourceId`) |
| `audio` | (feature `audio`) Audio playback |
| `physics` | (feature `physics`) Physics wrapper |

#### `kiln_runtime::GameLoop`
Fixed timestep (1/60s) game loop.

```rust
pub struct GameLoop { /* private */ }
impl GameLoop {
    pub fn new() -> Self;
    pub fn tick<F>(&mut self, update: F) where F: FnMut(std::time::Duration);
    pub fn delta(&self) -> std::time::Duration;
    pub fn elapsed(&self) -> std::time::Duration;
}
```

#### `kiln_runtime::GameWorld`
ECS world wrapper over `hecs::World`.

```rust
pub struct GameWorld { world: World }
impl GameWorld {
    pub fn new() -> Self;
    pub fn world_mut(&mut self) -> &mut World;
    pub fn world_ref(&self) -> &World;
    pub fn spawn<T>(&mut self, components: T) -> EntityId where T: hecs::Bundle;
}
```

#### `kiln_runtime::slice` — SliceSession
Script-driven runtime. Runs a KilnScript program once per frame. Reads *facts*, emits *commands*.

```rust
pub const BUILTINS: &[(&str, u32, u32)] = &[
    ("print", 0, 1),
    ("get_input_x", 1, 0),
    ("get_input_y", 2, 0),
    ("get_elapsed", 3, 0),
    ("get_score", 4, 0),
    ("add_score", 5, 1),
    ("get_coin_count", 6, 0),
    ("collect_coin", 7, 0),
    ("move_player", 8, 2),
    ("set_status", 9, 1),
    ("get_status", 10, 0),
    ("get_delta_time", 11, 0),
];
pub const PICKUP_RADIUS: f32 = 24.0;
pub const ROLE_PLAYER: &str = "player";
pub const ROLE_COIN: &str = "coin";

pub struct SliceSession { /* private */ }
impl SliceSession {
    pub fn new(source: &str) -> Result<Self, ...>;
    pub fn step(&mut self, world: &mut GameWorld, input: (f64, f64), dt: f64) -> Result<(), ...>;
    pub fn state(&self) -> &SliceState;
    pub fn set_state(&mut self, state: SliceState);
    pub fn sync_world(&mut self, world: &mut GameWorld);
    pub fn hud_lines(state: &SliceState) -> Vec<String>;
}

pub struct SliceState {
    pub score: i64,
    pub elapsed: f64,
    pub player_x: f32,
    pub player_y: f32,
    pub collected: Vec<String>,
    pub status: String,
}
```

#### `kiln_runtime::tilemap_collision`
Axis-split movement: horizontal then vertical, slides along walls.

```rust
pub fn entity_box(position: [f32; 2], size: [f32; 2]) -> [f32; 4];
pub fn first_solid_hit(world: &GameWorld, rect: [f32; 4]) -> Option<[f32; 2]>;
pub fn move_with_collision(world: &GameWorld, position: [f32; 2], size: [f32; 2], delta: [f32; 2]) -> [f32; 2];
```

#### `kiln_runtime::Sprite`
CPU-side sprite drawn via the sprite batcher.

```rust
pub struct Sprite {
    pub color: [f32; 4],
    pub size: [f32; 2],
    pub uv_rect: [f32; 4],
    pub texture: TextureId,
}
impl Sprite {
    pub fn new(r, g, b, a) -> Self;
    pub fn with_size(mut self, w, h) -> Self;
    pub fn with_texture(mut self, t: TextureId) -> Self;
    pub fn with_uv(mut self, uv: [f32; 4]) -> Self;
}
```

---

## `kiln-script` — KilnScript language frontend → IR

### `kiln_script::KilnScriptFrontend`
```rust
pub struct KilnScriptFrontend { builtins: Vec<(String, u32, u32)> }
impl KilnScriptFrontend {
    pub fn new() -> Self;
    pub fn with_builtin(mut self, name: impl Into<String>, host_id: u32, argc: u32) -> Self;
}
pub trait LanguageFrontend {
    fn compile(&mut self, source: &str) -> Result<Program, Vec<Diagnostic>>;
}
impl LanguageFrontend for KilnScriptFrontend { ... }
```

### Language surface
- **Literals:** Integer (`42`), Float (`3.14`), String (`"hi"`)
- **Keywords:** `if`, `else`, `return`, `let`, `call`
- **Operators:** `+ - * / == != < <= > >= =`
- **Statements:** `let name = expr;`, `name = expr;`, `expr;`, `return expr;`, `if cond { } else { }`
- Blocks are scoped; variables are block-scoped; compile errors produce diagnostics with source spans

---

## `kiln-ir` — IR bytecode VM

### `kiln_ir::Value`
```rust
pub enum Value { I64(i64), F64(f64), Bool(bool), Str(String) }
impl Value {
    pub fn as_i64(&self) -> Option<i64>;
    pub fn as_f64(&self) -> Option<f64>;
    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_str(&self) -> Option<&str>;
    pub fn is_number(self) -> bool;
    pub fn to_f64(&self) -> Option<f64>;
    pub fn values_equal(a: Value, b: Value) -> bool;
}
```

### `kiln_ir::Program`, `Instruction`, `Label`
```rust
pub struct Program { /* instructions + labels + local_slots */ }
impl Program {
    pub fn new() -> Self;
    pub fn add_instruction(&mut self, instr: Instruction);
    pub fn define_label(&mut self, name: &str) -> Result<Label, ProgramError>;
    pub fn validate_jump_targets(&self) -> Result<(), ProgramError>;
    pub fn set_local_slots(&mut self, slots: u16);
    pub fn local_slots(&self) -> u16;
}

pub enum Instruction {
    PushConst(Value), Pop, Dup, Swap,
    GetLocal(u16), SetLocal(u16),
    Add, Sub, Mul, Div, Neg,
    Eq, Ne, Lt, Le, Gt, Ge,
    Jump(Label), JumpIf(Label), JumpIfNot(Label),
    Call(Label), Return,
    HostCall { id: u32, argc: u32 },
}
```

### `kiln_ir::Vm`, `HostContext`
```rust
pub struct Vm<H: HostContext> { /* stack, locals, pc, program, host */ }
impl<H: HostContext> Vm<H> {
    pub fn new(host: H) -> Self;
    pub fn execute(&mut self, program: &Program) -> Result<(), VmError>;
    pub fn set_instruction_budget(&mut self, budget: u64);
    pub fn stack(&self) -> &[Value];
    pub fn host(&mut self) -> &mut H;
}

pub trait HostContext {
    fn call_host(&mut self, func_id: u32, args: &[Value]) -> Result<Value, VmError>;
}
```

---

## `kiln-editor` — Native desktop editor app (egui shell, scene view, PIE, undo)

### Key modules: `app`, `assets`, `editing`, `history`, `scene_view`, `tilemap_editor`

#### `kiln_editor::SceneHistory`
Undo/redo stack of scene snapshots.

```rust
pub struct SceneHistory { /* undo/redo stacks */ }
impl SceneHistory {
    pub fn new() -> Self;
    pub fn record(&mut self, scene: Scene);
    pub fn undo(&mut self, current: Scene) -> Option<Scene>;
    pub fn redo(&mut self, current: Scene) -> Option<Scene>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn undo_depth(&self) -> usize;
}
```

#### `kiln_editor::editing`
```rust
pub struct EntityDefaults { pub size: [f32; 2], pub color: [f32; 4] }
pub fn add_entity(world, existing_ids, position, defaults) -> (String, hecs::Entity);
pub fn remove_entity(world, entity) -> bool;
pub fn set_role(world, entity, role) -> bool;
pub fn next_entity_id(existing: &[String]) -> String;
```

#### `kiln_editor::scene_view`
Engine-drawn scene viewport with letterboxing.

```rust
pub struct ViewportRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
pub fn letterbox(panel_w, panel_h, world_w, world_h) -> ViewportRect;
pub fn world_camera(world_w, world_h) -> Camera;
pub fn overlay_camera(viewport) -> Camera;
```

#### `kiln_editor::tilemap_editor`
```rust
pub struct TilemapEditorState { pub selected_map: Option<String>, pub selected_layer: usize, pub brush_tile: u16, pub scroll: [i32; 2] }
impl TilemapEditorState {
    pub fn new() -> Self;
    pub fn paint(&mut self, project, tx, ty, tile);
    pub fn paint_brush(&mut self, project, tx, ty);
    pub fn toggle_brush_solid(&mut self, project, tileset_id);
    pub fn brush_is_solid(&self, project, tileset_id) -> bool;
}
```

---

## `kiln-player` — Thin standalone game launcher

Loads a game from `.kar` archive or project directory and runs it.

```rust
// CLI: cargo run -p kiln-player -- --archive <path> [--scene <id>]
pub struct LoadedGame { pub project: Project, pub assets: HashMap<String, Vec<u8>>, pub scene_id: String }
pub fn load(archive: &Path, scene: Option<&str>) -> Result<LoadedGame, LoaderError>;
```

Texture loading: `TextureLibrary` decodes PNGs and uploads to GPU, keyed by project-relative path.

---

## Remaining crates (brief)

| Crate | Key public types |
|-------|-----------------|
| `kiln-assets` | `Image`, `decode_png`, `encode_png_rgba`, `Archive`, `AssetCache` |
| `kiln-graph` | `Graph`, `Node`, `NodeType`, `Pin`, `compile_graph` |
| `kiln-ui` | `Button`, `Label`, `List`, `Panel`, `Widget`, `UiRenderer`, `TextRenderer`, `NineSlice` |
| `kiln-input` | `ActionMap`, `Action`, `Binding`, `BindingType` |
| `kiln-save` | `SaveSlotManager`, `SaveSlot`, `SaveVersion`, `VersionMigration` |
| `kiln-anim` | `Clip`, `Frame`, `Tween`, `Ease` |
| `kiln-vfx` | `ParticleSystem`, `Emitter`, `EmitterConfig`, `Particle` |
| `kiln-gameplay` | `DataTable`, `Dialogue`, `Quest`, `QuestManager` |
| `kiln-ai` | `NavMesh` |
| `kiln-platform` | `PlatformServices`, `StubPlatformServices`, `SteamPlatformServices` |
| `kiln-updater` | `Updater` |
| `kiln-web` | WASM shell (stub — no game loop yet) |
| `kiln-cli` | `init`, `import`, `pack`, `export`, `publish` commands |


