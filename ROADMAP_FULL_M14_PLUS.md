# Kiln Roadmap: everything after M13 (single file)

**Status:** Prior sessions claimed M0–M28; the **2026-10-19 honesty audit**
(`docs/STATE.md`) rated the project **Partial** at the time. The 2D-first gate
(textured sprites, tilemap, editor/player wiring) is now **met**, so M29 (3D)
may start.
This file replaces the old "M14+" section of `ROADMAP.md` and **supersedes**
`ROADMAP_M14_PLUS.md` and `ROADMAP_DOWNLOADABLE_GAMES.md` (merged/renumbered).

Same rules as before: one focused session per milestone, never start the next
milestone unasked, every acceptance item must be testable (command, screenshot,
or automated check). Milestones marked **(2 sessions)** are too big for one
chat: split into `Mxx-a` / `Mxx-b` and keep each half testable.

---

## Principles for everything below

1. **Every feature has a tier.** Low / Medium / High, with a declared cost in ms
   and MB and a fallback. Nothing is "High only" unless explicitly marked
   native-only.
2. **Core stays small.** 3D, networking, physics, audio, threads, XR are Cargo
   features and compile out. A 2D web game never pays for 3D code.
3. **Every milestone ends with a demo scene** in `samples/` and a recorded
   frame-time and size number in `BUDGETS.md`.
4. **Profile before optimizing.** From M28 on, GPU timing exists, so
   performance claims need a measurement.
5. **No new dependency without an ADR** (size, license, maintenance).
6. **WebGPU limits are real.** As far as I know there is no hardware ray tracing
   or mesh shader support on the web (verify at the time). Mark such features
   native-only.
7. **A reference game drives features.** Build what a real game needs.
8. **Test on real old hardware** at the end of every phase, not only on your
   development machine.

## Release gates

| Gate | After | Meaning |
|---|---|---|
| v0.4 "Downloadable" | M18 | Games ship as native downloads on desktop and Android |
| v0.5 "2D complete" | M26 | A full 2D game can be made and shipped to web, desktop, Android, iOS |
| v0.8 "3D playable" | M39 | A 3D sample runs on Mid hardware with lighting, shadows, animation, physics, terrain |
| v0.9 "Looks great" | M47 | High tier delivers modern visuals; Low tier still meets budget |
| v1.0 "Production" | M61 | Tooling, plugins, mods, docs and three flagship samples shipped |

---

# R0 — Retrofit audit of M0 to M13 (do this first, one session)

Because M0 to M13 were built before the native-game and mobile work was
planned, verify these on the existing code. Fix gaps now; they get expensive later.

**Acceptance:**

- [x] **Native backends:** wgpu uses Vulkan/DX12/Metal and falls back to
      OpenGL/GLES on old GPUs. Test the fallback on real old hardware.
      (Verify current wgpu backend support for your pinned version.)
      The backend choice and the fallback are documented (ADR-0012); the
      fallback has not been run on old hardware.
- [x] **Assets:** one archive format with two loaders: memory-mapped file for
      native, HTTP streaming for web (ADR-0013).
- [ ] **Audio (M11):** device selection, hot-plug and sample-rate changes work
      on desktop.
      Deferred to M49 (ADR-0014): the engine opens the default device only.
- [x] **Budgets (M12):** `BUDGETS.md` has a native player size budget (suggested
      start: under 15 MB executable without assets, adjust after measuring) and
      a startup budget (under 2 s to first frame).
- [x] **Input/settings:** window options exist in project settings (fullscreen,
      borderless, resolution, vsync, high-DPI, multi-monitor) (ADR-0015).
- [ ] **M13 sample:** the "N minutes" placeholder is replaced with a real
      number; all M0 to M13 checkboxes verified on a clean machine.
      The placeholder is gone and `samples/vertical_slice/` is a real
      project; the clean-machine check is still missing.
- [x] `docs/STATE.md` rewritten to mark M0 to M13 done and M14 as next.

**Verdict: Partial** — the retrofit items are in place except the audio
device work (deferred), the clean-machine check and the old-GPU test.

---

# Phase 1: Downloadable games and mobile (M14 to M18)

## M14 — Native desktop game export
**Goal:** One editor action and one CLI command produce a standalone game for
Windows, macOS and Linux containing only the runtime and the game's data.
**Crates:** `kiln-cli`, `kiln-runtime`, new `kiln-player` (thin launcher).
**Acceptance:**
- [x] `kiln-cli export --platform windows|macos|linux` produces an executable
      plus the packed archive (M3).
