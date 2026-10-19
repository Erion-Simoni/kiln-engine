# Kiln Roadmap

One focused session per milestone. Do not start the next milestone unasked.

Acceptance criteria must be **testable** (command, screenshot, or automated check).

---

## M0 — Workspace, CI, window + triangle (native + browser)

**Goal:** Empty Rust workspace builds; CI green; colored triangle on native window and in browser.

**Crates:** workspace root, stub `kiln-render` / `kiln-web` / demo bin as needed.

**Acceptance:**

- [ ] `cargo check --workspace` (or `just check`) passes on CI (GitHub Actions).
- [ ] Native window shows a clear-color + triangle via `wgpu` + `winit`.
- [ ] Browser build shows the same triangle (WebGPU path; note WebGL2 status).
- [ ] Versions pinned; no Electron/Tauri.

---

## M1 — Core loop, input, ECS, project file format

**Goal:** Fixed timestep loop, keyboard/mouse input, ECS world, load/save minimal project.

**Crates:** `kiln-core`, `kiln-runtime` (thin), project format skel.

**Acceptance:**

- [ ] ECS choice recorded as **accepted** ADR with size notes.
- [ ] Input events reach systems; demo moves a marker.
- [ ] `kiln.toml` + one scene file round-trip load/save.
- [ ] Unit tests for project parse/serialize.

---

## M2 — 2D renderer (sprites, batching, camera, tilemap)

**Goal:** Batched 2D sprites, orthographic camera, tilemap draw.

**Crates:** `kiln-render`, `kiln-runtime`.

**Acceptance:**

- [ ] ≥1k sprites at 60 FPS on Mid reference (document GPU).
- [ ] Camera pan/zoom; tilemap sample in `samples/`.
- [ ] No per-sprite draw calls (batching verified in profiler/log).

---

## M3 — Asset pipeline + packed archive + CLI

**Goal:** Import with content-hash cache; pack archive; CLI drives it.

**Crates:** `kiln-assets`, `kiln-cli`.

**Acceptance:**

- [ ] Re-import skips unchanged files (hash proof in test).
- [ ] Packed archive loads textures used by M2 sample.
- [ ] `just assets` and `kiln-cli pack` documented and green.

---

## M4 — IR + VM

**Goal:** Minimal IR, bytecode, VM with host calls for transform/print.

**Crates:** `kiln-ir`, glue in `kiln-runtime`.

**Acceptance:**

- [ ] Hand-written IR module runs in tests (arith + branch + host call).
- [ ] Deterministic VM tests; no allocations in opcode dispatch hot path (documented/enforced later with criterion).
- [ ] Public IR/VM API documented.

---

## M5 — Scripting frontend + diagnostics

**Goal:** Text frontend implements `LanguageFrontend`; editor-ready diagnostics.

**Crates:** `kiln-script` (+ ADR for Rhai vs KilnScript).

**Acceptance:**

- [ ] Sample script compiles to IR and runs on VM.
- [ ] Parse/type errors include spans usable for squiggles.
- [ ] Language choice **accepted** in ADR with size impact.

---

## M6 — Editor shell

**Goal:** Docking panels, scene view (framebuffer), project hub, save/load.

**Crates:** `kiln-editor`.

**Acceptance:**

- [ ] App starts; docking layout persists for session.
- [ ] Open sample project; scene view shows M2 content.
- [ ] Save/load project from hub without data loss smoke test.

---

## M7 — Inspector + undo/redo + PIE + hot reload

**Goal:** Edit components; undo; play in editor; reload scripts/assets.

**Crates:** `kiln-editor`, `kiln-runtime`.

**Acceptance:**

- [x] Inspector edits persist; Ctrl+Z restores prior value (deferred: undo/redo)
- [x] Play-in-editor runs VM gameplay; stop returns to edit mode.
- [ ] Script change hot-reloads without full process restart (deferred).

---

## M8 — Node-graph editor + graph → IR

**Goal:** Blueprint-style graph edits compile to same IR as scripts.

**Crates:** `kiln-graph`, `kiln-editor`.

**Acceptance:**

- [x] Graph UI (egui-snarl or custom per ADR) creates event→action graph.
- [x] Graph compiles to IR; VM behavior matches equivalent script test.
- [ ] Live value preview while PIE (deferred).

---

## M9 — Lighting, post-processing, render tiers

**Goal:** Art-directed 2D lighting + post; Low/Medium/High tiers; low-end preview.

**Crates:** `kiln-render`, `kiln-editor` toggle.

**Acceptance:**

- [x] Tier auto-select + forced Low preview works.
- [ ] Effects declare cost and degrade; Low meets frame budget on reference (deferred).
- [ ] Bloom/LUT/fog/outline present on Medium+ as designed (deferred).

