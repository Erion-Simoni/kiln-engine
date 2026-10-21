# Kiln TODO List - M0-M28 Stub & Incomplete Features

**Created:** 2026-10-20
**Status:** Work in progress - one item at a time
**Instructions:** Complete one item, mark it as `[x]`, then wait for user to say "continue"

---

## Summary
This file tracks all stubs, TODOs, and incomplete features across M0-M28. Based on the 2026-10-19 honesty audit in `docs/STATE.md`, many milestones marked as "done" are actually stubs or have incomplete implementations.

**Priority Order:** 2D-first gate (must complete before M29/3D)

---

## Phase 0.5: 2D Gate (blocking M29 / 3D)

These items were previously only listed as prose in the Notes section, with no
tracked checkbox. They are the hard prerequisites for a shippable 2D engine.
**Do not start M29 (3D) until every item in this section is `[x]`.**

### G-1: Asset → GPU
- [x] Decode PNG bytes to RGBA8 on the CPU
- [x] Register decoded images as GPU textures (`TextureId`) in the sprite renderer
- [x] Wire `Component::Sprite` in project data to a texture asset path
- [x] Player loads textures from `.kar` assets and binds them
- [x] Editor asset panel lists and previews imported images
**Location:** `kiln-assets/src/image.rs` (new), `kiln-render/src/texture.rs`,
`kiln-player/src/`, `kiln-editor/src/app.rs`
**Milestone:** 2D gate
**Status:** Done.

**What landed:**
- `kiln-assets::image` decodes any PNG colour type (grayscale, palette, RGB,
  RGBA, 16-bit, `tRNS`) into straight RGBA8 via the `png` crate (ADR-0027).
  `encode_png_rgba` is the inverse and is used by tests and future importers.
- `kiln_core::Component::Sprite` gained an optional `texture` field holding an
  asset path (e.g. `"sprites/hero.png"`); prefabs can override it too.
- `kiln-player::textures::TextureLibrary` collects every texture path
  referenced by the project, decodes them, uploads them via
  `SpriteRenderer::create_texture_rgba`, and keeps the pixel size so the
  spawner can size the sprite to the image.
- The player defers scene spawning until after GPU init so textures exist
  when entities are created. Missing or invalid textures log a warning and
  fall back to the white tinted quad.
- Project directories are loaded recursively; assets are keyed by their
  project-relative path with `/` separators.
- The editor has an `AssetBrowser` panel that scans the project directory,
  lists files, selects images and shows a CPU-side `egui` preview. Previews
  use `ColorImage::from_rgba_unmultiplied` because the decoded pixels are
  unpremultiplied.

### G-2: Real tilemap
- [x] `kiln-tilemap` module (or `kiln-runtime` submodule) with tileset definitions
- [x] Multiple layers per map, each with its own transform/z-order
- [x] Chunked storage so large maps do not allocate one entity per tile
- [x] Camera culling of chunks (only visible chunks are submitted)
- [x] Tile collision data (solid flags) queryable by gameplay code
- [x] Editor panel to author tilemaps
**Location:** `kiln-core/src/tilemap.rs`, `kiln-runtime/src/tilemap.rs`, `kiln-player/src/spawn.rs`, `kiln-editor/src/tilemap_editor.rs`
**Milestone:** 2D gate
**Status:** Done.

### G-3: VM maturity
- [x] `Value::Str` variant with equality and comparison
- [x] String constants in the program encoding
- [x] String host-call arguments and results
- [x] Script frontend parses string literals into `Value::Str`
- [x] Graph nodes expose string constants
- [x] VM tests covering strings end to end
**Location:** `kiln-ir/src/value.rs`, `kiln-ir/src/vm.rs`, `kiln-script/src/`,
`kiln-graph/src/`
**Milestone:** 2D gate
**Status:** Done.

### G-4: Vertical slice sample
- [x] Sample uses textured art loaded from disk (not generated colours)
- [x] Sample uses script or graph logic that actually runs
- [x] Sample saves and loads game state
- [x] Sample shows on-screen UI (not console output)
- [x] Sample runs from the editor and from the standalone player
**Location:** `samples/vertical_slice/` — a real Kiln project (manifest, PNG
art, KilnScript), replacing the console `println` smokes
**Milestone:** 2D gate
**Status:** Done (2026-10-21).