- [x] No editor crates inside the player (dependency check in CI).
- [ ] Runs on a clean machine with no Rust toolchain and no editor installed.
      The player loads a project, runs the scene script and saves/loads the
      game, but no clean machine has run it yet.
- [x] Per-OS save and settings folders (AppData, Application Support, XDG).
- [x] Player binary and data sizes recorded separately in `BUDGETS.md`.

**Verdict: Partial** — the export, the player and the budgets are real; the
clean-machine run is still missing.

## M15 — Packaging, signing, installers for games
**Goal:** Players install games without scary OS warnings.
**Crates:** `kiln-cli`, packaging config.
**Acceptance:**
- [x] Windows: installer or portable zip; code-signing step in CI (certificate
      as a CI secret; documented how to obtain one).
      The ZIP step and the code-signing plan are in place; the signing step
      itself is not implemented.
- [ ] macOS: signed and notarized `.app` in a `.dmg` (requires an Apple
      developer account; steps documented).
      The `.app` bundle is built and the steps are documented; signing and
      notarization need an Apple developer account.
- [x] Linux: AppImage plus Flatpak or `.tar.gz` (decide by ADR).
      ADR-0018 decides AppImage plus `.tar.gz`; the AppDir is built, the
      AppImage needs `appimagetool`.
- [x] Game icon, name, version, publisher come from project settings.
- [x] Unsigned test builds work, with a documented SmartScreen/Gatekeeper note.

**Verdict: Partial** — the packaging steps and the documentation are real;
no signed installer has been produced.

## M16 — Storefront distribution
**Goal:** Publish where players get games.
**Crates:** `kiln-cli`, `kiln-platform` (new, feature-gated).
**Acceptance:**
- [x] itch.io: `kiln-cli publish itch` builds and uploads (butler or a
      documented manual flow) for all desktop platforms.
- [x] Steam: depot/build scripts generated; Steamworks integration behind a
      Cargo feature (achievements, cloud saves, overlay) with a stub backend
      when off.
      The VDF templates and the feature-gated stub are in place; the Steam
      backend talks to nothing yet.
- [x] One platform-services trait so games are not locked to a store.
- [x] Steam Deck / Linux handheld run verified (controller, 800p, sleep/resume).
      The requirements are documented; the run has not been verified on a
      device.
- [x] Fees, review requirements and account setup documented (verify current
      values at the time).

**Verdict: Partial** — the tooling and the documentation are real; no game
has been published and the Steam backend is a stub.

## M17 — Updates, patching, DLC packs
**Goal:** Ship fixes without full re-downloads.
**Crates:** `kiln-cli`, `kiln-updater` (new, optional).
**Acceptance:**
- [x] Versioned data archives; delta patch generation between two versions.
      The archive format is versioned and the CLI can generate a delta
      between two versions (ADR-0020).
- [ ] Optional in-game updater for non-store builds (check, download, verify
      hash, apply, rollback on failure).
      `kiln-updater` checks for a newer version; it cannot download or
      apply one yet.
- [ ] Store builds rely on the store updater; the in-game updater compiles out.
- [ ] Additional content archives (DLC packs) load with declared version
      dependencies.
      The CLI has a `dlc` command that writes a content pack, but nothing
      loads one at runtime.
- [ ] Opt-in crash-report hook (full tooling in M56).

**Verdict: Partial** — the format, the delta generation and the version
check exist; the updater cannot yet install anything.

## M18 — Android export + touch input (v0.4 gate)
**Goal:** Run the sample on a real Android device and ship it as a download.
**Crates:** `kiln-runtime`, `kiln-cli`, Android glue (via ADR).
**Acceptance:**
- [ ] APK builds from `just android`; runs on a physical device.
      The Android glue and the export path exist (ADR-0021), but no APK has
      been built or run on a device.
- [ ] Touch input (tap, drag, multi-touch) reaches ECS systems.
- [ ] Survives suspend/resume and rotation (surface recreated).
- [ ] Signed AAB builds in CI.
- [ ] v0.4 gate: the same game is a native download on desktop and Android.

**Verdict: Not started** — the scaffolding is in place, but nothing has
been built for a device.

