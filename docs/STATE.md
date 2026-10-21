# Kiln Session Handoff

**Current milestone:** The 2D gate (G-1 … G-6) is **complete**. G-6 "Editor
truth" landed in this session: the editor edits real scene data, the scene
view draws the scene with the engine's own renderer, and play-in-editor runs
the real script. M29 (3D) may now start, but the 2D engine still has gaps
listed under **Known issues**.

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
| Editor scene view | **Now real**: the GPU sprite renderer draws the scene into the panel (E-9) |
| Play-in-editor | **Now real**: the scene script runs through the VM (E-10) |
| Undo / save / asset hub | **Now real**: undo/redo over scene data, saves write the file (E-1) |
| `kiln-player` | Loads projects, runs scripts, HUD, save/load; no menu/loader UI |
| `kiln-web` | Isolation/shell; not a full game loop export |
| `UiRenderer` | Produces quads; the player draws a HUD with it, the editor uses egui |
| Store / installers / iOS ship | CLI/docs scaffolding; not end-to-end proven |
| Most `samples/` | Console `println` smokes, except the vertical slice |

### Band summary

| Band | Verdict |
| --- | --- |
| M0–M7 | Partial — core loop + colored batch + editor shell; editor viewport is now the engine's |
| M8–M14 | Partial — tilemaps/textures real; post + web still thin |
| M15–M21 | Stub-heavy→Partial — libs ahead of product wiring; UI renders but unwired |
| M22–M28 | Partial — AI lib solid; M27–M28 infra real; product gaps remain |

---

## Done (this session)

- **The build was broken on `master` and is fixed.** The previous commit called
  `Program::validate_jump_targets()` from the script compiler but never
  wrote it, so nothing in the workspace compiled.
  - `kiln-ir`: added `ProgramError` (`DuplicateLabel`, `InvalidJumpTarget`),
    implemented `validate_jump_targets()`, and made `define_label` reject
    duplicate names instead of silently overwriting the first definition.
  - The graph and script compilers propagate the new error.
- **The 2D gate item G-6 "Editor truth" is done.**
  - **One source of truth.** The editor keeps the scene as data
    (`kiln-editor::scene_ops::Scene`). The inspector, the entity list and the
    save path all work on that data, and the runtime world is rebuilt from it
    after every edit, so the editor, the renderer and the saved file cannot
    disagree.
  - **Undo and redo** (Edit menu, Ctrl+Z / Ctrl+Y) restore whole scenes from
    `kiln-editor::history`, with unit tests.
  - **The scene view draws the real thing.** The panel writes the rectangle of
    its canvas and the renderer draws the scene into it with the engine's
    sprite renderer, the player's camera convention (world Y grows upwards)
    and one world unit per pixel. The duplicated egui sprite painting is gone,
    and the HUD is drawn with the engine's text renderer, so the editor and
    the player show the same pixels.
  - **Play-in-editor** runs the scene's script through
    `kiln_runtime::SliceSession` against the *edited* scene, and stopping the
    session keeps unsaved edits.
  - **Saves write the file.** "Save Scene" writes the project file, and "Save
    Project" writes the edited scene into the project first. Before this,
    inspector edits were silently dropped.
  - **The remaining editor stubs are implemented**: add/delete entity, add
    and remove components with a per-component menu, prefab create /
    instantiate / rename, scene create / delete / start-scene / nested scenes,
    asset import, export dialog, about dialog, documentation link, console
    clear, and a real frame time for the FPS counter.
- **The GPU screenshot test now skips on headless machines**, so the test
  suite passes on a build server without a GPU.

---

## In progress

None.

---

## Next steps (2D hardening, before or alongside M29)

The 2D **gate** is done; these are the remaining 2D **features** that the
roadmap still lists as open (see `docs/TODO_LIST.md`, Phase 2):

1. **M19 2D animation** — sprite-sheet clips with frame events and tweens
   exist in `kiln-anim`; the animation state machine is not in the graph
   editor yet, and there is no ADR for skeletal animation.
2. **M20 particles** — CPU and GPU particle systems exist in `kiln-vfx`; the
   editor has no live preview panel and there is no 10k particle benchmark.
3. **M21 UI** — flex layout, anchors, 9-slice, focus navigation and
   localization keys exist in `kiln-ui`; scrolling lists, SDF text and font
   subsetting do not.
4. **M22 input / saves** — action maps exist in `kiln-input`; per-user action
   maps, versioned save slots and the graphics settings menu are open.
5. **M23 prefabs** — prefabs and nested scenes work in the editor; editing a
   prefab does not yet update its instances, and there are no project
   templates.
6. **M24 2D AI** — A* on grids and navmeshes exist in `kiln-ai`; the
   behaviour-tree nodes and the path debug overlay are not in the editor.
7. **M25 gameplay tools** — dialogue, quests and data tables exist in
   `kiln-gameplay`; the dialogue graph editor, CSV import and the
   data-table-driven sample are open.
8. **Post-processing** — the chain runs, but it still needs per-effect
   spatial regression tests and manual on-hardware verification.

---

## Known issues

- Post effects (bloom/outline/fog) still need **manual** on-hardware visual
  verification with `B` / `O` / `F`; the GPU test only checks non-black
  output. Per-effect spatial regression tests are still to do.
- The editor's scene view draws the scene with the engine's renderer, but the
  **grid and the panel chrome are still egui**, so the viewport is not a
  separate render target yet.
- The editor does not draw **tilemaps** in its scene view; only the player
  does (`kiln-player` spawns them with their tileset textures).
- The script API is a fixed builtin list; a script cannot yet spawn entities
  or read arbitrary world state.
- The editor has no **native file dialog**, so importing an asset asks for a
  path instead of opening a browser.
- The `audio` sample needs ALSA headers, so it is excluded from the local
  test run; the `kiln-render` screenshot test needs a real GPU adapter and
  now skips without one.
- `kiln-editor/src/app.rs` and `kiln-cli/src/main.rs` are large; allowed
  under ADR-0026. The new editor logic lives in `scene_ops.rs` and
  `history.rs` to keep it testable.

**Blocked:** None — the 2D gate is done, so M29 (3D) is unblocked.
