# Kiln Session Handoff

**Current milestone:** the 2D gate (G-1 … G-6) is **complete**. The
honesty audit below is kept for the record, but the items it listed as
stubs have been implemented since. **M29 (3D) may start.**

**Do not start M29 (3D) until the 2D gate below is met** — the gate is
met, so M29 may start. Everything the audit called "partial" that is
still partial is listed under **Known issues**.

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
| Post (M9) | **Now real on the GPU**: bloom, fog, outline; each effect has a test of its own |
| Editor scene view | **Now real**: the engine draws the scene into the panel, like the player |
| Play-in-editor | **Now real**: the editor runs the same runtime session as the player |
| Inspector edits | **Now real**: edits are saved to disk, with undo and redo |
| Undo / save / asset hub | **Now real**: undo and redo on the scene, save writes to disk, assets import into the cache |
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

The project did not compile: `kiln-script` called
`Program::validate_jump_targets`, which was never added, so the workspace
could not be built at all. That is fixed, and the rest of the session
made the 2D gate honest.

### Build and test health

- **The workspace builds again.** `Program::validate_jump_targets` is
  implemented in `kiln-ir` and covered by a test.
- **Clippy is clean** with `-D warnings` across the workspace, and
  `cargo fmt --check` passes.
- **The audio sample is its own workspace.** It needs a system audio
  backend, and as a workspace member that backend was forced onto every
  other crate, so the workspace only built on machines with ALSA
  headers. The main workspace now builds anywhere; `just check` still
  checks the audio sample.
- **The GPU tests skip instead of failing** when the machine has no
  adapter, so headless CI runs them.

### Editor truth (the 2D gate)

- **The scene view is drawn by the engine** (`kiln-editor/src/scene_view.rs`).
  The panel's rectangle becomes a viewport, the world is projected with
  the same camera the player uses, and the HUD is drawn over it in
  screen space. The editor no longer paints the scene with egui, so it
  and the player show the same pixels.
- **The project's textures are uploaded to the GPU** and bound to the
  sprites, so the editor shows the same art as the player.
- **The editor plays the scene with the real runtime**: the same
  `kiln_runtime::SliceSession` the player runs, with the HUD lines
  built by the runtime so both hosts show the same information.
- **The inspector writes to disk** and the editor has **undo and redo**
  over the scene.
- **Every editor panel that was a stub now works**: add and remove
  entities and their components, create and instantiate prefabs, create
  and nest scenes, export and package the project, import the assets,
  clear the console, and the about and documentation entries.

### Post-processing

- **Each post effect has a test of its own** that renders a scene of
  known colours and compares the result with the same scene rendered
  without the effect, so an effect that does nothing fails. The tests
  were checked by breaking the shader: each one fails when its own
  effect is removed.

---

## In progress

None.

---

## Next steps

The 2D gate is done, so the next milestone is **M29 (3D)**. Before
starting it, read `ROADMAP_FULL_M14_PLUS.md` for the M29 plan and
`docs/ARCHITECTURE.md` for the layers.

Known gaps that are worth picking up on the way (none of them block
M29):

- The frame graph still names the post passes without running the post
  chain, so the editor's scene view has no post effects. The player
  does, through `Renderer::apply_post`.
- `kiln-web` is a shell, not a full game loop export.
- The iOS and Android capability paths return fixed values.
- The player has no menu or loader UI, and the store and installer
  flows are scaffolding.

---

## Known issues

- Carry forward prior M14–M28 gaps (player has no menu/loader UI, web
  Workers ping-only, iOS device testing needs Mac, etc.)
- The frame graph's post passes are still stubs: the editor's scene view
  has no post effects, while the player has them.
- The effect toggles (B/O/F) are only verified by hand; the automatic
  tests cover the effects themselves.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed
  under ADR-0026
- The script API is a fixed builtin list; a script cannot yet spawn
  entities or read arbitrary world state
- The iOS and Android device capability paths return fixed values
- The audio sample needs a system audio backend, so it is built on its
  own: `cargo check --manifest-path samples/audio/Cargo.toml`

**Blocked:** None — the 2D gate is complete, so M29 may start.
