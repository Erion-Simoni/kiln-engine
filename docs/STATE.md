# Kiln Session Handoff

**Current milestone:** M0–M28 claimed complete in prior sessions — **honesty audit
says Partial overall**. Do **not** start M29 (3D) until the 2D gate below is met.
**G-1, G-2 and G-3 (Asset → GPU, Real tilemap, VM maturity) are now done** — G-4 is next.

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

- **G-1: Asset → GPU** — the full PNG → GPU path now exists end to end:
  - `kiln-assets/src/image.rs` (new) decodes every PNG colour type
    (grayscale, palette, RGB, RGBA, 16-bit, `tRNS`) into straight RGBA8 using
    the `png` crate (ADR-0027). `encode_png_rgba` is the inverse helper.
  - `kiln_core::Component::Sprite` gained an optional `texture` asset path
    (`"sprites/hero.png"`); prefab `ComponentOverride::Sprite` can override it.
  - `kiln-player/src/textures.rs` (new) collects every texture path referenced
    by the project, decodes them and uploads them via
    `SpriteRenderer::create_texture_rgba`, keeping the pixel size so sprites
    are sized to the image.
  - The player now **defers scene spawning until after GPU init** so textures
    exist when entities are created. Missing/invalid textures log a warning and
    fall back to the white tinted quad.
  - `kiln-player/src/loader.rs` loads project directories **recursively** and
    keys assets by project-relative path with `/` separators.
  - `kiln-editor/src/assets.rs` (new) adds an `AssetBrowser` that scans the
    project directory, lists files, selects images and renders a CPU-side
    `egui` preview (`ColorImage::from_rgba_unmultiplied`, since the decoded
    pixels are unpremultiplied). Project manifests are skipped and decode
    failures are listed in the asset panel, not only logged.
  - Also fixed pre-existing editor breakage: missing scene save/load request
    handling and menu actions in `kiln-editor/src/app.rs`.
  - `cargo fmt --all`, targeted Clippy and `cargo build --workspace` are clean;
    all 81 tests in `kiln-assets`/`kiln-core`/`kiln-player`/`kiln-editor` pass.

- **G-2: Real tilemap** — full chunked tilemap module, end to end:
  - `kiln-core/src/tilemap.rs` (new): `TileSet`, `TileMapLayer`, `TileMap`,
    `TileChunk`; per-tile solid flags; UV lookup; chunked storage keyed by
    `"cx,cy"`; solid-tile world rect query (`first_solid_hit`).
  - `kiln_core::Component::Tilemap` + `Project.tilemaps`/`Project.tilesets`
    (RO trip TOML). 15 new core tests.
  - `kiln-runtime/src/tilemap.rs` (new): runtime `Tilemap` component bound to
    tileset textures; camera-culling `collect_visible(view_bounds)` that skips
    off-screen chunks; solid hit query. 6 runtime tests.
  - `kiln-player`: uploads tileset PNGs with sprites; `spawn_tilemaps` in
    `kiln-player/src/spawn.rs` binds each entity's map; the frame gather pass
    draws tilemap chunks before sprites so entities draw on top. 4 new player
    tests.
  - `kiln-editor/src/tilemap_editor.rs` (new): `TilemapEditorState` panel —
    tilemap/layer selection, tileset combo, tile brush, solid toggle, a
    clickable painting grid, chunk-scrolled origin, +Layer/New. 3 editor
    tests; wired as `Tab::TilemapEditor` in `kiln-editor/src/app.rs`.
  - `kiln-render/src/camera.rs`: added `view_bounds()` + test.
  - `cargo fmt --all`, targeted Clippy clean; all tests pass across
    `kiln-core`/`kiln-render`/`kiln-runtime`/`kiln-player`/`kiln-editor`.

- **G-3: VM maturity** — strings are now first-class in the VM and frontends:
  - `kiln-ir/src/value.rs`: added `Value::Str(String)` (enum is `Clone`, not
    `Copy`); accessors now take `&self`; `values_equal` compares strings by
    content; `From<&str>`/`From<String>` conversions.
  - `kiln-ir/src/vm.rs`: added a `compare` helper so `Lt/Le/Gt/Ge` compare
    strings lexicographically (numbers still via `binop`); `Eq`/`Ne` use
    `values_equal`. `PushConst`/`Dup`/host-call `Ok(v)` now clone.
  - `kiln-script`: the lexer reads `"…"` string literals (with `\"`, `\\`,
    `\n`, `\r`, `\t` escapes) into `Token::String`; the parser produces
    `Expr::String`; the compiler emits `PushConst(Value::Str)`.
  - `kiln-graph/src/compiler.rs`: `ConstantValue::String` now compiles to a
    real `PushConst(Value::Str)` instead of the old hash placeholder.
  - New tests: value accessors/equality, VM string `Eq/Ne/Lt`, host-call string
    round-trip, lexer string + unterminated-string errors, script string
    return + equality end-to-end, graph string constant → push. `cargo fmt`,
    targeted Clippy clean; all tests pass in `kiln-ir`/`kiln-script`/`kiln-graph`.

---

## In progress

None

---

## Next steps (2D-first gate — before M29)

These are now tracked as checkboxes in `docs/TODO_LIST.md` **Phase 0.5 (G-1 … G-6)**.
Ordered by impact. One item per session; commit after each.

1. **G-1: Asset → GPU** — ✅ done
2. **G-2: Real tilemap** — ✅ done
3. **G-3: VM maturity** — ✅ done this session
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
- G-1 leftovers: the editor preview is CPU-side only (no runtime GPU texture in
  the editor), and the editor asset browser cannot yet **assign** a texture to a
  sprite — the `texture` field is only editable in project data.

**Blocked:** None — blocked only by product choice: finish 2D before 3D.
