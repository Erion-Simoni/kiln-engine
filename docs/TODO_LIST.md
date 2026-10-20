# Kiln TODO List - M0-M28 Stub & Incomplete Features

**Created:** 2026-10-20
**Status:** Work in progress - one item at a time
**Instructions:** Complete one item, mark it as `[x]`, then wait for user to say "continue"

---

## Summary
This file tracks all stubs, TODOs, and incomplete features across M0-M28. Based on the 2026-10-19 honesty audit in `docs/STATE.md`, many milestones marked as "done" are actually stubs or have incomplete implementations.

**Priority Order:** 2D-first gate (must complete before M29/3D)

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
- [ ] Implement memory-mapped file loader for native
- [ ] Implement HTTP streaming loader for web
- [ ] Test both loaders with same archive format
**Location:** `kiln-assets/src/archive.rs`
**Milestone:** R0

### R0-3: Audio device management (M11)
- [ ] Implement device selection
- [ ] Implement hot-plug detection
- [ ] Implement sample-rate changes
- [ ] Test on desktop
**Location:** `kiln-audio/src/device.rs` (currently stub)
**Milestone:** R0, M11

### R0-4: Budgets documentation
- [ ] Add native player size budget to `BUDGETS.md` (target: <15 MB executable)
- [ ] Add startup budget (<2s to first frame)
- [ ] Measure and record actual values
**Location:** `docs/BUDGETS.md`
**Milestone:** R0, M12

### R0-5: Window settings in project settings
- [ ] Add fullscreen option to project settings
- [ ] Add borderless option to project settings
- [ ] Add resolution option to project settings
- [ ] Add vsync option to project settings
- [ ] Add high-DPI option to project settings
- [ ] Add multi-monitor option to project settings
**Location:** `kiln-core/src/settings.rs`, `kiln.toml` schema
**Milestone:** R0

### R0-6: M13 sample completion
- [ ] Replace "N minutes" placeholder with real number
- [ ] Verify all M0-M13 checkboxes on clean machine
**Location:** `samples/` directories
**Milestone:** R0, M13

---

## Phase 1: Downloadable games and mobile (M14-M18)

### M14-1: Native desktop game export
- [x] ~~`kiln-cli export --platform windows|macos|linux` produces executable~~ (marked done in STATE.md)
- [ ] Verify no editor crates inside player (dependency check in CI)
- [ ] Test on clean machine with no Rust toolchain
- [ ] Implement per-OS save and settings folders (AppData, Application Support, XDG)
- [ ] Record player binary and data sizes separately in `BUDGETS.md`
**Location:** `kiln-cli/src/export.rs`, `kiln-player/`
**Milestone:** M14

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
**Location:** `kiln-cli/src/package.rs`, CI configuration
**Milestone:** M15

### M16-1: Storefront distribution
- [ ] itch.io: `kiln-cli publish itch` builds and uploads
- [ ] Document butler CLI integration or manual flow
- [ ] Steam: generate depot/build scripts
- [ ] Steam: Steamworks integration behind Cargo feature
- [ ] Steam: implement achievements, cloud saves, overlay
- [ ] Implement stub backend when Steam feature is off
- [ ] Implement platform-services trait (not locked to one store)
- [ ] Test Steam Deck / Linux handheld (controller, 800p, sleep/resume)
- [ ] Document fees, review requirements, account setup
**Location:** `kiln-platform/src/lib.rs` (stub exists), `kiln-cli/src/publish.rs`
**Milestone:** M16

### M17-1: Updates, patching, DLC packs
- [ ] Implement versioned data archives
- [ ] Implement delta patch generation between versions
- [ ] Implement optional in-game updater (check, download, verify hash, apply, rollback)
- [ ] Ensure store builds use store updater, in-game updater compiles out
- [ ] Implement additional content archives (DLC packs) with version dependencies
- [ ] Add opt-in crash-report hook (full tooling in M56)
**Location:** `kiln-updater/src/updater.rs` (stub when feature disabled)
**Milestone:** M17

