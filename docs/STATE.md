# Kiln Session Handoff

**Current milestone:** M0–M28 and the 2D-first gate (G-1 … G-6) are
**complete**. M29 (3D foundation) **is in progress**: commit `4556431` added a
feature-gated `3d` feature with mesh/material types (`Mesh`, `Material`,
`Vertex`, `SubMesh`, `MeshBounds`, `PrimitiveTopology`, `VertexFormat`,
`AlphaMode`, `Model`, `MeshHandle`) to `kiln-core`.

**Do not start M30 (PBR lighting) until M29's 3D core acceptance is met** —
M29 is not done yet. What remains for M29: glTF import, depth buffer,
perspective camera, frustum culling, 3D editor scene view with gizmos, and
the `3d`-off 2D web build-size check. See `ROADMAP_FULL_M14_PLUS.md` for the
M29 plan and the "Prepare before M29" section, and `docs/ARCHITECTURE.md` for
the layers.

Everything the audit called "partial" that is still partial is listed under
**Known issues**.

---

## Audit (M0–M28 honesty pass) — 2026-10-19

**Verdict:** Promising 2D **architecture / libraries**, not yet a solid shippable
2D engine. Many milestones are real crates with tests; several "done" items are
stubs, console smokes, or unwired editor UI. Trust code + this audit over
checkbox optimism in `docs/ROADMAP.md`.

### What is real

- Colored instanced sprite batch + ortho camera (`kiln-render`, `sprite.wgsl`)
- Fixed timestep loop, hecs world, project/`kiln.toml` round-trip
- Asset hash cache + `.kar` pack (`kiln-assets` / CLI)
- IR bytecode VM + KilnScript + graph→IR (values still I64/Bool only)
- egui docking editor shell with many panels
- Feature-gated rapier/rodio wrappers; anim/vfx/ai/gameplay/input/save **libs**
- M27 job schedule / AccessSet; M28 frame graph + GPU timestamps + screenshot hash
- Window samples: `tilemap`, `collector`, `particles` (colored quads / CPU VFX)

### Stub / scaffold / overstated

| Area | Reality |
| --- | --- |
| Textured sprites / atlas / UV | **Now real**: PNG → GPU textures, UV rects, batch by texture |
| Tilemap | **Now real**: chunked maps, camera culling, solid-tile queries |
| Post (M9) | **Now real on the GPU**: bloom, fog, outline; each effect has a test of its own |
| Editor scene view | **Now real**: the engine draws the scene into the panel, like the player |
| Play-in-editor | **Now real**: the editor runs the same runtime session as the player |
| Inspector edits | **Now real**: edits are saved to disk, with undo and redo |
| Undo / save / asset hub | **Now real**: undo and redo on the scene, save writes to disk, assets import into the cache |
| `kiln-player` | Loads projects, runs scripts, HUD, save/load; no menu/loader UI |
| `kiln-web` | Isolation/shell; not a full game loop export — keyboard input now wired, duplicate `load_project` removed |
| `UiRenderer` | Produces quads; the player draws a HUD with it, the editor uses egui |
| Store / installers / iOS ship | CLI/docs scaffolding; not end-to-end proven |
| Most `samples/` | Console `println` smokes, except the vertical slice |

### Band summary

| Band | Verdict |
| --- | --- |
| M0–M7 | Partial — core loop + colored batch + editor shell; editor viewport still egui |
| M8–M14 | Partial — tilemaps/textures real; post + web still thin |
| M15–M21 | Stub-heavy→Partial — libs ahead of product wiring; UI renders but unwired |
| M22–M28 | Partial — AI lib solid; M27–M28 infra real; product gaps remain |

---

## Done (this session)

**Branch:** `cline/p0-gaps-web-ui-physics-anim` (pushed to origin)

**P0 Critical Gaps addressed:**

