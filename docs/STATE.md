# Kiln Session Handoff

**Current milestone:** M0–M28 claimed complete in prior sessions — **honesty audit
says Partial overall**. Do **not** start M29 (3D) until the 2D gate below is met.

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
| Textured sprites / atlas / UV | Missing — color-only shader |
| Tilemap | Sample of colored sprites, not a tilemap module |
| Post (M9) | `PostPipeline::apply` TODO; graph schedules clear stubs |
| Editor scene view | egui painted rects, not engine framebuffer |
| Play-in-editor | Bool toggle; no VM/runtime PIE |
| Undo / save / asset hub | TODOs in `kiln-editor` |
| `kiln-player` | Loads `.kar` then exits (M14 TODO) |
| `kiln-web` | Isolation/shell; not a full game loop export |
| `UiRenderer` | Returns empty `Vec`s — widgets don't draw |
| Store / installers / iOS ship | CLI/docs scaffolding; not end-to-end proven |
| Most `samples/` | Console `println` smokes, not playable vertical slices |

### Band summary

| Band | Verdict |
| --- | --- |
| M0–M7 | Partial — core loop + colored batch + editor shell; no textured 2D / real PIE |
| M8–M14 | Partial — graph/IR solid-ish; post/player/web stub-heavy |
| M15–M21 | Stub-heavy→Partial — libs ahead of product wiring |
| M22–M28 | Partial — AI lib solid; M27–M28 infra real; product gaps remain |

---

## Done (this session)

- **M18-1: Android export + touch input** — audited current implementation: APK build command exists in export command via cargo-apk. Missing: touch input implementation, physical device testing, suspend/resume handling, rotation handling, signed AAB in CI, v0.4 gate verification. Marked as partial in TODO_LIST.md.
- **M17-1: Updates, patching, DLC packs** — audited current implementation: versioned archives (pack_with_version), delta patch generation (generate_patch with qbsdiff), and DLC pack CLI command exist. Missing: in-game updater (check/download/verify/apply/rollback), store updater integration, compile-out logic, crash-report hook. Marked as partial in TODO_LIST.md.
- **M16-1: Storefront distribution** — audited current implementation: CLI stubs for itch.io and Steam depot generation exist in main.rs; kiln-platform crate has trait and stub implementation. Missing: actual Steamworks SDK integration, achievements/cloud saves/overlay, Steam Deck testing, fees/docs. Marked as partial in TODO_LIST.md.
- **M15-1: Packaging, signing, installers** — audited current implementation: basic packaging stubs exist in export command (zip for Windows, .app for macOS, AppDir for Linux). Missing: no package.rs module, no code signing, no certificate documentation, no notarization, no ADR for Linux format, no project settings integration, no unsigned build testing, no SmartScreen/Gatekeeper docs. Marked as stub in TODO_LIST.md.
- **R0-6: M13 sample completion** — "N minutes" placeholder already replaced with "≤2 minutes" in ROADMAP.md per prior ADR work. Clean machine verification requires physical access to a machine without Rust toolchain (deferred).
- **R0-5: Window settings in project settings** — added `monitor` field to `WindowSettings` for multi-monitor support. All other window settings (fullscreen, borderless, resolution via width/height, vsync, high-DPI, resizable) already existed. Tests pass.
- **R0-3: Audio device management** — documented current M11 implementation (rodio high-level
  API, default device only). Device selection, hot-plug detection, sample-rate changes
  deferred to M49 audio v2 per ADR-0014 (sufficient for basic 2D games).
- **R0-2: Asset archive dual loaders** — implemented memory-mapped loader for native
  (`memmap2` crate, zero-copy access) and HTTP streaming loader for web (WASM,
  range requests via `WebArchiveLoader`). Archive format unchanged (KILNARV1/V2).
  All 14 tests pass. ADR-0013 updated to accepted.
- **R0-1: Native backends verification** — documented wgpu 22.1 backend support in `BUDGETS.md`:
  Vulkan, Metal, DX12, GL/WebGL2, WebGPU with fallback order and environment variables.
  Old hardware testing deferred (requires physical device).
- Created `docs/TODO_LIST.md` with 90+ stub/TODO items organized by milestone (R0-M28).
- Prior session work (from STATE.md):
  - ADR-0026 + `AGENTS.md`: AI may break the 400-line soft rule
  - Honesty audit recorded here; **M29 deferred** until 2D gate
  - **M9 Post (F5)** — real GPU post chain in `kiln-render`
  - **UI draw path (F1/F2/F3)** — `kiln-ui` bitmap font and quad emission
  - **Textured sprites** — atlas builder, UV rects, batch-by-texture sample
  - **F6** — `kiln-player` project/scene/archive loading
  - Clippy clean workspace-wide

---

## In progress

None

---

## Next steps (2D-first gate — before M29)

These are now tracked as checkboxes in `docs/TODO_LIST.md` **Phase 0.5 (G-1 … G-6)**.
Ordered by impact. One item per session; commit after each.

1. **G-1: Asset → GPU** — decode PNG to RGBA8, register GPU textures, wire scene
   sprites to texture paths, load in player/editor
2. **G-2: Real tilemap** — tileset, layers, chunking, culling, collision flags
3. **G-3: VM maturity** — string values (`Value::Str`), string constants and
   host-call arguments, script/graph frontends
4. **G-4: Vertical slice sample** — textured art + script/graph + save + on-screen UI
5. **G-5: Reconcile ROADMAP checkboxes** with this audit
6. **G-6: Editor truth** — GPU scene view; inspector + save/load; undo; PIE runs VM
   (see E-1 / E-9 / E-10)

Already done (no TODO entry needed): textured sprites, standalone player (F6),
UI draw path (F1/F2/F3). Still needs manual verification: post effects.

**Do not start M29 (3D)** until G-1 … G-6 are honestly done and a 2D sample
ships from editor/player without console-only demos.

---

## Known issues

- Carry forward prior M14–M28 gaps (player TODO, web Workers ping-only,
  iOS device testing needs Mac, etc.)
- Post effects (bloom/outline/fog) still need **manual** on-hardware visual
  verification with `B` / `O` / `F`; the GPU test only checks non-black output.
  Per-effect spatial regression tests are still to do.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed under ADR-0026

**Blocked:** None — blocked only by product choice: finish 2D before 3D.