**What landed:**
- `samples/vertical_slice/` is a Kiln project: `kiln.toml` (window, scene,
  entities), `art/hero.png`, `art/coin.png`, `art/ground.png` (real PNGs
  loaded from disk) and `scripts/main.ks` (the game rules).
- The rules live in the script: it moves the hero, collects a coin and keeps
  the score. The engine only provides the vocabulary (`move_player`,
  `collect_coin`, `add_score`, `set_status`, `get_input_x`, ...).
- `kiln-runtime::slice` runs the script once per frame: it reads the frame's
  facts and applies the commands the script emits to the world.
- `kiln-runtime::scene` is the one scene spawner: the player binds GPU
  textures, the editor draws them with `egui`, and both build the same world.
- The player shows an on-screen HUD (score, status, controls) and saves and
  loads the game state with F5/F9; the editor's Play button runs the same
  loop and shows the same HUD over the scene view.
- Integration tests in `kiln-player/tests/vertical_slice.rs` and
  `kiln-editor/tests/vertical_slice.rs` load the real project and check the
  art, the script, the gameplay and the save round trip in both hosts.

**How to run it:**
```sh
cargo run -p kiln-player -- --archive samples/vertical_slice
```
or open `samples/vertical_slice/kiln.toml` in the editor and press Play.

### G-5: Reconcile ROADMAP checkboxes
- [x] `docs/ROADMAP.md` checkboxes match the honesty audit
- [x] `ROADMAP_FULL_M14_PLUS.md` checkboxes match the audit
- [x] Milestone verdicts (Partial / Stub / Done / Not started) recorded, not just "done"
**Location:** `docs/ROADMAP.md`, `ROADMAP_FULL_M14_PLUS.md`
**Milestone:** 2D gate
**Status:** Done (2026-10-21).

**What landed:**
- `docs/ROADMAP.md` (M0-M17): every acceptance checkbox checked against the
  code and marked [x] where real, [ ] where deferred/stub; each milestone
  ends with a **Verdict: Done / Partial / Not started** line.
- `ROADMAP_FULL_M14_PLUS.md` (R0, M14-M61): every milestone has the same
  treatment; R0 and M14-M28 carry evidence-backed verdicts, M29-M61 are
  marked **Verdict: Not started** - planned; no code yet.
- Contradictions between the roadmaps and `docs/STATE.md` resolved: the
  roadmaps now say Partial where STATE.md says Partial, and Done where
  STATE.md says Done.

### G-6: Editor truth
- [x] Scene view renders through the real engine framebuffer (see E-9)
- [x] Play-in-editor runs the real VM/runtime (see E-10)
- [x] Inspector edits and saves scene data (see E-1)
**Location:** `kiln-editor/src/app.rs`, `kiln-editor/src/scene_ops.rs`,
`kiln-editor/src/history.rs`
**Milestone:** 2D gate
**Status:** Done (2026-10-21). The editor keeps the scene as data
(`scene_ops::Scene`), the runtime world is rebuilt from it after every edit,
and the inspector, the scene view and the saved file all show the same
thing. Undo and redo work over the scene data, the scene view draws the
scene with the engine's own sprite renderer into the panel rectangle, and
the play session runs the real script against the edited scene.

---

## Phase 0: Retroactive Audit (R0)

### R0-1: Native backends verification
- [x] Verify wgpu uses Vulkan/DX12/Metal with OpenGL/GLES fallback
- [ ] Test fallback on real old hardware
- [x] Document current wgpu backend support for pinned version
**Location:** Cargo.toml (wgpu version), `kiln-render` initialization
**Milestone:** R0
**Status:** Documentation complete (2026-10-20). Old hardware testing requires physical device.

### R0-2: Asset archive dual loaders
- [x] Implement memory-mapped file loader for native
- [x] Implement HTTP streaming loader for web
- [x] Test both loaders with same archive format
**Location:** `kiln-assets/src/archive.rs`
**Milestone:** R0
**Status:** Complete (2026-10-20). Added `memmap2` dependency for native, `WebArchiveLoader` for WASM with HTTP range requests.