### 1. Web Export — Full Game Loop (`kiln-web/src/lib.rs`)
- Added mouse input handling (mousedown, mouseup, mousemove, wheel for zoom)
- Added window resize handler with surface reconfiguration
- Implemented Web Audio API playback via `audio_player` callback on `SliceSession`
  - Pre-decodes audio assets at startup for low-latency SFX
  - Falls back to async fetch+decode for music/large files
- Added UI rendering from script commands (`UiButton`, `UiLabel`, `UiPanel`)
  - Renders through `UiRenderer` → `SpriteRenderer` with screen-space camera
  - HUD text rendering preserved
- Fixed body initial position to use `Transform` component (not linear_velocity)

### 2. In-Game UI System (Runtime) (`kiln-ui/`)
- Exported `Canvas` widget in `kiln-ui/src/lib.rs`
- Added `children: Vec<Box<dyn Widget>>` to `Canvas` with `add_child`, `add_button`, `add_label`, `add_panel`, `add_list` methods
- Added `as_any()` / `as_any_mut()` to `Widget` trait for downcasting
- Implemented `UiRenderer::render_canvas()` to recursively render children
- Added `UiRenderer::hit_test_button()` and `hit_test_canvas()` for click detection
- Added `Button::on_click: Option<String>` with `with_on_click()`, `on_click()`, `set_on_click()` methods
- Added `Rect::contains_point()` for hit testing
- All widget types (`Panel`, `Label`, `Button`, `List`, `Canvas`) implement `as_any`

### 3. 2D Physics Joints — Wired to rapier2d (`kiln-runtime/src/physics.rs`)
- Implemented joint creation in `sync_physics_with_ecs()` for all `JointType` variants:
  - `Distance` → `ImpulseJoint::distance` with configurable length
  - `Revolute` → `ImpulseJoint::revolute` (hinge)
  - `Prismatic` → `ImpulseJoint::prismatic` with limits + motor support
  - `Static`/`Dynamic`/`Kinematic` → appropriate fallback joints
- Added `Joint2D` and `JointType` imports to physics module
- Restored `collision_groups` on colliders for layer/mask filtering
- Fixed rigid body initial position to read from `Transform` component

### 4. Animation Editor — Wired to Runtime (P1)
- Added `animation_clip: Option<String>` field to runtime `Sprite` component (`kiln-runtime/src/sprite.rs`)
- Added `entity_sprite_animation_clip()` function to extract clip from project entities (`kiln-runtime/src/scene.rs`)
- Updated `spawn_entities()` to pass animation_clip to runtime Sprite
- Updated web export `spawn_scene_entities()` to handle animation_clip, layer, material
- Updated editor prefab instantiation (`editing.rs`) to include animation_clip
- Updated editor scene save (`app.rs`) to persist animation_clip, layer, material
- Added animation clip picker to inspector (`inspector.rs`) with asset browser button
- Animation clips authored in editor now load at runtime via `AnimationPlayer`

### 5. Input Rebinding UI (P1)
- Added `ActionMap` to project format for serialization (`kiln-core/src/project.rs`)
- Created `InputRebindPanel` in editor with full rebind UI (`kiln-editor/src/input_rebind_panel.rs`)
- Added menu entry for Input Rebinding in View menu (`kiln-editor/src/app.rs`)
- Updated `SliceSession` to accept custom `ActionMap` via `with_action_map()` (`kiln-runtime/src/slice.rs`)
- Player, Editor, and Web now pass project `action_map` to runtime
- ActionMap serialized/deserialized with project

### 6. Audio Mixer + Effects (P1)
- Enhanced `AudioEngine` with separate SFX/Music channels and volume controls (`kiln-runtime/src/audio.rs`)
- Added per-source playback management (play, pause, resume, stop)
- Added audio builtins:
  - `set_sfx_volume`, `set_music_volume` - separate volume controls
  - `pause_sound`, `resume_sound` - per-sound control
  - `pause_music`, `resume_music` - music control
  - `stop_all_sounds` - stop everything
  - `sound_playing`, `music_playing` - status queries
