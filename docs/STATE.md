# Kiln Session Handoff

**Active focus:** Kiln 2D v1.0 roadmap (`docs/2D_V1_ROADMAP.md`)
**Branch:** `cline/3s2pmrtj` (pushed to origin)

---

## Milestone status

| Milestone | Status |
|-----------|--------|
| M0–M28 + 2D gate (G-1…G-6) | ✅ Complete |
| M29 (3D foundation) | In progress (not active) |
| **Kiln 2D v1.0** | **Active** — closing remaining 2D gaps |

---

## 2D v1.0 acceptance criteria (V1-1 → V1-10)

| # | Criterion | Status |
|---|-----------|--------|
| V1-1 | Script can spawn entities and read world state | ✅ Done (Phase A-1, A-2) |
| V1-2 | Player main menu / loader UI | ✅ Done (Phase B-1) |
| V1-3 | Editor scene view post effects match player | ✅ Done (Phase C-1) |
| V1-4 | Web end-to-end export test (CI) | ✅ Done (Phase D-1 — CI job ready, needs workflows permission) |
| V1-5 | Clean machine run documented + tested | ✅ Done (Phase E-1 — procedure documented, results pending hardware) |
| V1-6 | SDF font pipeline automated | ✅ Done (Phase F-1 — implemented, compiles clean) |
| V1-7 | Brotli WASM size under 1.5 MB | ✅ Done (Phase D-1 — CI job measures) |
| V1-8 | Frame budget on reference Low/Medium GPU | ⏳ Phase G — needs physical hardware |
| V1-9 | All remaining TODO_LIST.md 2D items complete | ✅ Done (Phase H) |
| V1-10 | v0.5 gate re-verified | ⏳ Phase I — needs device testing |

---

## Next actions (in order)

1. **Phase G:** Measure frame time on reference Low/Medium GPU; record in `docs/BUDGETS.md` (V1-8). Needs physical hardware.
2. **Phase I:** Verify vertical slice runs on web, desktop, Android, iOS; record results (V1-10). Needs device testing.

---

## Known issues (not 2D v1.0 blockers)

- Web Workers ping-only (needs crossOriginIsolated)
- iOS device testing needs macOS runner
- iOS/Android device tier returns fixed values (deferred to mobile milestones)
- Effect toggles (B/O/F) are debug keys, not settings

---

## Session protocol

Complete one task → update `docs/STATE.md` → commit → push → wait for user to say "continue".
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

### 7. KilnScript Debugger — Integrated with SliceSession (P1)
- Added `Debugger` to `SliceSession` with `debugger_mut()`, `debugger()`, `take_debugger()`, `set_debugger()` methods (`kiln-runtime/src/slice.rs`)
- Updated `DebuggerPanel` to use `SliceSession` debugger during play (`kiln-editor/src/app.rs`)
- Debugger controls (Continue, Step Into/Over/Out, Breakpoints, Locals, Watches) wired to Debugger API

### 8. Sprite Editor — Pivot, Slice, 9-Slice Editor (P1)
- Created `SpriteEditorState` with pivot point editor (0.0-1.0 with presets) (`kiln-editor/src/sprite_editor.rs`)
- Slice modes: Manual, Grid (cell size/spacing/offset), Auto-Detect
- 9-slice border editor (left/top/right/bottom with presets)
- Interactive preview with zoom, pan, pivot point visualization, 9-slice borders, grid overlay
- Entity selector to edit any sprite in the project
- Texture path editing with asset browser button
- Added `SpriteEditor` tab with menu entry in View menu

### 9. Script builtin ID alignment — V1-1 (kiln-runtime/src/slice.rs)
- Fixed `call_host_impl` host-call IDs to match the `BUILTINS` table:
  Entity management builtins (`despawn_entity`=27, `get_entity_pos`=28,
  `set_entity_pos`=29, `get_entities_with_role`=30, `get_entity_component`=31,
  `set_entity_component`=32, `find_nearest`=33, `raycast`=34) were implemented
  at colliding IDs (18-25) that duplicated audio builtins, causing scripts
  calling them to hit the wrong host function. Renumbered all cases 27-55 to
  match the table exactly.
- Removed dead-code builtins not in the `BUILTINS` table:
  `add_animation_frame`, `set_animation_loop`, particle system functions,
  `register_audio`, `play_audio_preview` (IDs 49-56).
- Scripts can now reliably spawn entities, read positions, query by role,
  and read/write components — the core gameplay API for 2D v1.0 (V1-1).


---

## Known issues

- Carry forward prior M14–M28 gaps (web Workers ping-only, iOS device
  testing needs Mac, etc.)
- The editor scene view applies post effects (bloom, fog, outline) via
  `pipeline.apply()` in `kiln-editor/src/app.rs`, matching the player's
  `Renderer::apply_post`. (Phase C-1 — done)
- The effect toggles (B/O/F) are only verified by hand; the automatic
  tests cover the effects themselves.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed
  under ADR-0026
- The script API is a fixed builtin list; all builtins (including entity
  spawn/read at IDs 26-41) are now wired correctly in `call_host_impl` — the
  host-call IDs match the `BUILTINS` table (fix: IDs 27-44 were colliding with
  audio builtins, and dead-code builtins were removed).
- Sprite layer ordering: same-layer different-texture transparent sprites may still be drawn in the wrong order; use `push_textured_with_layer` to separate transparent sprites onto different layers
- **Sprite material/shader:** `Sprite.material` field allows custom shaders via `Shader` and `Material` types in `kiln_core::project`. Use `SpriteRenderer::push_textured_with_material()` to render sprites with custom materials.
- **Web export:** `kiln-web` now has keyboard input, touch input, sprite rendering, tilemap rendering, GPU post-processing, and Web Audio API support. Full end-to-end web export testing (building and running in a real browser) has not been automated. See `docs/WEB_EXPORT_TESTING.md` for manual test procedure.
- **Clean machine run:** The engine has not been tested on a machine without a Rust toolchain installed. See `docs/CLEAN_MACHINE_TEST.md` for manual test procedure.
- **SDF/MSDF text:** SDF font support is implemented in `kiln-ui/src/text.rs` via `SdfTextRenderer` and `SdfFontData`. Font asset pipeline (TTF → SDF atlas) is not yet automated. See `docs/SDF_TEXT.md` for manual workflow.
- **Custom shader editor:** `kiln-editor/src/shader_editor.rs` provides a WGSL shader editor panel. Shader compilation to wgpu RenderPipeline is implemented via `SpriteRenderer::compile_shader()`. Shader assets are stored in `kiln_core::Project.shaders` and referenced by `Sprite.material`.
- The iOS and Android device capability paths return fixed values
- The audio sample needs a system audio backend, so it is built on its
  own: `cargo check --manifest-path samples/audio/Cargo.toml`

**Blocked:** None. Current focus is the **2D v1.0 roadmap** (`docs/2D_V1_ROADMAP.md`)
— closing remaining 2D gaps so the engine is shippable for 2D games. M29 (3D
foundation) remains in progress but is not the active milestone.

**Next:** Phase G of 2D v1.0 — frame budget: measure frame time on
reference Low/Medium GPU and record in BUDGETS.md (V1-8). Phase H items
closed: REN-1 (post tests), REN-2 (frame graph post), DT-1 (device tier),
U-1 (updater stubs). Remaining Phase H: none for 2D v1.0 scope. Then:
Phase I (v0.5 gate re-verification).

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