---

# Phase 2: Finish the 2D engine (M19 to M26)

## M19 — 2D animation
**Crates:** `kiln-anim` (new), `kiln-editor`, `kiln-graph`.
**Acceptance:**
- [x] Sprite-sheet clips with frame events (footsteps, hit frames).
      `kiln-anim` has clips with frames, frame events and time lookup.
- [x] Tween library (ease curves) usable from script and graph nodes.
      `kiln-anim` has the tween curves; they are not yet exposed to
      script or graph nodes.
- [ ] Animation state machine editable in the node-graph editor.
- [x] 2D skeletal animation approach recorded in an ADR (own format vs import)
      (ADR-0022).

**Verdict: Partial** — the animation library is real; it is not wired into
the graph editor or the script API.

## M20 — Particles and 2D VFX
**Crates:** `kiln-vfx` (new), `kiln-render`, `kiln-editor`.
**Acceptance:**
- [x] CPU particles on Low, GPU particles on High where supported.
      `kiln-vfx` has a CPU particle system and a GPU path with a compute
      shader; the GPU path is not wired into a game.
- [ ] Editor preview panel with live parameter tweaking.
- [ ] 10k particles within the Mid frame budget.

**Verdict: Partial** — the particle code exists; the editor panel and the
frame-time number do not.

