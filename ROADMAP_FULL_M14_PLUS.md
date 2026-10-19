# Kiln Roadmap: everything after M13 (single file)

**Status:** Prior sessions claimed M0–M28; **2026-10-19 honesty audit**
(`docs/STATE.md`) rates overall **Partial** — do **not** start M29 (3D) until
the 2D-first gate (textured sprites, tilemap, editor/player wiring) is met.
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

- [ ] **Native backends:** wgpu uses Vulkan/DX12/Metal and falls back to
      OpenGL/GLES on old GPUs. Test the fallback on real old hardware.
      (Verify current wgpu backend support for your pinned version.)
- [ ] **Assets:** one archive format with two loaders: memory-mapped file for
      native, HTTP streaming for web.
- [ ] **Audio (M11):** device selection, hot-plug and sample-rate changes work
      on desktop.
- [ ] **Budgets (M12):** `BUDGETS.md` has a native player size budget (suggested
      start: under 15 MB executable without assets, adjust after measuring) and
      a startup budget (under 2 s to first frame).
- [ ] **Input/settings:** window options exist in project settings (fullscreen,
      borderless, resolution, vsync, high-DPI, multi-monitor).
- [ ] **M13 sample:** the "N minutes" placeholder is replaced with a real
      number; all M0 to M13 checkboxes verified on a clean machine.
- [ ] `docs/STATE.md` rewritten to mark M0 to M13 done and M14 as next.

---

# Phase 1: Downloadable games and mobile (M14 to M18)

## M14 — Native desktop game export
**Goal:** One editor action and one CLI command produce a standalone game for
Windows, macOS and Linux containing only the runtime and the game's data.
**Crates:** `kiln-cli`, `kiln-runtime`, new `kiln-player` (thin launcher).
**Acceptance:**
- [ ] `kiln-cli export --platform windows|macos|linux` produces an executable
      plus the packed archive (M3).
- [ ] No editor crates inside the player (dependency check in CI).
- [ ] Runs on a clean machine with no Rust toolchain and no editor installed.
- [ ] Per-OS save and settings folders (AppData, Application Support, XDG).
- [ ] Player binary and data sizes recorded separately in `BUDGETS.md`.

## M15 — Packaging, signing, installers for games
**Goal:** Players install games without scary OS warnings.
**Crates:** `kiln-cli`, packaging config.
**Acceptance:**
- [ ] Windows: installer or portable zip; code-signing step in CI (certificate
      as a CI secret; documented how to obtain one).
- [ ] macOS: signed and notarized `.app` in a `.dmg` (requires an Apple
      developer account; steps documented).
- [ ] Linux: AppImage plus Flatpak or `.tar.gz` (decide by ADR).
- [ ] Game icon, name, version, publisher come from project settings.
- [ ] Unsigned test builds work, with a documented SmartScreen/Gatekeeper note.

## M16 — Storefront distribution
**Goal:** Publish where players get games.
**Crates:** `kiln-cli`, `kiln-platform` (new, feature-gated).
**Acceptance:**
- [ ] itch.io: `kiln-cli publish itch` builds and uploads (butler or a
      documented manual flow) for all desktop platforms.
- [ ] Steam: depot/build scripts generated; Steamworks integration behind a
      Cargo feature (achievements, cloud saves, overlay) with a stub backend
      when off.
- [ ] One platform-services trait so games are not locked to a store.
- [ ] Steam Deck / Linux handheld run verified (controller, 800p, sleep/resume).
- [ ] Fees, review requirements and account setup documented (verify current
      values at the time).

## M17 — Updates, patching, DLC packs
**Goal:** Ship fixes without full re-downloads.
**Crates:** `kiln-cli`, `kiln-updater` (new, optional).
**Acceptance:**
- [ ] Versioned data archives; delta patch generation between two versions.
- [ ] Optional in-game updater for non-store builds (check, download, verify
      hash, apply, rollback on failure).
