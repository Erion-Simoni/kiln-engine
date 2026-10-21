# Kiln Session Handoff

**Current milestone:** M0–M28 claimed complete in prior sessions — the
honesty audit says **Partial overall**. Do **not** start M29 (3D) until the 2D
gate below is met.
**G-1, G-2, G-3 and G-4 are done** — the vertical slice now runs from both
the standalone player and the editor. **G-5 is next.**

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
| Post (M9) | `PostPipeline::apply` runs; still needs manual visual verification |
| Editor scene view | egui painted rects, **not** the engine framebuffer (E-9) |
| Play-in-editor | **Now runs the real game loop**; still no GPU viewport (E-9/E-10) |
| Undo / save / asset hub | TODOs in `kiln-editor` (E-1) |
| `kiln-player` | Loads projects, runs scripts, HUD, save/load; no menu/loader UI |
| `kiln-web` | Isolation/shell; not a full game loop export |
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

- **G-4: Vertical slice sample** — `samples/vertical_slice/` is a real Kiln
  project, not a console smoke:
  - **Art from disk**: `art/hero.png`, `art/coin.png` and `art/ground.png`
    are PNG files the player decodes and uploads to the GPU; the editor
    draws the same files with `egui`.
  - **Script logic that runs**: `scripts/main.ks` holds the game rules
    (move, collect, score). `kiln-runtime::slice` runs it once per frame: the
    script reads the frame's facts and the runtime applies the commands it
    emits to the world. The script decides the score — a test proves the
    engine does not award points on its own.
  - **Save and load**: the game state is a serialisable struct written to a
    `kiln-save` slot (F5 saves, F9 loads in the player).
  - **On-screen UI**: the player draws a HUD (score, status, controls) in
    screen space with the built-in bitmap font; the editor draws the same
    HUD over the scene view.
  - **Both hosts**: the standalone player
    (`cargo run -p kiln-player -- --archive samples/vertical_slice`) and the
    editor (open the project, press Play) run the same loop, because
    `kiln-runtime::scene` is the single scene spawner for both.
- **Engine work the sample needed**:
  - `kiln-ir`: `HostCall` carries an argument count, so a call consumes only
    its own arguments and calls compose; `GetLocal`/`SetLocal` add local
    variables.
  - `kiln-script`: `let` bindings, assignment, block scoping, unary minus,
    line comments, and a builtin registry so a game can expose host
    functions to its scripts.
  - `kiln-core`: a scene can name its script; entities can declare a
    gameplay role; `Project::default_scene_id` picks the start scene.
  - `kiln-runtime`: the `slice` game loop and the `scene` spawner.
  - `kiln-player`: runs the scene script, HUD, save/load, project-sized
    window.
  - `kiln-editor`: Play runs the scene script, the scene view shows the
    textures and the HUD, and Open Project takes a path.
  - 5 integration tests load the real sample project and check the art, the
    script, the gameplay and the save round trip.

---

## In progress

None.

---

## Next steps (2D-first gate — before M29)

These are tracked as checkboxes in `docs/TODO_LIST.md` **Phase 0.5 (G-1 … G-6)**.
Ordered by impact. One item per session; commit after each.

1. **G-1: Asset → GPU** — ✅ done
2. **G-2: Real tilemap** — ✅ done
3. **G-3: VM maturity** — ✅ done
4. **G-4: Vertical slice sample** — ✅ done this session
5. **G-5: Reconcile ROADMAP checkboxes** with this audit
6. **G-6: Editor truth** — GPU scene view; inspector + save/load; undo; PIE
   viewport (E-1 / E-9 / E-10)

**Do not start M29 (3D)** until G-5 and G-6 are honestly done.

---

## Known issues

- Carry forward prior M14–M28 gaps (player has no menu/loader UI, web
  Workers ping-only, iOS device testing needs Mac, etc.)
- Post effects (bloom/outline/fog) still need **manual** on-hardware visual
  verification with `B` / `O` / `F`; the GPU test only checks non-black
  output. Per-effect spatial regression tests are still to do.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed
  under ADR-0026
- The editor scene view is still egui-painted (E-9): the game runs there,
  but the viewport is not the engine framebuffer
- The editor's HUD is egui text, the player's is a sprite-batch HUD; they
  show the same state but not the same pixels
- The script API is a fixed builtin list; a script cannot yet spawn entities
  or read arbitrary world state
- The `audio` sample needs ALSA headers, so it is excluded from the local
  test run; `kiln-render`'s screenshot test needs a real GPU adapter

**Blocked:** None — blocked only by product choice: finish 2D before 3D.