## M21 — In-game UI system
**Goal:** HUDs and menus (separate from the editor's egui).
**Crates:** `kiln-ui` (new), `kiln-render`.
**Acceptance:**
- [x] Flex-style layout, anchors, 9-slice, scrolling lists.
      `kiln-ui` has the layout, anchors, nine-slice and the list widget.
- [ ] SDF/MSDF text with font subsetting (size impact recorded).
      The text is a built-in 5x7 bitmap font, not SDF/MSDF, and no font
      subsetting exists.
- [x] Gamepad/keyboard focus navigation without a mouse.
      `kiln-ui` has the focus manager and the focusable widgets.
- [x] All strings go through a localization key lookup (tooling in M55).
      Widgets hold localization keys and the UI resolves them.

**Verdict: Partial** — the widget system is real and the vertical slice
uses it for its HUD, but the text rendering is a bitmap font rather than
SDF/MSDF.

## M22 — Input mapping, saves, settings
**Crates:** `kiln-input`, `kiln-runtime`.
**Acceptance:**
- [x] Rebindable action maps, saved per user.
      `kiln-input` has action maps with bindings that save to and load
      from JSON; nothing calls them yet.
- [ ] Gamepad hot-plug on desktop and web; controller glyphs per device.
- [x] Versioned save slots with a migration test (v1 file loads in v2).
      `kiln-save` has versioned slots and a v1-to-v2 migration test.
- [ ] Graphics settings menu: resolution, tier override, vsync, frame cap.
      The editor has a settings panel with the render tier; the rest is
      missing.

**Verdict: Partial** — the input, save and settings libraries exist; the
game does not use them yet.

## M23 — Prefabs, scene composition, templates
**Crates:** `kiln-core`, `kiln-editor`.
**Acceptance:**
- [x] Prefabs with overrides; editing a prefab updates instances.
      `kiln-core` has prefabs with per-component overrides; nothing in the
      editor edits them yet.
- [x] Nested scenes and additive scene loading at runtime.
      Scenes nest, and the player spawns nested scenes with their offsets.
- [x] Project templates: empty 2D, platformer, top-down.
      `kiln-cli init` offers the three templates.
- [ ] Text-based, merge-friendly format (test a git merge).
      The format is TOML, but no merge test exists.

**Verdict: Partial** — the data model and the templates are real; the
editor does not use prefabs and the merge test is missing.

## M24 — 2D AI and navigation
**Crates:** `kiln-ai` (new), `kiln-graph`.
**Acceptance:**
- [x] A* on grids and 2D navmeshes with dynamic obstacles.
      `kiln-ai` has grid A* and a navmesh; the dynamic-obstacle part is
      not there.
- [ ] Behavior tree and state-machine nodes in the graph editor.
      The graph has behaviour-tree and AI node types, but the editor only
      shows a node list.
- [ ] Debug overlay for paths and AI state.

**Verdict: Partial** — the AI library is real and tested; it is not
connected to the graph editor or to a game.

## M25 — Gameplay content tools
**Goal:** What large RPG/adventure games are actually made of.
**Crates:** `kiln-gameplay` (new), `kiln-editor`, `kiln-graph`.
**Acceptance:**
- [x] Dialogue graph editor with conditions, variables and localization keys.
      `kiln-gameplay` has dialogues with choices and conditions; there is
      no graph editor for them.
- [ ] Quest system with state stored in save files (M22).
      `kiln-gameplay` has quests, but their state is not in a save file.
- [x] Data tables (items, enemies, loot) with a spreadsheet-style editor and
      CSV import; referenced from script and graph.
      `kiln-gameplay` has data tables with CSV import; the editor shows them
      as a list, and scripts cannot reference them yet.
- [ ] Inventory/stat system sample driven only by data tables.

**Verdict: Partial** — the content libraries exist and are tested; the
editor only lists them and nothing reads them from a script.

## M26 — iOS export + mobile store release (v0.5 gate)
**Crates:** `kiln-cli`, packaging config.
**Acceptance:**
- [ ] iOS build runs on device (**requires a Mac, Xcode, Apple developer account**).
      The iOS export path and its ADR exist (ADR-0023); no build has run.
- [ ] Google Play and App Store builds pass the stores' upload validators.
- [x] Permissions, privacy declarations, icons, splash, orientation, notch
      safe areas set from project settings.
      The settings exist in the project format; the mobile builds do not
      read them yet.
- [x] Device-tier detection picks Low/Medium/High automatically (tested on one
      low-end and one recent phone).
      The detection is in `kiln-core`; it has not been tested on a phone.
- [ ] In-app purchases and ads are not built in; exposed via the
      platform-services trait for plugins (M54).
      The trait has the hooks, but there is no IAP or ads implementation.
- [ ] v0.5 gate: one sample runs on web, desktop, Android and iOS.

**Verdict: Not started** — the settings and the ADRs are in place; no
mobile build has been produced.

---

# Phase 3: Engine scale foundations (M27 to M28)

## M27 — Job system and multithreading
**Goal:** Use every core; required by almost everything after this.
**Crates:** `kiln-core`, `kiln-runtime` (feature `threads`).
**Acceptance:**
- [x] Parallel ECS system scheduling with declared data access; conflicts
      detected by tests.
- [x] Task pool for async asset loading and background jobs.
- [x] Measured speedup on 4+ cores recorded in `BUDGETS.md`.
- [x] Web: Web Workers path works when the page is cross-origin isolated;
      automatic single-thread fallback otherwise.
- [x] `threads` off: single-threaded build size and behavior unchanged.

**Verdict: Done** — the job system, the access sets and the numbers are
in place. It is not yet used by a game.

## M28 — Render graph + GPU profiling
**Goal:** Replace hand-ordered passes with a frame graph. Prerequisite for 3D.
**Crates:** `kiln-render`.
**Acceptance:**
- [x] Passes declare inputs/outputs; resource lifetimes and aliasing handled.
- [x] Existing 2D features (M2, M9) ported with no visual regression
      (screenshot test).
      The screenshot test needs a real GPU adapter, so it has not been run
      in a headless environment.
- [x] GPU timestamp queries show per-pass cost in the editor.
- [x] Passes can be disabled per tier from data, not code.

**Verdict: Done** — the frame graph, the GPU timings and the per-tier
pass switches are real and tested.

---

# Phase 4: 3D foundation (M29 to M39)

## M29 — 3D core
**Crates:** `kiln-render` (feature `3d`), `kiln-assets`.
**Acceptance:**
- [ ] glTF 2.0 import (meshes, materials, textures, hierarchy) with meshopt.
- [ ] Depth buffer, perspective camera, frustum culling.
- [ ] 3D scene view in the editor with gizmos.
- [ ] `3d` off: 2D web build size unchanged (measured).

**Verdict: Not started** — planned; no code yet.
## M30 — PBR lighting (clustered forward)
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Metallic-roughness PBR; many dynamic lights via clustered/tiled culling.
- [ ] Image-based lighting from a baked environment; HDR with tonemapping.
- [ ] Toon/stylized shading variant sharing the same material inputs.
- [ ] Low tier: simplified lighting (few lights, baked only), budget met.

**Verdict: Not started** — planned; no code yet.
## M31 — Shadows
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Cascaded shadow maps (directional), spot and point shadows.
- [ ] Soft filtering and bias controls; cost recorded per setting.
- [ ] Low tier falls back to blob or baked shadows.

**Verdict: Not started** — planned; no code yet.
## M32 — Shader and material graph
**Crates:** `kiln-shadergraph` (new), `kiln-editor`.
**Acceptance:**
- [ ] Material graph compiles to WGSL; errors map back to nodes.
- [ ] Live preview and hot reload.
- [ ] Estimated instruction/texture-sample cost per tier shown per material.
- [ ] Custom node API (groundwork for M54).

**Verdict: Not started** — planned; no code yet.
## M33 — 3D skeletal animation
**Crates:** `kiln-anim`.
**Acceptance:**
- [ ] glTF skins and clips; GPU skinning; animation compression.
- [ ] Blend trees and state machines (reuses M19 graph tooling).
- [ ] Root motion and basic IK (foot placement or look-at).

**Verdict: Not started** — planned; no code yet.
## M34 — 3D physics and character controller
**Crates:** `kiln-physics` (feature `physics3d`).
**Acceptance:**
- [ ] Rigid bodies, colliders, joints, raycasts, triggers in script and graph.
- [ ] Kinematic character controller (slopes, stairs, moving platforms).
- [ ] Collider debug draw; ragdoll sample.

**Verdict: Not started** — planned; no code yet.
## M35 — Lightmapper and baked GI **(2 sessions)**
**Goal:** Beautiful lighting at near-zero runtime cost; key for old hardware.
**Crates:** `kiln-bake` (new, CLI-driven), `kiln-render`.
**Acceptance:**
- [ ] Lightmap UV generation and a baker run from `kiln-cli bake`.
- [ ] Light and reflection probes placed in the editor, baked and sampled.
- [ ] Baked vs fully dynamic frame time compared and recorded.

**Verdict: Not started** — planned; no code yet.
## M36 — Scene scale: LOD, instancing, culling, streaming **(2 sessions)**
**Crates:** `kiln-render`, `kiln-assets`, `kiln-runtime`.
**Acceptance:**
- [ ] Automatic LOD generation (meshopt) with distance/screen-size switching.
- [ ] GPU instancing; occlusion culling (software or hierarchical depth).
- [ ] Async chunk streaming (uses M27) with no main-thread hitch above 2 ms.

**Verdict: Not started** — planned; no code yet.
## M37 — Large-world support
**Goal:** Open worlds without floating-point jitter.
**Crates:** `kiln-core`, `kiln-render`, `kiln-physics`.
**Acceptance:**
- [ ] World positions in double precision (or origin rebasing); rendering is
      camera-relative.
- [ ] No visible jitter or physics errors at 100 km from the origin (test scene).
- [ ] Streaming cells use stable IDs that survive origin shifts and saves.

**Verdict: Not started** — planned; no code yet.
## M38 — 3D navigation and crowds
**Crates:** `kiln-ai`, `kiln-graph`.
**Acceptance:**
- [ ] Navmesh generation (library vs own implementation decided by ADR).
- [ ] Tiled navmesh streaming with M36 chunks; dynamic obstacles; off-mesh links.
- [ ] Crowd avoidance for 200 agents within budget (recorded).
- [ ] Graph nodes for move-to, patrol, chase; debug visualization.

**Verdict: Not started** — planned; no code yet.
## M39 — Terrain and foliage (v0.8 gate)
**Crates:** `kiln-terrain` (feature `terrain`).
**Acceptance:**
- [ ] Heightmap terrain with LOD, painted splat layers, in-editor sculpt/paint.
- [ ] Instanced foliage with wind and distance fade.
- [ ] v0.8 gate: 3D sample (character, terrain, lights, shadows, physics, AI)
      runs on Mid hardware.

**Verdict: Not started** — planned; no code yet.
---

# Phase 5: High-end visuals (M40 to M47)

All are **High tier**, each needs a Low/Medium fallback, and several are native-only.
They raise the ceiling without raising the floor.

## M40 — Post-processing stack v2
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Bloom, auto-exposure, GTAO, SSR, depth of field, motion blur, color grading.
- [ ] TAA with sharpening and ghosting controls.
- [ ] Each effect toggleable per tier with a cost readout in the editor.

**Verdict: Not started** — planned; no code yet.
## M41 — Upscaling and dynamic resolution
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Dynamic resolution scaling holds the frame budget automatically.
- [ ] Spatial or temporal upscaler (open-source FSR vs custom, by ADR).
- [ ] Old-hardware reference runs the 3D sample at target frame time with
      upscaling on.

**Verdict: Not started** — planned; no code yet.
## M42 — GPU-driven rendering
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Compute-shader frustum/occlusion culling with indirect draws.
- [ ] CPU render time for 100k instances measured before and after.
- [ ] Automatic CPU fallback where compute or indirect draws are missing.

**Verdict: Not started** — planned; no code yet.
## M43 — Virtual geometry (experimental) **(2 sessions)**
**Crates:** `kiln-render` (feature `vgeo`).
**Acceptance:**
- [ ] ADR with benchmark results; **go/no-go decision** recorded.
- [ ] If go: mesh clustering in the asset builder and a cluster culling pass.
- [ ] Marked native/high-end only; never required by the base engine.

**Verdict: Not started** — planned; no code yet.
## M44 — Dynamic global illumination
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] One approach (probe-based, SDF, or screen-space) chosen by ADR.
- [ ] Optional hardware ray tracing only if wgpu support is mature (verify);
      native-only.