- [ ] Store builds rely on the store updater; the in-game updater compiles out.
- [ ] Additional content archives (DLC packs) load with declared version
      dependencies.
- [ ] Opt-in crash-report hook (full tooling in M56).

## M18 — Android export + touch input (v0.4 gate)
**Goal:** Run the sample on a real Android device and ship it as a download.
**Crates:** `kiln-runtime`, `kiln-cli`, Android glue (via ADR).
**Acceptance:**
- [ ] APK builds from `just android`; runs on a physical device.
- [ ] Touch input (tap, drag, multi-touch) reaches ECS systems.
- [ ] Survives suspend/resume and rotation (surface recreated).
- [ ] Signed AAB builds in CI.
- [ ] v0.4 gate: the same game is a native download on desktop and Android.

---

# Phase 2: Finish the 2D engine (M19 to M26)

## M19 — 2D animation
**Crates:** `kiln-anim` (new), `kiln-editor`, `kiln-graph`.
**Acceptance:**
- [ ] Sprite-sheet clips with frame events (footsteps, hit frames).
- [ ] Tween library (ease curves) usable from script and graph nodes.
- [ ] Animation state machine editable in the node-graph editor.
- [ ] 2D skeletal animation approach recorded in an ADR (own format vs import).

## M20 — Particles and 2D VFX
**Crates:** `kiln-vfx` (new), `kiln-render`, `kiln-editor`.
**Acceptance:**
- [ ] CPU particles on Low, GPU particles on High where supported.
- [ ] Editor preview panel with live parameter tweaking.
- [ ] 10k particles within the Mid frame budget.

