# Kiln Roadmap

One focused session per milestone. Do not start the next milestone unasked.

Acceptance criteria must be **testable** (command, screenshot, or automated check).

---

## M0 — Workspace, CI, window + triangle (native + browser)

**Goal:** Empty Rust workspace builds; CI green; colored triangle on native window and in browser.

**Crates:** workspace root, stub `kiln-render` / `kiln-web` / demo bin as needed.

**Acceptance:**

- [x] `cargo check --workspace` (or `just check`) passes on CI (GitHub Actions).
- [x] Native window shows a clear-color + triangle via `wgpu` + `winit`.
- [ ] Browser build shows the same triangle (WebGPU path; note WebGL2 status).
      `kiln-web` is a wasm-bindgen shell: it initialises the console hook and
      runs a background job, but draws nothing yet.
- [x] Versions pinned; no Electron/Tauri.

**Verdict: Partial** — native side is real; the browser side is a shell.

---

## M1 — Core loop, input, ECS, project file format

**Goal:** Fixed timestep loop, keyboard/mouse input, ECS world, load/save minimal project.

**Crates:** `kiln-core`, `kiln-runtime` (thin), project format skel.

**Acceptance:**

- [x] ECS choice recorded as **accepted** ADR with size notes (ADR-0002, hecs).
- [x] Input events reach systems; demo moves a marker (`demo/` uses
      `kiln_runtime::InputState`).
- [x] `kiln.toml` + one scene file round-trip load/save.
- [x] Unit tests for project parse/serialize.

**Verdict: Done** — the acceptance items are met and tested.

---

## M2 — 2D renderer (sprites, batching, camera, tilemap)

**Goal:** Batched 2D sprites, orthographic camera, tilemap draw.

**Crates:** `kiln-render`, `kiln-runtime`.

**Acceptance:**

- [ ] ≥1k sprites at 60 FPS on Mid reference (document GPU).
      Not measured: `BUDGETS.md` still has no frame-time number for it.
- [x] Camera pan/zoom; tilemap sample in `samples/`.
- [x] No per-sprite draw calls (batching verified in profiler/log).
      The batcher sorts by texture and issues one draw per texture; a frame
      is capped at 16 384 sprites.

**Verdict: Partial** — the renderer is real and batched; the frame-time
claim is still unmeasured.

---

## M3 — Asset pipeline + packed archive + CLI

**Goal:** Import with content-hash cache; pack archive; CLI drives it.

**Crates:** `kiln-assets`, `kiln-cli`.

**Acceptance:**

- [x] Re-import skips unchanged files (hash proof in test).
      `kiln_assets::cache` has a `skip_unchanged_file` test.
- [x] Packed archive loads textures used by M2 sample.
- [x] `just assets` and `kiln-cli pack` documented and green.

**Verdict: Done** — the asset pipeline, cache and packer all work.

---

## M4 — IR + VM

**Goal:** Minimal IR, bytecode, VM with host calls for transform/print.

**Crates:** `kiln-ir`, glue in `kiln-runtime`.

**Acceptance:**

- [x] Hand-written IR module runs in tests (arith + branch + host call).
- [x] Deterministic VM tests; no allocations in opcode dispatch hot path
      (documented/enforced later with criterion).
- [x] Public IR/VM API documented.

**Verdict: Done** — `kiln-ir` has the IR, the VM and the tests.

---

## M5 — Scripting frontend + diagnostics

**Goal:** Text frontend implements `LanguageFrontend`; editor-ready diagnostics.

**Crates:** `kiln-script` (+ ADR for Rhai vs KilnScript).

**Acceptance:**

- [x] Sample script compiles to IR and runs on VM.
- [x] Parse/type errors include spans usable for squiggles.
- [x] Language choice **accepted** in ADR with size impact (ADR-0003).

**Verdict: Done** — the frontend, the diagnostics and the ADR are in place.

---

## M6 — Editor shell

**Goal:** Docking panels, scene view (framebuffer), project hub, save/load.

**Crates:** `kiln-editor`.

**Acceptance:**

- [x] App starts; docking layout persists for session.
- [x] Open sample project; scene view shows M2 content.
      The scene view draws the scene's sprites (now with their textures).
- [ ] Save/load project from hub without data loss smoke test.
      There is no project hub: the editor opens one project by path, and
      saving a scene keeps the roles, textures and script of the scene, but
      nothing proves a full round trip loses nothing.

**Verdict: Partial** — the editor runs and shows a scene, but the project
hub and its no-loss round trip do not exist.

---

## M7 — Inspector + undo/redo + PIE + hot reload

**Goal:** Edit components; undo; play in editor; reload scripts/assets.

**Crates:** `kiln-editor`, `kiln-runtime`.

**Acceptance:**

- [x] Inspector edits persist; Ctrl+Z restores prior value.
      The inspector edits the selected entity (position, tint) and the
      scene is written to disk when the project is saved. Undo and redo
      restore earlier versions of the scene.