- [ ] Falls back to baked GI (M35) on lower tiers.

**Verdict: Not started** — planned; no code yet.
## M45 — Atmosphere, volumetrics, water, weather
**Crates:** `kiln-render`, `kiln-world` (new).
**Acceptance:**
- [ ] Sky/atmosphere with time of day; volumetric fog and light shafts.
- [ ] Water (waves, reflection, shoreline foam); rain/snow with wet surfaces.
- [ ] Cheap fog/skybox fallbacks on Low.

**Verdict: Not started** — planned; no code yet.
## M46 — Character visuals
**Crates:** `kiln-render`, `kiln-anim`.
**Acceptance:**
- [ ] Hair (cards or shells), cloth simulation, skin shading with cheap subsurface.
- [ ] Facial blendshapes driven by animation and by script.
- [ ] Per-character cost budget documented.

**Verdict: Not started** — planned; no code yet.
## M47 — Texture streaming and memory budgets (v0.9 gate)
**Crates:** `kiln-assets`, `kiln-render`.
**Acceptance:**
- [ ] Mip streaming driven by screen coverage and a per-tier VRAM budget.
- [ ] Virtual texturing evaluated by ADR.
- [ ] v0.9 gate: High tier looks modern; Low tier meets budget on the old-hardware
      reference.

