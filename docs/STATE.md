# Kiln Session Handoff

**Current milestone:** M0–M28 claimed complete in prior sessions — **honesty audit
says Partial overall**. Do **not** start M29 (3D) until the 2D gate below is met.

---

## Audit (M0–M28 honesty pass) — 2026-10-19

**Verdict:** Promising 2D **architecture / libraries**, not yet a solid shippable
2D engine. Many milestones are real crates with tests; several “done” items are
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
| `UiRenderer` | Returns empty `Vec`s — widgets don’t draw |
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

- **F6 — `kiln-player` project/scene/archive loading** (M14): **done** ->
  `loader.rs` loads `.kar` archives and project directories (`kiln.toml`/`kiln.ron`);
  `spawn.rs` spawns `Transform`/`Sprite` into `GameWorld`, incl. nested scenes with
  offsets and a cycle guard; `main.rs` wires load -> spawn -> winit render loop.
  `cargo check -p kiln-player --all-targets`, clippy, and 6 unit tests pass.

---

## Next steps (2D-first gate — before M29)

Ordered by impact. One milestone per session when work resumes:

1. **Textured sprites** — atlas, UV rects, batch by texture
2. **Real tilemap** — tileset, layers, chunking (not checkerboard sprites)
3. **Asset → GPU** — packed images become bindable textures in samples/editor
4. **Editor truth** — GPU scene view; inspector + save/load; undo; PIE runs VM
5. **Standalone player** — load `.kar` + scene + run loop (finish M14 for real) — **done (F6)**
6. **VM maturity** — f32/f64 (and strings or host transforms) for usable gameplay
7. **Vertical slice sample** — textured art + script/graph + save + on-screen UI
8. **UI draw path** — replace `UiRenderer` stubs (or explicitly defer UI)
9. **Post** — implement a Low-safe path or stop claiming Medium+ looks
10. Reconcile `docs/ROADMAP.md` / `ROADMAP_FULL` checkboxes with this audit

**Do not start M29 (3D)** until items 1–5 are honestly done and a 2D sample
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