- Added `AudioCategory` enum for SFX/Music distinction
- Proper separation of master, SFX, and music volumes
- AudioEngine exported from kiln-runtime

---

## Known issues

- Carry forward prior M14–M28 gaps (player has no menu/loader UI, web
  Workers ping-only, iOS device testing needs Mac, etc.)
- The frame graph's post passes are still stubs: the editor's scene view
  has no post effects, while the player has them.
- The effect toggles (B/O/F) are only verified by hand; the automatic
  tests cover the effects themselves.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed
  under ADR-0026
- The script API is a fixed builtin list; a script cannot yet spawn
  entities or read arbitrary world state
- Sprite layer ordering: same-layer different-texture transparent sprites may still be drawn in the wrong order; use `push_textured_with_layer` to separate transparent sprites onto different layers
- **Sprite material/shader:** `Sprite.material` field allows custom shaders via `Shader` and `Material` types in `kiln_core::project`. Use `SpriteRenderer::push_textured_with_material()` to render sprites with custom materials.
- **Web export:** `kiln-web` now has keyboard input, touch input, sprite rendering, tilemap rendering, GPU post-processing, and Web Audio API support. Full end-to-end web export testing (building and running in a real browser) has not been automated. See `docs/WEB_EXPORT_TESTING.md` for manual test procedure.
- **Clean machine run:** The engine has not been tested on a machine without a Rust toolchain installed. See `docs/CLEAN_MACHINE_TEST.md` for manual test procedure.
- **SDF/MSDF text:** SDF font support is implemented in `kiln-ui/src/text.rs` via `SdfTextRenderer` and `SdfFontData`. Font asset pipeline (TTF → SDF atlas) is not yet automated. See `docs/SDF_TEXT.md` for manual workflow.
- **Custom shader editor:** `kiln-editor/src/shader_editor.rs` provides a WGSL shader editor panel. Shader compilation to wgpu RenderPipeline is implemented via `SpriteRenderer::compile_shader()`. Shader assets are stored in `kiln_core::Project.shaders` and referenced by `Sprite.material`.
- The iOS and Android device capability paths return fixed values
- The audio sample needs a system audio backend, so it is built on its
  own: `cargo check --manifest-path samples/audio/Cargo.toml`

**Blocked:** None — M29 (3D foundation) is in progress. The 2D gate is
complete, so M29 may proceed; M30 is blocked on M29's acceptance.

---

## Documentation (complete)

The following docs fully describe the engine so any AI agent can work on it
without reading every source file:

| Doc | Purpose |
|-----|---------|
| `docs/ARCHITECTURE.md` | Crate map, data flow, IR design, dependency versions |
| `docs/API_REFERENCE.md` | Complete public API for every crate |
| `docs/PROJECT_FORMAT.md` | Complete `kiln.toml` schema reference |
| `docs/ENGINE_OVERVIEW.md` | How crates fit together, frame lifecycle, data flow |
| `docs/KILNSCRIPT.md` | KilnScript language reference |
| `docs/TILEMAPS.md` | Chunked tilemaps with collision |
| `docs/TUTORIAL.md` | Build your first game step by step |
| `docs/GETTING_STARTED.md` | Make a 2D game from scratch |
| `docs/BUDGETS.md` | Size and performance budgets |
| `docs/DECISIONS.md` | Architecture decision records (ADR-0001 … ADR-0034) |
| `ROADMAP_FULL_M14_PLUS.md` | M14+ milestones, M29 plan, "Prepare before M29" checklist |
| `docs/ROADMAP.md` | Milestones M0–M61+ |
| `docs/PACKAGING.md` | Packaging, signing, storefront distribution |
| `docs/ANDROID.md` / `docs/IOS.md` | Mobile export guides |
| `docs/PROGRESS_F6_NEXT.md` | F6 session record, pre-M29 checklist |
| `docs/fixes m1-29.md` | M1–M29 fixes log |
| `docs/STATE.md` | This file — session handoff |