## M21 — In-game UI system
**Goal:** HUDs and menus (separate from the editor's egui).
**Crates:** `kiln-ui` (new), `kiln-render`.
**Acceptance:**
- [ ] Flex-style layout, anchors, 9-slice, scrolling lists.
- [ ] SDF/MSDF text with font subsetting (size impact recorded).
- [ ] Gamepad/keyboard focus navigation without a mouse.
- [ ] All strings go through a localization key lookup (tooling in M55).

## M22 — Input mapping, saves, settings
**Crates:** `kiln-input`, `kiln-runtime`.
**Acceptance:**
- [ ] Rebindable action maps, saved per user.
- [ ] Gamepad hot-plug on desktop and web; controller glyphs per device.
- [ ] Versioned save slots with a migration test (v1 file loads in v2).
- [ ] Graphics settings menu: resolution, tier override, vsync, frame cap.

## M23 — Prefabs, scene composition, templates
**Crates:** `kiln-core`, `kiln-editor`.
**Acceptance:**
- [ ] Prefabs with overrides; editing a prefab updates instances.
- [ ] Nested scenes and additive scene loading at runtime.
- [ ] Project templates: empty 2D, platformer, top-down.
- [ ] Text-based, merge-friendly format (test a git merge).

## M24 — 2D AI and navigation
**Crates:** `kiln-ai` (new), `kiln-graph`.
**Acceptance:**
- [ ] A* on grids and 2D navmeshes with dynamic obstacles.
- [ ] Behavior tree and state-machine nodes in the graph editor.
- [ ] Debug overlay for paths and AI state.

## M25 — Gameplay content tools
**Goal:** What large RPG/adventure games are actually made of.
**Crates:** `kiln-gameplay` (new), `kiln-editor`, `kiln-graph`.
**Acceptance:**
- [ ] Dialogue graph editor with conditions, variables and localization keys.
- [ ] Quest system with state stored in save files (M22).
- [ ] Data tables (items, enemies, loot) with a spreadsheet-style editor and
      CSV import; referenced from script and graph.
- [ ] Inventory/stat system sample driven only by data tables.

## M26 — iOS export + mobile store release (v0.5 gate)
**Crates:** `kiln-cli`, packaging config.
**Acceptance:**
- [ ] iOS build runs on device (**requires a Mac, Xcode, Apple developer account**).
- [ ] Google Play and App Store builds pass the stores' upload validators.
- [ ] Permissions, privacy declarations, icons, splash, orientation, notch
      safe areas set from project settings.
- [ ] Device-tier detection picks Low/Medium/High automatically (tested on one
      low-end and one recent phone).
- [ ] In-app purchases and ads are not built in; exposed via the
      platform-services trait for plugins (M54).
- [ ] v0.5 gate: one sample runs on web, desktop, Android and iOS.

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

## M28 — Render graph + GPU profiling
**Goal:** Replace hand-ordered passes with a frame graph. Prerequisite for 3D.
**Crates:** `kiln-render`.
**Acceptance:**
- [x] Passes declare inputs/outputs; resource lifetimes and aliasing handled.
- [x] Existing 2D features (M2, M9) ported with no visual regression
      (screenshot test).
- [x] GPU timestamp queries show per-pass cost in the editor.
- [x] Passes can be disabled per tier from data, not code.

---

# Phase 4: 3D foundation (M29 to M39)

## M29 — 3D core
**Crates:** `kiln-render` (feature `3d`), `kiln-assets`.
**Acceptance:**
- [ ] glTF 2.0 import (meshes, materials, textures, hierarchy) with meshopt.
- [ ] Depth buffer, perspective camera, frustum culling.
- [ ] 3D scene view in the editor with gizmos.
- [ ] `3d` off: 2D web build size unchanged (measured).

## M30 — PBR lighting (clustered forward)
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Metallic-roughness PBR; many dynamic lights via clustered/tiled culling.
- [ ] Image-based lighting from a baked environment; HDR with tonemapping.
- [ ] Toon/stylized shading variant sharing the same material inputs.
- [ ] Low tier: simplified lighting (few lights, baked only), budget met.

## M31 — Shadows
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Cascaded shadow maps (directional), spot and point shadows.
- [ ] Soft filtering and bias controls; cost recorded per setting.
- [ ] Low tier falls back to blob or baked shadows.

## M32 — Shader and material graph
**Crates:** `kiln-shadergraph` (new), `kiln-editor`.
**Acceptance:**
- [ ] Material graph compiles to WGSL; errors map back to nodes.
- [ ] Live preview and hot reload.
- [ ] Estimated instruction/texture-sample cost per tier shown per material.
- [ ] Custom node API (groundwork for M54).

## M33 — 3D skeletal animation
**Crates:** `kiln-anim`.
**Acceptance:**
- [ ] glTF skins and clips; GPU skinning; animation compression.
- [ ] Blend trees and state machines (reuses M19 graph tooling).
- [ ] Root motion and basic IK (foot placement or look-at).

## M34 — 3D physics and character controller
**Crates:** `kiln-physics` (feature `physics3d`).
**Acceptance:**
- [ ] Rigid bodies, colliders, joints, raycasts, triggers in script and graph.
- [ ] Kinematic character controller (slopes, stairs, moving platforms).
- [ ] Collider debug draw; ragdoll sample.

## M35 — Lightmapper and baked GI **(2 sessions)**
**Goal:** Beautiful lighting at near-zero runtime cost; key for old hardware.
**Crates:** `kiln-bake` (new, CLI-driven), `kiln-render`.
**Acceptance:**
- [ ] Lightmap UV generation and a baker run from `kiln-cli bake`.
- [ ] Light and reflection probes placed in the editor, baked and sampled.
- [ ] Baked vs fully dynamic frame time compared and recorded.

## M36 — Scene scale: LOD, instancing, culling, streaming **(2 sessions)**
**Crates:** `kiln-render`, `kiln-assets`, `kiln-runtime`.
**Acceptance:**
- [ ] Automatic LOD generation (meshopt) with distance/screen-size switching.
- [ ] GPU instancing; occlusion culling (software or hierarchical depth).
- [ ] Async chunk streaming (uses M27) with no main-thread hitch above 2 ms.

## M37 — Large-world support
**Goal:** Open worlds without floating-point jitter.
**Crates:** `kiln-core`, `kiln-render`, `kiln-physics`.
**Acceptance:**
- [ ] World positions in double precision (or origin rebasing); rendering is
      camera-relative.
- [ ] No visible jitter or physics errors at 100 km from the origin (test scene).
- [ ] Streaming cells use stable IDs that survive origin shifts and saves.

## M38 — 3D navigation and crowds
**Crates:** `kiln-ai`, `kiln-graph`.
**Acceptance:**
- [ ] Navmesh generation (library vs own implementation decided by ADR).
- [ ] Tiled navmesh streaming with M36 chunks; dynamic obstacles; off-mesh links.
- [ ] Crowd avoidance for 200 agents within budget (recorded).
- [ ] Graph nodes for move-to, patrol, chase; debug visualization.

## M39 — Terrain and foliage (v0.8 gate)
**Crates:** `kiln-terrain` (feature `terrain`).
**Acceptance:**
- [ ] Heightmap terrain with LOD, painted splat layers, in-editor sculpt/paint.
- [ ] Instanced foliage with wind and distance fade.
- [ ] v0.8 gate: 3D sample (character, terrain, lights, shadows, physics, AI)
      runs on Mid hardware.

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

## M41 — Upscaling and dynamic resolution
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Dynamic resolution scaling holds the frame budget automatically.
- [ ] Spatial or temporal upscaler (open-source FSR vs custom, by ADR).
- [ ] Old-hardware reference runs the 3D sample at target frame time with
      upscaling on.

## M42 — GPU-driven rendering
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] Compute-shader frustum/occlusion culling with indirect draws.
- [ ] CPU render time for 100k instances measured before and after.
- [ ] Automatic CPU fallback where compute or indirect draws are missing.