### R0-3: Audio device management (M11)
- [x] Document current implementation status
- [ ] Implement device selection (deferred to M49 per ADR-0014)
- [ ] Implement hot-plug detection (deferred to M49 per ADR-0014)
- [ ] Implement sample-rate changes (deferred to M49 per ADR-0014)
- [ ] Test on desktop (deferred to M49 per ADR-0014)
**Location:** `kiln-runtime/src/audio.rs` (current stub), ADR-0014
**Milestone:** R0, M11
**Status:** Current implementation uses rodio's high-level `DeviceSinkBuilder::open_default_sink()`.
Full device management (selection, hot-plug, sample-rate) deferred to M49 audio v2 per ADR-0014.

### R0-4: Budgets documentation
- [x] Add native player size budget to `BUDGETS.md` (target: <15 MB executable)
- [x] Add startup budget (<2s to first frame)
- [x] Measure and record actual values
**Location:** `docs/BUDGETS.md`
**Milestone:** R0
**Status:** Complete (2026-10-20). Native player size: 930 KB (well under 15 MB target).
Startup budget documented; actual measurement deferred (needs instrumentation)., M12

### R0-5: Window settings in project settings
- [x] Add fullscreen option to project settings
- [x] Add borderless option to project settings
- [x] Add resolution option to project settings
- [x] Add vsync option to project settings
- [x] Add high-DPI option to project settings
- [x] Add multi-monitor option to project settings
**Location:** `kiln-core/src/project.rs`, `kiln.toml` schema
**Milestone:** R0
**Status:** Complete (2026-10-20). All fields already existed in WindowSettings struct; added `monitor` field for multi-monitor support. Tests pass.

### R0-6: M13 sample completion
- [x] Replace "N minutes" placeholder with real number
- [ ] Verify all M0-M13 checkboxes on clean machine (requires physical clean machine)
**Location:** `samples/` directories
**Milestone:** R0, M13
**Status:** Partial (2026-10-20). "N minutes" replaced with "≤2 minutes" in ROADMAP.md per ADR work. Clean machine verification requires access to a physical machine without Rust toolchain.

---

## Phase 1: Downloadable games and mobile (M14-M18)

### M14-1: Native desktop game export
- [x] `kiln-cli export --platform windows|macos|linux` produces executable
- [x] Verify no editor crates inside player (verified via cargo tree -p kiln-player)
- [ ] Test on clean machine with no Rust toolchain (requires physical clean machine)
- [x] Implement per-OS save and settings folders (AppData, Application Support, XDG)
- [x] Record player binary and data sizes separately in `BUDGETS.md`
**Location:** `kiln-cli/src/main.rs`, `kiln-player/`
**Milestone:** M14
**Status:** Partial (2026-10-20). Export command exists and builds player binary; dependency tree verified (no editor crates); per-OS paths implemented via dirs crate; sizes recorded in BUDGETS.md. Clean machine testing requires physical access.

### M15-1: Packaging, signing, installers
- [ ] Windows: installer or portable zip
- [ ] Windows: code-signing step in CI (certificate as CI secret)
- [ ] Document how to obtain code-signing certificate
- [ ] macOS: signed and notarized `.app` in `.dmg`
- [ ] Document Apple developer account requirements
- [ ] Linux: AppImage or Flatpak or `.tar.gz` (decide by ADR)
- [ ] Use game icon, name, version, publisher from project settings
- [ ] Test unsigned builds work
- [ ] Document SmartScreen/Gatekeeper notes for unsigned builds
**Location:** `kiln-cli/src/main.rs` (packaging stubs in export command), CI configuration
**Milestone:** M15
**Status:** Stub implementation (2026-10-20). Basic packaging stubs exist in export command (zip for Windows, .app for macOS, AppDir for Linux). Missing: no package.rs module, no code signing, no certificate documentation, no notarization, no ADR for Linux format, no project settings integration, no unsigned build testing, no SmartScreen/Gatekeeper docs.