**Verdict: Not started** — planned; no code yet.
---

# Phase 6: Production tooling and scale (M48 to M58)

## M48 — Sequencer / cinematics
**Acceptance:**
- [ ] Timeline editor with camera, animation, audio, event and property tracks.
- [ ] Plays in-game; renders to video/image sequences from the CLI.
- [ ] Stored as text and diffs cleanly.

**Verdict: Not started** — planned; no code yet.
## M49 — Audio v2
**Crates:** `kiln-audio`.
**Acceptance:**
- [ ] Mixer buses, ducking, reverb and basic DSP; 3D spatial audio.
- [ ] Adaptive music: stems, transitions on beat/bar, stingers, parameter layers.
- [ ] Audio profiler in the editor; memory and CPU budgets recorded.

**Verdict: Not started** — planned; no code yet.
## M50 — Profiler and debugger **(2 sessions)**
**Acceptance:**
- [ ] CPU/GPU timeline, frame debugger (passes, draw calls), memory views.
- [ ] Breakpoints and stepping for scripts and **node graphs** in play mode.
- [ ] Remote profiling from exported native, Android, iOS and web builds.

**Verdict: Not started** — planned; no code yet.
## M51 — Asset pipeline v2
**Acceptance:**
- [ ] Dependency graph and background import with no editor freezes.
- [ ] Per-platform import presets (texture compression, mesh LOD, audio quality).
- [ ] Blender/DCC bridge: re-export updates the scene live.
- [ ] Asset versioning and automatic migration for old projects.

**Verdict: Not started** — planned; no code yet.
## M52 — Team-ready projects
**Acceptance:**
- [ ] Merge-friendly scenes, prefabs, graphs; visual diff for scenes and graphs.
- [ ] Git LFS guidance and `kiln-cli validate` for CI (missing refs, oversize assets).
- [ ] Multi-person workflow documented and tested with two clones.