## M43 — Virtual geometry (experimental) **(2 sessions)**
**Crates:** `kiln-render` (feature `vgeo`).
**Acceptance:**
- [ ] ADR with benchmark results; **go/no-go decision** recorded.
- [ ] If go: mesh clustering in the asset builder and a cluster culling pass.
- [ ] Marked native/high-end only; never required by the base engine.

## M44 — Dynamic global illumination
**Crates:** `kiln-render`.
**Acceptance:**
- [ ] One approach (probe-based, SDF, or screen-space) chosen by ADR.
- [ ] Optional hardware ray tracing only if wgpu support is mature (verify);
      native-only.
- [ ] Falls back to baked GI (M35) on lower tiers.

## M45 — Atmosphere, volumetrics, water, weather
**Crates:** `kiln-render`, `kiln-world` (new).
**Acceptance:**
- [ ] Sky/atmosphere with time of day; volumetric fog and light shafts.
- [ ] Water (waves, reflection, shoreline foam); rain/snow with wet surfaces.
- [ ] Cheap fog/skybox fallbacks on Low.

## M46 — Character visuals
**Crates:** `kiln-render`, `kiln-anim`.
**Acceptance:**
- [ ] Hair (cards or shells), cloth simulation, skin shading with cheap subsurface.
- [ ] Facial blendshapes driven by animation and by script.
- [ ] Per-character cost budget documented.

## M47 — Texture streaming and memory budgets (v0.9 gate)
**Crates:** `kiln-assets`, `kiln-render`.
**Acceptance:**
- [ ] Mip streaming driven by screen coverage and a per-tier VRAM budget.
- [ ] Virtual texturing evaluated by ADR.
- [ ] v0.9 gate: High tier looks modern; Low tier meets budget on the old-hardware
      reference.

---

# Phase 6: Production tooling and scale (M48 to M58)

## M48 — Sequencer / cinematics
**Acceptance:**
- [ ] Timeline editor with camera, animation, audio, event and property tracks.
- [ ] Plays in-game; renders to video/image sequences from the CLI.
- [ ] Stored as text and diffs cleanly.