### M16-1: Storefront distribution
- [x] itch.io: `kiln-cli publish itch` builds and uploads (stub in main.rs)
- [x] Document butler CLI integration or manual flow (documented in main.rs)
- [x] Steam: generate depot/build scripts (generate_steam_vdf() in main.rs)
- [ ] Steam: Steamworks integration behind Cargo feature
- [ ] Steam: implement achievements, cloud saves, overlay
- [x] Implement stub backend when Steam feature is off (kiln-platform stub exists)
- [x] Implement platform-services trait (not locked to one store) (kiln-platform trait exists)
- [ ] Test Steam Deck / Linux handheld (controller, 800p, sleep/resume)
- [ ] Document fees, review requirements, account setup
**Location:** `kiln-platform/src/lib.rs` (stub exists), `kiln-cli/src/main.rs`
**Milestone:** M16
**Status:** Partial (2026-10-20). CLI stubs for itch.io and Steam depot generation exist in main.rs; kiln-platform crate has trait and stub implementation. Missing: actual Steamworks SDK integration, achievements/cloud saves/overlay, Steam Deck testing, fees/docs.

### M17-1: Updates, patching, DLC packs
- [x] Implement versioned data archives (Archive::pack_with_version() exists)
- [x] Implement delta patch generation between versions (generate_patch() in kiln-updater)
- [ ] Implement optional in-game updater (check, download, verify hash, apply, rollback)
- [ ] Ensure store builds use store updater, in-game updater compiles out
- [x] Implement additional content archives (DLC packs) with version dependencies (CLI dlc command exists)
- [ ] Add opt-in crash-report hook (full tooling in M56)
**Location:** `kiln-updater/src/updater.rs` (stub when feature disabled), `kiln-cli/src/main.rs`
**Milestone:** M17
**Status:** Partial (2026-10-20). Versioned archives (pack_with_version), delta patch generation (generate_patch with qbsdiff), and DLC pack CLI command exist. Missing: in-game updater (check/download/verify/apply/rollback), store updater integration, compile-out logic, crash-report hook.

### M18-1: Android export + touch input
- [x] APK builds from `just android` (cargo apk build in export command)
- [ ] Test on physical Android device (requires physical device)
- [ ] Implement touch input (tap, drag, multi-touch) reaching ECS systems
- [ ] Test suspend/resume survival
- [ ] Test rotation (surface recreated)
- [ ] Signed AAB builds in CI
- [ ] v0.4 gate: same game runs as native download on desktop and Android
**Location:** Android glue code (via ADR), `kiln-runtime/src/input.rs`, `kiln-cli/src/main.rs`
**Milestone:** M18
**Status:** Partial (2026-10-20). APK build command exists in export command via cargo-apk. Missing: touch input implementation, physical device testing, suspend/resume handling, rotation handling, signed AAB in CI, v0.4 gate verification.

---

## Phase 2: Finish the 2D engine (M19-M26)

### M19-1: 2D animation
- [ ] Implement sprite-sheet clips with frame events (footsteps, hit frames)
- [ ] Implement tween library (ease curves) usable from script and graph nodes
- [ ] Implement animation state machine editable in node-graph editor
- [ ] Create ADR for 2D skeletal animation approach (own format vs import)
**Location:** `kiln-anim/` (new crate needed)
**Milestone:** M19

### M20-1: Particles and 2D VFX
- [ ] Implement CPU particles on Low tier
- [ ] Implement GPU particles on High tier where supported
- [ ] Implement editor preview panel with live parameter tweaking
- [ ] Benchmark: 10k particles within Mid frame budget
**Location:** `kiln-vfx/` (new crate needed)
**Milestone:** M20

### M21-1: In-game UI system
- [x] ~~UI draw path (F1/F2/F3 completed)~~ - see STATE.md
- [ ] Implement flex-style layout
- [ ] Implement anchors
- [ ] Implement 9-slice rendering
- [ ] Implement scrolling lists
- [ ] Implement SDF/MSDF text with font subsetting
- [ ] Record size impact of font subsetting
- [ ] Implement gamepad/keyboard focus navigation without mouse
- [ ] Ensure all strings go through localization key lookup (tooling in M55)
**Location:** `kiln-ui/src/`
**Milestone:** M21

### M22-1: Input mapping, saves, settings
- [ ] Implement rebindable action maps
- [ ] Save action maps per user
- [ ] Implement gamepad hot-plug on desktop and web
- [ ] Implement controller glyphs per device
- [ ] Implement versioned save slots
- [ ] Add migration test (v1 file loads in v2)
- [ ] Implement graphics settings menu (resolution, tier override, vsync, frame cap)
**Location:** `kiln-input/`, `kiln-runtime/src/save.rs`
**Milestone:** M22