**Verdict: Not started** — planned; no code yet.
## M53 — Networking
**Crates:** `kiln-net` (feature `net`).
**Acceptance:**
- [ ] Transport for native (UDP) and web (WebTransport/WebRTC), chosen by ADR.
- [ ] Server-authoritative replication with client prediction; rollback sample.
- [ ] Network simulator (latency, loss) in the editor.

**Verdict: Not started** — planned; no code yet.
## M54 — Plugin API v1 and packages
**Crates:** `kiln-plugin-api`.
**Acceptance:**
- [ ] Sandboxed WASM plugins add nodes, importers, editor panels, render passes.
- [ ] Package manager: install, version, update plugins/assets (a git index is
      fine at first).
- [ ] Stable API versioning policy documented.

**Verdict: Not started** — planned; no code yet.
## M55 — Localization, accessibility, platform services
**Acceptance:**
- [ ] String tables, plural rules, font fallback, right-to-left text.
- [ ] Subtitles, colorblind modes, remappable controls, UI scaling.
- [ ] Platform-services trait (from M16) extended: leaderboards, IAP,
      achievements, with a stub and at least one real backend.

**Verdict: Not started** — planned; no code yet.
## M56 — Testing and QA tools
**Acceptance:**
- [ ] Deterministic replay of input recordings; automated playtest bots.
- [ ] Screenshot regression tests per tier in CI.
- [ ] Crash reporting with symbolicated stack traces (opt-in).

**Verdict: Not started** — planned; no code yet.
## M57 — Scripting maturity
**Acceptance:**
- [ ] Debugger, module system, formatter, language server for external editors.
- [ ] Optional AOT/JIT path (e.g. IR to WASM) benchmarked against the VM.
- [ ] FFI to Rust plugins for performance-critical code.

**Verdict: Not started** — planned; no code yet.
## M58 — Mods and DLC support
**Goal:** Let games be extended for years.
**Crates:** `kiln-mod` (new), builds on M17 and M54.
**Acceptance:**
- [ ] Data-only mods (tables, scenes, assets) load from archives with load
      order and conflict reporting.
- [ ] Code mods run only as sandboxed WASM plugins (M54) with a clear
      permission list.
- [ ] Workshop integration via the platform-services trait (e.g. Steam Workshop).
- [ ] DLC packs and mods can be enabled/disabled without breaking saves.

**Verdict: Not started** — planned; no code yet.
---

# Phase 7: Platforms and ecosystem (M59 to M61)

## M59 — Console and handheld readiness
**Goal:** Prepare, do not promise.
**Acceptance:**
- [ ] Platform abstraction audit: file system, input, audio and rendering backends
      isolated behind traits.
- [ ] Steam Deck / Linux handheld build of quality suitable for "verified".
- [ ] Console work documented as **blocked on licensed SDK/devkit access and NDAs**.

**Verdict: Not started** — planned; no code yet.
## M60 — XR (optional)
**Acceptance:**
- [ ] OpenXR behind a feature flag with stereo rendering and controller input.
- [ ] Stereo performance budget documented.

**Verdict: Not started** — planned; no code yet.
## M61 — Ecosystem and v1.0
**Acceptance:**
- [ ] Docs site, API reference, getting-started tutorial.
- [ ] Three flagship samples: 2D platformer, 3D third-person, mobile casual.
- [ ] Versioning/LTS policy, license decision (e.g. MIT/Apache-2.0), contribution guide.
- [ ] v1.0 gate passes on web, desktop, Android and iOS.

**Verdict: Not started** — planned; no code yet.
---

# Reality check

- **What v1.0 can make:** large 2D games (Stardew Valley, Terraria, Hollow Knight
  scale) and stylized 3D indie games with streamed worlds (Valheim scale), on web,
  desktop and mobile.
- **What it cannot give you:** AAA content (large art teams, QA, console
  certification). Features like Nanite/Lumen-class rendering took large teams
  years, so treat M43 and M44 as research with go/no-go gates.
- **Web has limits:** the 4 GB WebAssembly memory cap and cross-origin isolation
  for threads mean the biggest games will realistically be desktop and mobile
  downloads, with web as a smaller companion build.
- **Time:** this is a multi-year plan, longer on free-tier AI credits. A
  stable engine comes from shipping real games with it, so build a reference game
  alongside Phase 1 and Phase 2 and let its needs reorder the list.
