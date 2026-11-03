# Kiln Editor Fix Plan

**Goal:** Get the editor compiling and opening so it can be used.
**Approach:** Fix errors session by session, never adding new errors. Each session ends with a working (or closer-to-working) build.
**Architecture inspiration:** Godot's `Editor` + `EditorInterface` pattern — `EditorApp` owns all state, `TabViewer` holds references to what it needs for rendering, and UI logic delegates cleanly.

---

## Current State

- **Syntax errors:** ALL FIXED (app.rs, shader_editor.rs parse cleanly)
- **Remaining errors:** ~141 type/borrow/visibility errors across ~22 files
- **Build status:** `cargo check -p kiln-editor` fails on type errors only — no more `unexpected closing delimiter` or `self parameter` errors

## Error Breakdown

| Category | Count | Example |
|---|---|---|
| `no field X on TabViewer` | ~45 | `scene_tabs`, `active_scene_tab`, `gizmo`, `mouse_x`, `mouse_y`, `entity_id_to_hecs`, etc. |
| `no method X on TabViewer` | ~10 | `load_project`, `create_project`, etc. |
| `mismatched types` | ~31 | `f32 * f64`, wrong argument types |
| Borrow checker | ~10 | mutable vs immutable borrows |
| Missing types/values | ~30 | `SpriteEditorState`, `world`, `rect`, `response`, `serde_json` |
| Other | ~15 | winit imports, `Component::Parent`, `pivot` field, `get_or_insert_mut`, etc. |

*Note: Private field errors (`emitters`, `rigid_body_set`, `collider_set`, `plugins`) and Rapier API errors (`Static` → `Fixed`) have already been fixed with `pub(crate)` and API updates.*

## Root Architectural Issue

`TabViewer<'a>` holds `&'a mut` references to some `EditorApp` fields, but the UI code in `impl egui_dock::TabViewer for TabViewer<'a>` (line 2675) and `impl<'a> TabViewer<'a>` (line 4263) accesses **EditorApp fields that are NOT in TabViewer** — e.g. `self.gizmo`, `self.scene_tabs`, `self.mouse_x`, `self.asset_filter`, `self.audio_editor`, etc.

This is the Godot `EditorInterface` problem: the dock UI needs access to editor state but doesn't hold references to all of it.

## Fix Design (Godot-inspired)

`EditorApp` is the `Editor` (owns everything). `TabViewer` is like `EditorInterface` — it should hold references to every field the dock UI needs to render. The `ui()` method delegates tab rendering to helper methods.

**Step 1:** Add all missing fields as `&'a mut` references to `TabViewer` struct.
**Step 2:** Update `EditorApp` to construct `TabViewer` with all fields.

---

## Session 1 Plan: TabViewer Fields + Type Mismatches (~86 errors)

**Estimated time:** 1 session (2-3 hours)

### Fix 1a: Add missing fields to `TabViewer` struct (~45 errors)

Add these `&'a mut` fields to `struct TabViewer<'a>` (app.rs line 2392):

```rust
gizmo: &'a mut GizmoState,
scene_tabs: &'a mut Vec<SceneTab>,
active_scene_tab: &'a mut Option<usize>,
mouse_x: &'a mut f32,
mouse_y: &'a mut f32,
entity_id_to_hecs: &'a mut HashMap<String, hecs::Entity>,
grid_snap_enabled: &'a mut bool,
audio_editor: &'a mut AudioEditorState,
asset_filter: &'a mut String,
project_hub: &'a mut ProjectHubState,
profiler_panel: &'a mut ProfilerPanel,
vfx_editor: &'a mut VfxEditorState,
version_control: &'a mut VersionControlPanel,
ui_layout_editor: &'a mut UiLayoutEditorState,
sprite_editor: &'a mut SpriteEditorState,
shader_editor: &'a mut ShaderEditorState,
project: &'a mut Option<Project>,
preferences: &'a mut PreferencesPanel,
plugin_manager: &'a mut PluginManagerPanel,
physics_editor: &'a mut PhysicsEditorState,
input_rebind: &'a mut InputRebindPanel,
grid_snap_size: &'a mut f32,
gpu: &'a mut Option<GpuState>,
debugger_panel: &'a mut DebuggerPanel,
build_panel: &'a mut BuildPanel,
anim_editor: &'a mut AnimationEditorState,
```

And add these methods to `impl<'a> TabViewer<'a>`:
- `load_project()` — delegates to EditorApp's project loading
- `create_project()` — delegates to EditorApp's project creation

### Fix 1b: Update `EditorApp` construction of `TabViewer` (~10 call sites)

Find where `TabViewer { ... }` is constructed in `EditorApp` and pass all new fields as references.

### Fix 1c: Fix type mismatches (~31 errors)

- `f32 * f64` → cast `f64` to `f32` (3 errors in profiler_panel.rs)
- Wrong argument types (4 errors) — check each call site
- `type GameWorld cannot be dereferenced` (2 errors) — remove `*`
- `Component::Sprite` missing `pivot` field (1 error) — add `pivot` to Sprite struct or remove usage
- `unresolved imports Key, NamedKey` (1 error) — winit 0.30 API change