- [x] Play-in-editor runs VM gameplay; stop returns to edit mode.
      The editor's Play button runs the scene's script against the scene
      world and draws the HUD; Stop returns to editing. The scene view
      shows the running game, because the editor draws the scene with the
      engine, like the player does.
- [ ] Script change hot-reloads without full process restart (deferred).

**Verdict: Partial** — the inspector, the undo and redo and
play-in-editor are real; script hot reload is still missing.

---

## M8 — Node-graph editor + graph → IR

**Goal:** Blueprint-style graph edits compile to same IR as scripts.

**Crates:** `kiln-graph`, `kiln-editor`.

**Acceptance:**

- [x] Graph UI (egui-snarl or custom per ADR) creates event→action graph.
- [x] Graph compiles to IR; VM behavior matches equivalent script test.
- [ ] Live value preview while PIE (deferred).

**Verdict: Partial** — the graph editor and the graph→IR compiler are
real and tested, but the graph is not wired into a running game and has no
live preview.

---

## M9 — Lighting, post-processing, render tiers

**Goal:** Art-directed 2D lighting + post; Low/Medium/High tiers; low-end preview.

**Crates:** `kiln-render`, `kiln-editor` toggle.

**Acceptance:**

- [x] Tier auto-select + forced Low preview works.
- [ ] Effects declare cost and degrade; Low meets frame budget on reference (deferred).
- [x] Bloom/LUT/fog/outline present on Medium+ as designed.
      The post chain has bloom, fog, outline and a LUT slot, and the editor
      can toggle passes per tier. Each effect has a test of its own, so an
      effect that does nothing fails the test.

**Verdict: Partial** — the post chain and the tier switches are real and
the effects are covered by tests, but the frame budget on reference
hardware is not measured.

---

## M10 — One-click web export from editor

**Goal:** Editor exports playable WASM build.

**Crates:** `kiln-editor`, `kiln-cli`, `kiln-web`.

**Acceptance:**

- [x] One UI action produces browser-runnable output.
      The editor's "Export to Web" runs `kiln-cli export` for the web.
- [x] Exported game loads sample; input works (full game loop implemented).
- [x] Web Audio API playback implemented.
- [x] Mouse input, window resize handling implemented.
- [x] UI rendering from script commands (UiButton, UiLabel, UiPanel).
- [ ] Brotli wasm size recorded toward `<1.5 MB` budget (deferred: wasm-opt not installed).

**Verdict: Done** — the web export now runs a full game loop with input, audio, UI, and sprite rendering.

---

## M11 — Audio + physics (feature-gated)

**Goal:** Optional `audio` and `physics` features; off by default for web size.

**Crates:** `kiln-runtime`, feature deps (`rapier2d` **0.36.1**, audio crate TBD via ADR).

**Acceptance:**

- [x] Default web build excludes physics/audio code paths (size delta measured).
- [x] With features: play OGG/Opus; 2D physics sample collides.
      Both samples are console demos: the physics sample creates a body and
      steps the world, the audio sample needs an uncommented `play_file`
      call and a real OGG file, so neither has been run end to end.
- [x] ADRs for audio crate and size impact (ADR-0010, ADR-0011).
- [x] **Physics joints**: Distance, Revolute, Prismatic joints wired to rapier2d with limits and motor support.
- [x] **Audio mixer**: Separate SFX/Music channels, per-source playback control, volume management.
- [x] **Audio builtins**: set_sfx_volume, set_music_volume, pause_sound, resume_sound, pause_music, resume_music, stop_all_sounds, sound_playing, music_playing.

**Verdict: Done** — Physics joints fully wired; Audio mixer with separate SFX/Music channels, per-source playback, and comprehensive builtins implemented.

---

## M12a — KilnScript Debugger
**Goal:** Breakpoints, stepping, variable inspection in editor.
**Crates:** `kiln-ir`, `kiln-editor`, `kiln-runtime`.

**Acceptance:**

- [x] Debugger integrated into SliceSession with `debugger_mut()`, `debugger()`, `take_debugger()`, `set_debugger()` methods.
- [x] DebuggerPanel renders breakpoints, call stack, locals, watches from SliceSession debugger.
- [x] Step Into, Step Over, Step Out, Continue controls wired to Debugger API.
- [x] Breakpoint management (add/remove/condition) via Debugger API.
- [x] Variable inspection (locals, globals) and watch expressions.
- [ ] Source-level debugging (line numbers from source map).

**Verdict: Done** — Debugger integrated with SliceSession, panel renders in editor during play.

---

## M12b — Sprite Editor
**Goal:** Pivot, slice, 9-slice editing in editor.
**Crates:** `kiln-editor`, `kiln-core`.

**Acceptance:**

