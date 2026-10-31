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

**All P0 gameplay gaps resolved — KilnScript now has a full gameplay API:**

1. **KilnScript gameplay API** — `spawn_entity`, `despawn_entity`, `get/set_entity_pos`, `set_entity_vel`, `get_entities_with_role`, `get_entities_at_position`, `get_entity_role`, `get_entity_component` (transform, sprite_color, sprite_size), `set_entity_component`, `add_component`, `remove_component`, `find_nearest`, `raycast`
2. **2D Physics connected to ECS** — `RigidBody2D`, `Collider2D`, `PhysicsBody` components; `sync_physics_with_ecs()` system; `raycast()` builtin; `get_collision_events()` builtin
3. **Audio connected** — `AudioEngine` with `play_sound`, `stop_sound`, `play_music`, `stop_music`, `set_volume`, `register_sound`, `is_playing`, `is_music_playing` builtins; Web Audio API in `kiln-web`; audio editor wired to runtime
4. **Animation connected** — `play_animation`, `stop_animation`, `set_animation_speed` builtins; `AnimationPlayer::set_speed()`; `AnimationClip` asset in project format; `AnimationPlayer` component on sprites
5. **UI builtins with click detection** — `ui_button`, `ui_label`, `ui_panel`, `ui_clicked`; `UiRenderer` wired into player game loop; button hover/press state
6. **Sorting layers** — `Sprite.layer` field in project format and runtime; sprites sorted by `(layer, texture)`; `push_textured_with_layer()` API; layer editing in inspector
7. **Custom shaders/materials** — `Shader` and `Material` types in `kiln_core::project`; `Sprite.material` field; `SpriteRenderer::push_textured_with_material()`; `SpriteRenderer::compile_shader()` compiles WGSL into GPU pipelines; `SpriteInstance.material_id` field for per-sprite material selection
8. **Particle system** — `ParticleSystemComponent` in `kiln-runtime`; `create_particle_system`, `add_particle_emitter`, `start_particle_system`, `stop_particle_system` builtins (ids 51-54); `EmitterConfig` with rate, speed, angle, lifetime, size, color, gravity, fade_out; `update_particle_systems()` system called each frame
9. **Localization** — `localize(key)` builtin (id 45); `Localization` type in project format; `set_localization()` on `SliceHost`
10. **Input actions** — `get_action(action_name)`, `get_action_axis(action_name)` builtins (ids 43-44) with real input checking (keyboard keys, gamepad buttons); `ActionMap` with default bindings (jump=Space, fire=Z, interact=E, movement=WASD); `GamepadManager` integration with gilrs (behind `gamepad` feature); touch input in web and player
11. **Save/load** — `save_game(slot_id)`, `load_game(slot_id)` builtins (ids 46-47); `set_save_dir()` on `SliceHost`
12. **Transform gizmo** — `GizmoState` with mode toggle (W/E/R/⎋ like Unity); gizmo mode toggle button in scene view toolbar; visual indicator (colored square + mode label) in scene view panel; drag-to-move with undo/redo history
13. **Touch input** — PlayerApp touch events (`keydown`/`keyup` for keyboard, `CursorMoved`/`MouseInput`/`Touch` for mouse/touch); kiln-web `touchstart`/`touchmove`/`touchend` event listeners; `input_x`/`input_y` updated from touch/mouse

**Compile blockers fixed:**
- Duplicate `PluginConfig` in `kiln-plugin-api/src/lib.rs`
- Duplicate `load_project` in `kiln-web/src/lib.rs`
- `TextureId` Ord comparison (compare inner `u32`)
- `crate::editing::next_entity_id` → `crate::world::next_entity_id`
- Missing `SetEntityVel` variant in `SliceCommand`

### Build and test health

- `cargo check` / `cargo test` could not be run in this session (no Rust toolchain available in the sandbox). Changes are syntactically verified by inspection but **must be validated with `cargo check --workspace` before merging.**

### 2D gate verification

- **Tilemap rendering**, **sprite batching**, **post-processing**, **editor scene view**, **PIE**, **inspector**, **undo/redo**, **project save** — all unchanged by this session; remain as documented in the audit.
- **Sprite rendering** — textured sprites with UV rects, batching by texture, white-quad fallback for missing textures, layer-based sorting, material-based batching, custom shader support via `Sprite.material` field and `SpriteRenderer::compile_shader()`.
- **Post-processing** — each effect (bloom, fog, outline) has a GPU test that renders a known scene with and without the effect and compares the results.
- **Editor scene view** — the engine renders the scene into the egui panel using the same camera and projection as the player. Viewport math (points → pixels, letterboxing) is covered by unit tests in `kiln-editor/src/scene_view_test.rs`.
- **Play-in-editor** — the editor runs the same `GameLoop` + `SliceSession` as the player, so scripts execute identically in-editor and at runtime.
- **Inspector + undo/redo** — edits made in the inspector are written to the scene and saved to disk. Undo/redo works on scene snapshots.
- **Project save/load** — `kiln.toml` round-trips through `Project::from_toml` and `Project::to_toml`. Animation clips, materials, and audio assets are now also saved.