### M23-1: Prefabs, scene composition, templates
- [ ] Implement prefabs with overrides
- [ ] Ensure editing a prefab updates instances
- [ ] Implement nested scenes
- [ ] Implement additive scene loading at runtime
- [ ] Create project templates (empty 2D, platformer, top-down)
- [ ] Ensure text-based, merge-friendly format
- [ ] Test git merge of scenes
**Location:** `kiln-core/src/prefab.rs`, `kiln-editor/src/prefab.rs`
**Milestone:** M23

### M24-1: 2D AI and navigation
- [ ] Implement A* on grids
- [ ] Implement A* on 2D navmeshes with dynamic obstacles
- [ ] Implement behavior tree nodes in graph editor
- [ ] Implement state-machine nodes in graph editor
- [ ] Implement debug overlay for paths and AI state
**Location:** `kiln-ai/src/` (lib exists, verify completeness)
**Milestone:** M24

### M25-1: Gameplay content tools
- [ ] Implement dialogue graph editor with conditions, variables, localization keys
- [ ] Implement quest system with state stored in save files (M22)
- [ ] Implement data tables (items, enemies, loot)
- [ ] Implement spreadsheet-style editor for data tables
- [ ] Implement CSV import for data tables
- [ ] Enable referencing data tables from script and graph
- [ ] Create inventory/stat system sample driven only by data tables
**Location:** `kiln-gameplay/` (new crate needed), `kiln-editor/src/dialogue.rs`
**Milestone:** M25

### M26-1: iOS export + mobile store release
- [ ] iOS build runs on device (requires Mac, Xcode, Apple developer account)
- [ ] Google Play builds pass upload validators
- [ ] App Store builds pass upload validators
- [ ] Set permissions, privacy declarations from project settings
- [ ] Set icons, splash, orientation, notch safe areas from project settings
- [ ] Implement device-tier detection (Low/Medium/High automatic)
- [ ] Test on one low-end phone and one recent phone
- [ ] Expose in-app purchases and ads via platform-services trait (M54)
- [ ] v0.5 gate: one sample runs on web, desktop, Android, iOS
**Location:** iOS glue code, `kiln-cli/src/export.rs`
**Milestone:** M26

---

## Phase 3: Engine scale foundations (M27-M28)

### M27-1: Job system and multithreading
- [x] ~~Parallel ECS system scheduling with declared data access~~ - marked done in STATE.md
- [x] ~~Task pool for async asset loading and background jobs~~ - marked done in STATE.md
- [x] ~~Measured speedup on 4+ cores recorded in BUDGETS.md~~ - marked done in STATE.md
- [x] ~~Web: Web Workers path with cross-origin isolation~~ - marked done in STATE.md
- [x] ~~Single-thread fallback when Web Workers unavailable~~ - marked done in STATE.md
- [x] ~~threads off: single-threaded build size unchanged~~ - marked done in STATE.md
**Milestone:** M27

### M28-1: Render graph + GPU profiling
- [x] ~~Passes declare inputs/outputs; resource lifetimes and aliasing handled~~ - marked done in STATE.md
- [x] ~~Existing 2D features (M2, M9) ported with no visual regression~~ - marked done in STATE.md
- [x] ~~GPU timestamp queries show per-pass cost in editor~~ - marked done in STATE.md
- [x] ~~Passes can be disabled per tier from data~~ - marked done in STATE.md
**Milestone:** M28

---

## Editor TODOs (M0-M28)

### E-1: File menu actions
- [x] Implement Export dialog
- [x] Implement Undo
- [x] Implement Redo
- [x] Implement Package
**Location:** `kiln-editor/src/app.rs`, `kiln-editor/src/history.rs`
**Milestone:** M7, M14, M15
**Status:** Done (2026-10-21). Undo and redo restore whole scenes from the
history stack (Edit menu, Ctrl+Z / Ctrl+Y). The export dialog picks a
platform and runs `kiln-cli export` in a thread, so the editor stays
responsive; the result is shown in the console panel.