### M18-1: Android export + touch input
- [ ] APK builds from `just android`
- [ ] Test on physical Android device
- [ ] Implement touch input (tap, drag, multi-touch) reaching ECS systems
- [ ] Test suspend/resume survival
- [ ] Test rotation (surface recreated)
- [ ] Signed AAB builds in CI
- [ ] v0.4 gate: same game runs as native download on desktop and Android
**Location:** Android glue code (via ADR), `kiln-runtime/src/input.rs`
**Milestone:** M18

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
- [ ] Implement Export dialog (line 645 in `kiln-editor/src/app.rs`)
- [ ] Implement Undo (line 656)
- [ ] Implement Redo (line 660)
- [ ] Implement Package (line 716)
**Location:** `kiln-editor/src/app.rs` lines 645-716
**Milestone:** M7, M14, M15

### E-2: Help menu actions
- [ ] Implement Open docs (line 730)
- [ ] Implement About dialog (line 734)
**Location:** `kiln-editor/src/app.rs` lines 730-734
**Milestone:** M7

### E-3: FPS timing
- [ ] Implement proper FPS timing (line 962)
**Location:** `kiln-editor/src/app.rs` line 962
**Milestone:** M7

### E-4: Scene Editor panel
- [ ] Implement Add Entity (line 1131)
- [ ] Implement Component menu for Transform (line 1188)
- [ ] Implement Component menu for Sprite (line 1245)
- [ ] Implement Add Component (line 1329)
**Location:** `kiln-editor/src/app.rs` lines 1131-1329
**Milestone:** M7

### E-5: Assets panel
- [ ] Implement Import assets (line 1504)
**Location:** `kiln-editor/src/app.rs` line 1504
**Milestone:** M7

### E-6: Console panel
- [ ] Implement Clear console (line 1531)
**Location:** `kiln-editor/src/app.rs` line 1531
**Milestone:** M7

### E-7: Prefab Editor panel
- [ ] Implement Create prefab from selected entity (line 1812)
- [ ] Implement Instantiate selected prefab (line 1818)
- [ ] Implement Edit prefab (line 1853)
**Location:** `kiln-editor/src/app.rs` lines 1812-1853
**Milestone:** M23

### E-8: Scene Editor panel (scene-specific)
- [ ] Implement Create new scene (line 1881)
- [ ] Implement Add nested scene (line 1887)
**Location:** `kiln-editor/src/app.rs` lines 1881-1887
**Milestone:** M23

### E-9: Editor scene view
- [ ] Replace egui painted rects with actual engine framebuffer
- [ ] Integrate real GPU rendering in scene view
**Location:** `kiln-editor/src/scene_view.rs`
**Milestone:** M7

### E-10: Play-in-editor
- [ ] Replace bool toggle with real VM/runtime PIE
- [ ] Integrate VM execution in editor context
- [ ] Handle scene serialization/deserialization for PIE
**Location:** `kiln-editor/src/app.rs`, `kiln-runtime/`
**Milestone:** M7

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

3. **2D-first gate**: Do not start M29 (3D) until the 2D gate items are complete:
   - Textured sprites (done)
   - Real tilemap (not done)
   - Asset → GPU (not done)
   - Editor truth (not done)
   - Standalone player (done)
   - VM maturity (not done)
   - Vertical slice sample (not done)
   - UI draw path (done)
   - Post (partially done - needs manual verification)
   - Reconcile ROADMAP checkboxes (not done)

4. **Session protocol**: Complete one item, mark as `[x]`, update `docs/STATE.md`, commit, then wait for user to say "continue".

---

## References

- `docs/STATE.md` - Current state and audit
- `docs/fixes m1-29.md` - Prior fixes log
- `ROADMAP_FULL_M14_PLUS.md` - Detailed milestone plan
- `AGENTS.md` - Coding rules and session protocol