### Fix 1d: Fix borrow checker issues (~10 errors)

- `cannot borrow *action_map as mutable because it is also borrowed as immutable` (3 errors)
- `cannot borrow project.scenes as mutable because it is also borrowed as immutable` (2 errors)
- `cannot borrow *state / *self.state` (4 errors)
- Pattern: clone the immutable value before mutable borrow, or restructure to hold mutable only

### Success criteria for Session 1

`cargo check -p kiln-editor` should go from ~141 errors to ~55 errors (only missing types/values + a few borrow issues remaining).

---

## Session 2 Plan: Missing Types/Values + Remaining Borrow Issues (~55 errors)

**Estimated time:** 1 session (1-2 hours)

### Fix 2a: Missing types and values (~30 errors)

- `cannot find type SpriteEditorState` — check import or add type alias
- `cannot find type InputRebindPanel` — check import
- `cannot find value world` — check scope
- `cannot find value rect / response / size` — check variable naming
- `cannot find module serde_json` — add import
- `no variant Parent in Component` — add variant or fix usage
- `no method get_or_insert_mut for Option` — use `get_or_insert_with` + `insert` pattern
- `no method shutdown_plugins / handle_event on PluginRegistry` — add methods
- `no method get_mut for &mut World` — check hecs API
- `function takes 3 arguments but 2 supplied` — fix call site

### Fix 2b: Remaining borrow issues (~10 errors)

Same pattern as Session 1 — restructure borrows.

### Fix 2c: Build and run the editor

`cargo build -p kiln-editor` should succeed. `cargo run -p kiln-editor` should open the editor window.

### Success criteria for Session 2

Editor compiles and opens. User can click around the UI.

---

## Future Plan: 2D Parity with Unity 2D / Godot 2D / GameMaker

Once the editor compiles, the roadmap for making Kiln 2D competitive:

### Phase A: Core Editor Features (M30-M34)

| Feature | Godot Equivalent | Unity Equivalent | Status |
|---|---|---|---|
| Scene tree / hierarchy | Scene dock | Hierarchy window | Partial (exists but buggy) |
| Inspector panel | Inspector dock | Inspector window | Partial |
| Asset browser | FileSystem dock | Project window | Partial |
| Tilemap editor | TileMap editor | Tilemap package | Exists (samples/tilemap) |
| Animation editor | AnimationPlayer | Animator | Partial (anim_editor.rs) |
| Sprite editor | Sprite editor | Sprite Renderer | Partial |
| Prefab system | PackedScene | Prefab | Partial (prefab_editor.rs) |
| Physics 2D | RigidBody2D + Collider2D | Rigidbody2D + Collider2D | Partial (rapier2d) |
| Audio bus | AudioBus | AudioMixer | Partial (audio_editor.rs) |
| Shader editor | ShaderEditor | Shader Graph | Partial (shader_editor.rs) |
| Debugger | Remote scene dock | Hierarchy during play | Partial |

### Phase B: Polish & UX (M35-M40)

- Undo/redo history (SceneHistory exists, needs wiring)
- Drag-and-drop between panels
- Context menus on scene nodes
- Snap-to-grid for entity placement
- Multi-selection in hierarchy
- Copy/paste entities

### Phase C: Export & Publishing (M41-M48)

- Desktop export (Windows/Mac/Linux) — already works via CLI
- Web export (WASM) — partially works (CI job ready)
- Mobile export (Android/iOS) — planned
- One-click export dialog in editor

### Phase D: Advanced 2D Features (M49-M60)

- 2D lights and shadows
- Particle system editor (VfxEditor exists, needs UI)
- Tilemap collision editor
- 2D navigation (NavMesh2D equivalent)
- Sprite animation editor improvements
- State machine visual editor (Behavior tree nodes exist in graph)

### Architecture Principles (from Godot)

1. **Everything is a node** — Entity-Component-System already in place (hecs)
2. **Signals/delegates** — KilnScript event system, needs UI wiring
3. **Scene instancing** — Prefab system exists, needs polish
4. **Editor is a game** — Editor runs in same engine, uses same UI system (egui)
5. **Tool scripts** — KilnScript can run in editor context (partial)

### No-New-Errors Rule

- Every fix must compile before the next fix
- Use `cargo check -p kiln-editor` after each change
- No `todo!()` or `unimplemented!()` in new code
- No `unsafe` blocks without justification
- All public items documented

---

## Session Tracking

| Session | Goal | Errors Fixed | Status |
|---|---|---|---|
| 1 | TabViewer fields + type mismatches + borrow fixes | ~152 | In Progress (82 errors remaining) |
| 2 | Missing types/values + final borrow issues + build & run | ~55 | Pending |
| Future | 2D feature parity roadmap | N/A | Planned |

**Next action:** Continue Session 1 (82 errors remaining), then Session 2 (~30 errors)
**Step 3:** Fix type mismatches, borrow issues, missing types/values.