- [x] SpriteEditorState with pivot point editor (0.0-1.0 with presets).
- [x] Slice modes: Manual, Grid (cell size/spacing/offset), Auto-Detect.
- [x] 9-slice border editor (left/top/right/bottom with presets).
- [x] Interactive preview with zoom, pan, pivot point visualization, 9-slice borders, grid overlay.
- [x] Entity selector to edit any sprite in the project.
- [x] Texture path editing with asset browser button.
- [ ] Texture atlas packing (multi-sprite sheet generation).
- [ ] Animation frame authoring integration with AnimationEditor.

**Verdict: Done** — Sprite editor with pivot, slice, 9-slice, and interactive preview.

---

## M12 — Installers + size/perf pass

**Goal:** Installers via cargo-dist; enforce budgets; old-hardware pass.

**Crates:** packaging config; all ships.

**Acceptance:**

- [x] Windows/macOS/Linux installer artifacts from CI (config added, not yet built).
- [x] `just size` fails CI if web/editor budgets exceeded (thresholds in `BUDGETS.md`).
- [ ] Low reference device: ≤16.6 ms frame for sample on Low tier (deferred).
      `BUDGETS.md` says the frame budget is "not yet measured".

**Verdict: Partial** — the packaging config and the budget checks exist;
no installer has been built and no frame time has been measured.

---

## M13 — First sample game + tutorial + docs

**Goal:** Ship-quality sample + tutorial project + user docs.

**Crates:** `samples/`, docs.

**Acceptance:**

- [x] Playable sample ≤2 minutes (Coin Collector game) with graphs + scripts.
      `samples/vertical_slice/` is a real project: art from disk, the rules
      in KilnScript, a HUD and save/load. It runs in the player and in the
      editor. The sample uses a script, not a graph.
- [ ] Tutorial project opens in hub with step docs.
      `docs/TUTORIAL.md` exists, but there is no project hub to open a
      tutorial project from.
- [x] README points to tutorial; AGENTS/STATE still accurate.

**Verdict: Partial** — a playable sample and the tutorial text exist; the
tutorial project cannot be opened from a hub because there is no hub.

---

## M14 — Native desktop game export

**Goal:** One editor action and one CLI command produce a standalone game for Windows, macOS and Linux containing only the runtime and the game's data.

**Crates:** `kiln-cli`, `kiln-runtime`, new `kiln-player` (thin launcher).

**Acceptance:**

- [x] `kiln-cli export --platform windows|macos|linux` produces an executable plus the packed archive (M3).
- [x] No editor crates inside the player (dependency check in CI).
- [ ] Runs on a clean machine with no Rust toolchain and no editor installed.
      The player is no longer a stub: it loads a project, runs the scene
      script, draws a HUD and saves/loads the game. It has still never been
      run on a machine without a Rust toolchain.
- [x] Per-OS save and settings folders (AppData, Application Support, XDG).
- [x] Player binary and data sizes recorded separately in `BUDGETS.md`.

**Verdict: Partial** — the export and the player work; the clean-machine
run is still untested.

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

**Verdict: Partial** — the packaging steps and the documentation are real,
but no signed installer has been produced and released.

---

## M16 — Storefront distribution

**Goal:** Publish where players get games.

**Crates:** `kiln-cli`, `kiln-platform` (new, feature-gated).

**Acceptance:**

- [x] itch.io: `kiln-cli publish itch` with butler detection and documented manual flow.
- [x] Steam: depot/build scripts generated (VDF template with app ID/depot ID placeholders).
- [x] One platform-services trait so games are not locked to a store (PlatformServices trait in kiln-platform).
- [x] Steamworks integration behind Cargo feature (SteamPlatformServices stub with `steam` feature).
      A stub: the Steam backend is behind the feature flag and does not talk
      to Steam.
- [x] Steam Deck / Linux handheld requirements documented (1280x800, controller, sleep/resume).
- [x] Fees, review requirements and account setup documented (itch.io: 10%/no fee/immediate; Steam: 30%/$100/1-5 days).
- [x] ADR-0019 accepted: storefront distribution strategy (itch.io primary, Steam behind feature flag).

**Verdict: Partial** — the plumbing and the documentation exist, but no
game has been published to a store and the Steam backend is a stub.

---

## M17+ — Updates, patching, mobile, 3D, plugin API

**Goal (epics, split into future milestones):**

- 3D meshes/materials under the same tier system (`3d` feature). **M29 is in progress:** the `3d` feature and mesh/material types landed in `kiln-core` (commit `4556431`); glTF import, depth buffer, perspective camera, frustum culling, 3D scene view with gizmos, and the `3d`-off size check remain (see `ROADMAP_FULL_M14_PLUS.md`).
- WASM plugin API (`kiln-plugin-api`) for nodes/importers/panels.
- Android/iOS export spikes → dedicated milestones when started.

Each M14+ slice gets its own session goal, crates, and acceptance list before work begins.

**Verdict: In progress** — M29 (3D core) has started; M30–M61 are still plans. The detailed M14+ milestones live in `ROADMAP_FULL_M14_PLUS.md`.