---

## M10 — One-click web export from editor

**Goal:** Editor exports playable WASM build.

**Crates:** `kiln-editor`, `kiln-cli`, `kiln-web`.

**Acceptance:**

- [x] One UI action produces browser-runnable output.
- [ ] Exported game loads sample; input works (deferred: full game loop).
- [ ] Brotli wasm size recorded toward `<1.5 MB` budget (deferred: wasm-opt not installed).

---

## M11 — Audio + physics (feature-gated)

**Goal:** Optional `audio` and `physics` features; off by default for web size.

**Crates:** `kiln-runtime`, feature deps (`rapier2d` **0.36.1**, audio crate TBD via ADR).

**Acceptance:**

- [x] Default web build excludes physics/audio code paths (size delta measured).
- [x] With features: play OGG/Opus; 2D physics sample collides.
- [x] ADRs for audio crate and size impact.

---

## M12 — Installers + size/perf pass

**Goal:** Installers via cargo-dist; enforce budgets; old-hardware pass.

**Crates:** packaging config; all ships.

**Acceptance:**

- [x] Windows/macOS/Linux installer artifacts from CI (config added, not yet built).
- [x] `just size` fails CI if web/editor budgets exceeded (thresholds in `BUDGETS.md`).
- [ ] Low reference device: ≤16.6 ms frame for sample on Low tier (deferred).

---

## M13 — First sample game + tutorial + docs

**Goal:** Ship-quality sample + tutorial project + user docs.

**Crates:** `samples/`, docs.

**Acceptance:**

- [x] Playable sample ≤2 minutes (Coin Collector game) with graphs + scripts.
- [x] Tutorial project opens in hub with step docs.
- [x] README points to tutorial; AGENTS/STATE still accurate.

---

## M14 — Native desktop game export

**Goal:** One editor action and one CLI command produce a standalone game for Windows, macOS and Linux containing only the runtime and the game's data.

**Crates:** `kiln-cli`, `kiln-runtime`, new `kiln-player` (thin launcher).

**Acceptance:**

- [x] `kiln-cli export --platform windows|macos|linux` produces an executable plus the packed archive (M3).
- [x] No editor crates inside the player (dependency check in CI).
- [ ] Runs on a clean machine with no Rust toolchain and no editor installed (deferred: player is stub).
- [x] Per-OS save and settings folders (AppData, Application Support, XDG).
- [x] Player binary and data sizes recorded separately in `BUDGETS.md`.

---

## M15 — Packaging, signing, installers for games

**Goal:** Players install games without scary OS warnings.

**Crates:** `kiln-cli`, packaging config.

**Acceptance:**

- [x] Windows: ZIP archive created via PowerShell (MSI installer deferred to cargo-dist CI).
- [x] macOS: .app bundle with Info.plist (code-signing and notarization documented in PACKAGING.md).
- [x] Linux: AppDir structure created (AppImage requires appimagetool; documented in PACKAGING.md).
- [x] Game icon, name, version, publisher fields added to Project settings.
- [x] Code-signing and notarization documented in PACKAGING.md (Windows, macOS).
- [x] Unsigned build warnings (SmartScreen, Gatekeeper) documented in PACKAGING.md.
- [x] ADR-0018 accepted: Linux packaging format (AppImage + tar.gz, Flatpak deferred).

---

## M16 — Storefront distribution

**Goal:** Publish where players get games.

**Crates:** `kiln-cli`, `kiln-platform` (new, feature-gated).

**Acceptance:**

- [x] itch.io: `kiln-cli publish itch` with butler detection and documented manual flow.
- [x] Steam: depot/build scripts generated (VDF template with app ID/depot ID placeholders).
- [x] One platform-services trait so games are not locked to a store (PlatformServices trait in kiln-platform).
- [x] Steamworks integration behind Cargo feature (SteamPlatformServices stub with `steam` feature).
- [x] Steam Deck / Linux handheld requirements documented (1280x800, controller, sleep/resume).
- [x] Fees, review requirements and account setup documented (itch.io: 10%/no fee/immediate; Steam: 30%/$100/1-5 days).
- [x] ADR-0019 accepted: storefront distribution strategy (itch.io primary, Steam behind feature flag).

---

## M17+ — Updates, patching, mobile, 3D, plugin API

**Goal (epics, split into future milestones):**

- 3D meshes/materials under same tier system (`3d` feature).
- WASM plugin API (`kiln-plugin-api`) for nodes/importers/panels.
- Android/iOS export spikes → dedicated milestones when started.

Each M14+ slice gets its own session goal, crates, and acceptance list before work begins.