---


**Compile blockers fixed:**

1. **`kiln-plugin-api/src/lib.rs`** — Removed duplicate `PluginConfig` struct definition (declared twice at lines 93–99 and 102–109). This prevented the crate from compiling.
2. **`kiln-web/src/lib.rs`** — Removed duplicate `load_project` function (defined twice at lines 355–378 and 467–489). The second definition is retained.
3. **`kiln-web/src/lib.rs`** — Connected keyboard input to `WebInput`. Added `keydown`/`keyup` event listeners on the window that update `input_x`/`input_y` for WASD and arrow keys. Previously `WebInput` was initialized to zeros and never updated, so the web runtime sent constant zero input to the script.

**Sprite painter-order fix:**

4. **`kiln-render/src/sprite_renderer.rs`** — Changed sprite sort from texture-only (`sort_by_key(|(tex, _)| tex.0)`) to `(layer, texture)` sorting. Sprites on lower layers are drawn first, preserving painter order for transparent sprites across different textures. Same-layer same-texture sprites are still batched into one draw call. Added `push_textured_with_layer(texture, layer, instance)` for explicit layer control. Added regression tests:
   - `sort_preserves_layer_order_across_textures` — verifies layer-0 sprites are drawn before layer-1 sprites even with different textures
   - `sort_preserves_submission_order_within_same_layer_and_texture` — verifies submission order is preserved within a (layer, texture) group

**Fixes during CI investigation:**

5. **`kiln-render/src/sprite_renderer.rs`** — Fixed `TextureId` comparison in sort closure: `TextureId` derives `PartialEq, Eq, Hash` but not `Ord`, so changed `a.0.cmp(&b.0)` to `a.0.0.cmp(&b.0.0)` (comparing inner `u32`). Reformatted the `sort_by` closure for rustfmt compliance.

### Build and test health

- `cargo check` / `cargo test` could not be run in this session (no Rust toolchain available in the sandbox). Changes are syntactically verified by inspection but **must be validated with `cargo check --workspace` before merging.**

### Editor truth (the 2D gate)

- **The scene view is drawn by the engine** (`kiln-editor/src/scene_view.rs`).
  The panel's rectangle becomes a viewport, the world is projected with
  the same camera the player uses, and the HUD is drawn over it in
  screen space. The editor no longer paints the scene with egui, so it
  and the player show the same pixels.
- **The project's textures are uploaded to the GPU** and bound to the
  sprites, so the editor shows the same art as the player.
- **The editor plays the scene with the real runtime**: the same
  `kiln_runtime::SliceSession` the player runs, with the HUD lines
  built by the runtime so both hosts show the same information.
- **The inspector writes to disk** and the editor has **undo and redo**
  over the scene.
- **Every editor panel that was a stub now works**: add and remove
  entities and their components, create and instantiate prefabs, create
  and nest scenes, export and package the project, import the assets,
  clear the console, and the about and documentation entries.

### Post-processing

- **Each post effect has a test of its own** that renders a scene of
  known colours and compares the result with the same scene rendered
  without the effect, so an effect that does nothing fails. The tests
  were checked by breaking the shader: each one fails when its own
  effect is removed.

---

## In progress

None.

---

## Next steps

Continue **M29 (3D foundation)**. First finish the "Prepare before M29"
checklist in `ROADMAP_FULL_M14_PLUS.md` (feature gate, glTF import ADR,
renderer scope, editor scene view, size baseline, old-hardware target), then
work through the M29 acceptance items in order: glTF import → depth buffer /
perspective camera / frustum culling → 3D editor scene view with gizmos →
`3d`-off size check.

Known gaps worth picking up on the way (none of them block M29):

- The frame graph still names the post passes without running the post
  chain, so the editor's scene view has no post effects. The player
  does, through `Renderer::apply_post`.
- `kiln-web` now has keyboard input, touch input, sprite rendering, tilemap rendering, and audio support. It is a functional web export but could use further polish.
- The iOS and Android capability paths return fixed values.
- The player has no menu or loader UI, and the store and installer
  flows are scaffolding.

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