## M49 — Audio v2
**Crates:** `kiln-audio`.
**Acceptance:**
- [ ] Mixer buses, ducking, reverb and basic DSP; 3D spatial audio.
- [ ] Adaptive music: stems, transitions on beat/bar, stingers, parameter layers.
- [ ] Audio profiler in the editor; memory and CPU budgets recorded.

## M50 — Profiler and debugger **(2 sessions)**
**Acceptance:**
- [ ] CPU/GPU timeline, frame debugger (passes, draw calls), memory views.
- [ ] Breakpoints and stepping for scripts and **node graphs** in play mode.
- [ ] Remote profiling from exported native, Android, iOS and web builds.

## M51 — Asset pipeline v2
**Acceptance:**
- [ ] Dependency graph and background import with no editor freezes.
- [ ] Per-platform import presets (texture compression, mesh LOD, audio quality).
- [ ] Blender/DCC bridge: re-export updates the scene live.
- [ ] Asset versioning and automatic migration for old projects.

## M52 — Team-ready projects
**Acceptance:**
- [ ] Merge-friendly scenes, prefabs, graphs; visual diff for scenes and graphs.
- [ ] Git LFS guidance and `kiln-cli validate` for CI (missing refs, oversize assets).
- [ ] Multi-person workflow documented and tested with two clones.

## M53 — Networking
**Crates:** `kiln-net` (feature `net`).
**Acceptance:**
- [ ] Transport for native (UDP) and web (WebTransport/WebRTC), chosen by ADR.
- [ ] Server-authoritative replication with client prediction; rollback sample.
- [ ] Network simulator (latency, loss) in the editor.

## M54 — Plugin API v1 and packages
**Crates:** `kiln-plugin-api`.
**Acceptance:**
- [ ] Sandboxed WASM plugins add nodes, importers, editor panels, render passes.
- [ ] Package manager: install, version, update plugins/assets (a git index is
      fine at first).
- [ ] Stable API versioning policy documented.

## M55 — Localization, accessibility, platform services
**Acceptance:**
- [ ] String tables, plural rules, font fallback, right-to-left text.
- [ ] Subtitles, colorblind modes, remappable controls, UI scaling.
- [ ] Platform-services trait (from M16) extended: leaderboards, IAP,
      achievements, with a stub and at least one real backend.

## M56 — Testing and QA tools
**Acceptance:**
- [ ] Deterministic replay of input recordings; automated playtest bots.
- [ ] Screenshot regression tests per tier in CI.
- [ ] Crash reporting with symbolicated stack traces (opt-in).

## M57 — Scripting maturity
**Acceptance:**
- [ ] Debugger, module system, formatter, language server for external editors.
- [ ] Optional AOT/JIT path (e.g. IR to WASM) benchmarked against the VM.
- [ ] FFI to Rust plugins for performance-critical code.

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

---

# Phase 7: Platforms and ecosystem (M59 to M61)

## M59 — Console and handheld readiness
**Goal:** Prepare, do not promise.
**Acceptance:**
- [ ] Platform abstraction audit: file system, input, audio and rendering backends
      isolated behind traits.
- [ ] Steam Deck / Linux handheld build of quality suitable for "verified".
- [ ] Console work documented as **blocked on licensed SDK/devkit access and NDAs**.

## M60 — XR (optional)
**Acceptance:**
- [ ] OpenXR behind a feature flag with stereo rendering and controller input.
- [ ] Stereo performance budget documented.

## M61 — Ecosystem and v1.0
**Acceptance:**
- [ ] Docs site, API reference, getting-started tutorial.
- [ ] Three flagship samples: 2D platformer, 3D third-person, mobile casual.
- [ ] Versioning/LTS policy, license decision (e.g. MIT/Apache-2.0), contribution guide.
- [ ] v1.0 gate passes on web, desktop, Android and iOS.

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