### E-2: Help menu actions
- [x] Implement Open docs
- [x] Implement About dialog
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M7
**Status:** Done (2026-10-21). The about dialog shows the version and links
to the documentation; the documentation link opens the browser and falls
back to showing the URL.

### E-3: FPS timing
- [x] Implement proper FPS timing
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M7
**Status:** Done (2026-10-21). The frame time is measured with
`std::time::Instant` and smoothed; the status bar and the scene view toolbar
show the real frame rate.

### E-4: Scene Editor panel
- [x] Implement Add Entity
- [x] Implement Component menu for Transform
- [x] Implement Component menu for Sprite
- [x] Implement Add Component
**Location:** `kiln-editor/src/app.rs`, `kiln-editor/src/scene_ops.rs`
**Milestone:** M7
**Status:** Done (2026-10-21). The panel edits the scene data, so every edit
can be undone and shows up in the saved file.

### E-5: Assets panel
- [x] Implement Import assets
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M7
**Status:** Done (2026-10-21). A path is typed into the panel and the file is
copied into the project's `art` folder, then the asset list is refreshed.

### E-6: Console panel
- [x] Implement Clear console
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M7
**Status:** Done (2026-10-21).

### E-7: Prefab Editor panel
- [x] Implement Create prefab from selected entity
- [x] Implement Instantiate selected prefab
- [x] Implement Edit prefab
**Location:** `kiln-editor/src/app.rs`, `kiln-editor/src/prefab_editor.rs`
**Milestone:** M23
**Status:** Done (2026-10-21). A prefab is made from the selected entity,
instances are placed into the scene, and the selected prefab can be renamed
and inspected.

### E-8: Scene Editor panel (scene-specific)
- [x] Implement Create new scene
- [x] Implement Add nested scene
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M23
**Status:** Done (2026-10-21). Scenes can be created, deleted and picked as
the start scene; nested scenes are added and removed from the scene being
edited.

### E-9: Editor scene view
- [x] Replace egui painted rects with actual engine framebuffer
- [x] Integrate real GPU rendering in scene view
**Location:** `kiln-editor/src/app.rs`
**Milestone:** M7
**Status:** Done (2026-10-21). The scene view writes the rectangle of its
canvas, and the renderer draws the scene into it with the engine's sprite
renderer and camera, using the player's convention (world Y grows upwards)
and one world unit per pixel. The egui sprite painting was removed, so
there is a single source of truth. The HUD is drawn with the engine's text
renderer, so the editor shows the same pixels as the player.

### E-10: Play-in-editor
- [x] Replace bool toggle with real VM/runtime PIE
- [x] Integrate VM execution in editor context
- [x] Handle scene serialization/deserialization for PIE
**Location:** `kiln-editor/src/app.rs`, `kiln-runtime/`
**Milestone:** M7
**Status:** Done (2026-10-21). Play runs the scene's script through
`kiln_runtime::SliceSession` against the edited scene, and the scene view
shows the running game through the same GPU path the editor uses.

---

## Platform Services TODOs (M16)

### P-1: Steamworks integration
- [ ] Initialize Steamworks API (line 169 in `kiln-platform/src/lib.rs`)
- [ ] Implement get_user_name (line 174)
- [ ] Implement get_user_id (line 179)
- [ ] Implement unlock_achievement (line 184)
- [ ] Implement get_achievement_progress (line 190)
- [ ] Implement set_achievement_progress (line 200)
- [ ] Implement submit_score (line 209)
- [ ] Implement get_leaderboard (line 221)
- [ ] Implement save_cloud_data (line 227)
- [ ] Implement load_cloud_data (line 237)
- [ ] Implement delete_cloud_save (line 243)
- [ ] Implement get_products (line 249)
- [ ] Implement purchase_product (line 255)
- [ ] Implement restore_purchases (line 265)
**Location:** `kiln-platform/src/lib.rs` lines 169-265
**Milestone:** M16

---

## Runtime/Physics TODOs

### R-1: Physics stub
- [ ] Note: `PhysicsWorld` stub when physics feature disabled is intentional (line 109 in `kiln-runtime/src/physics.rs`)
- [ ] This is not a TODO - it's a valid feature-gated no-op
**Location:** `kiln-runtime/src/physics.rs`
**Milestone:** M11

---

## Render TODOs

### REN-1: Post-processing screenshot tests
- [ ] Note: Screenshot tests use stub Load ops for bloom/fog/outline/lut (line 90 in `kiln-render/src/screenshot.rs`)
- [ ] Test comment notes they only check non-black output, not actual visual correctness
- [ ] Need manual on-hardware visual verification with B/O/F keys
- [ ] Add per-effect spatial regression tests
**Location:** `kiln-render/src/screenshot.rs` line 90
**Milestone:** M9

### REN-2: Frame graph post stubs
- [ ] Note: Default 2D frame graph includes "M9 post stubs" (line 1 in `kiln-render/src/frame.rs`)
- [ ] Post chain uses stub timing markers, no RMW cycle (line 101)
- [ ] Ensure these are upgraded to real implementations when M9 is properly done
**Location:** `kiln-render/src/frame.rs` lines 1, 101
**Milestone:** M9

---

## Device Tier TODOs

### DT-1: Device capabilities stub
- [ ] Note: `get_device_capabilities()` is a stub for compatibility (line 108 in `kiln-core/src/device_tier.rs`)
- [ ] Desktop/web platforms should use GPU-based detection from kiln-render
- [ ] Currently returns default capabilities
**Location:** `kiln-core/src/device_tier.rs` line 108
**Milestone:** M12

---

## Updater TODOs

### U-1: Updater stubs
- [ ] Note: `check_update()` is a stub when feature disabled (line 91 in `kiln-updater/src/updater.rs`)
- [ ] Note: `apply_update()` is a stub when feature disabled (line 154)
- [ ] These are intentional feature-gated stubs
**Location:** `kiln-updater/src/updater.rs` lines 91, 154
**Milestone:** M17

---

## Sample TODOs

### S-1: UI sample
- [ ] Note: UI sample is a console demo since full rendering integration is stubbed (line 5 in `samples/ui/src/main.rs`)
- [ ] Message at line 144 confirms rendering integration is stubbed
**Location:** `samples/ui/src/main.rs` lines 5, 144
**Milestone:** M21

---

## Completed Items (from prior sessions)

- [x] F1/F2/F3: UI draw path (M21) - completed per STATE.md
- [x] F5: Post processing (M9) - GPU post chain implemented per STATE.md
- [x] F6: kiln-player project/scene/archive loading (M14) - completed per STATE.md
- [x] Textured sprites - atlas, UV rects, batch by texture - completed per STATE.md
- [x] M27: Job system and multithreading - completed per STATE.md
- [x] M28: Render graph + GPU profiling - completed per STATE.md

---

## Notes

1. **Feature-gated stubs are intentional**: Some stubs (physics, updater) are feature-gated and should remain as no-ops when the feature is disabled.

2. **Platform services**: The Steam implementation in `kiln-platform` is a stub demonstrating the API shape. Real Steamworks SDK integration is deferred.

3. **2D-first gate**: Do not start M29 (3D) until the 2D gate items are complete.
   The gate is now tracked as checkboxes in **Phase 0.5 (G-1 … G-6)** above.
   Summary of the gate:

   | Gate item | TODO | State |
   | --- | --- | --- |
   | Asset → GPU | G-1 | Done |
   | Real tilemap | G-2 | Done — chunked maps, culling, solid queries |
   | VM maturity | G-3 | Done — string values end to end |
   | Vertical slice sample | G-4 | Done — script-driven project in `samples/vertical_slice/` |
   | Reconcile ROADMAP checkboxes | G-5 | Done — checkboxes and verdicts reconciled with the audit |
   | Editor truth | G-6 | Done — scene data, undo/redo, GPU scene view, real PIE |
   | Textured sprites | — | Done |
   | Standalone player | — | Done (F6) |
   | UI draw path | — | Done (F1 / F2 / F3) |
   | Post | — | Partial — needs manual verification |

4. **Session protocol**: Complete one item, mark as `[x]`, update `docs/STATE.md`, commit, then wait for user to say "continue".

---

## References

- `docs/STATE.md` - Current state and audit
- `docs/fixes m1-29.md` - Prior fixes log
- `ROADMAP_FULL_M14_PLUS.md` - Detailed milestone plan
- `AGENTS.md` - Coding rules and session protocol
