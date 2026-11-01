# Kiln 2D v1.0 Roadmap

**Goal:** A shippable 2D engine — every core 2D feature works end-to-end,
export to web and desktop is verified, and no critical gap blocks a real
2D game release. This is the milestone between the v0.5 "2D complete" gate
(M26) and the 3D work (M29+).

**Status:** Planning — no code written yet for the v1.0-specific items.

**Rule:** one focused session per task, every acceptance item testable
(command, screenshot, or automated check). No new dependency without an ADR.

---

## What "2D v1.0" means

The 2D-first gate (G-1 … G-6) is complete: textured sprites, real tilemaps,
VM maturity, a vertical-slice sample, reconciled roadmap checkboxes, and
editor truth (engine-drawn scene view, PIE, undo/redo).

---

## Acceptance criteria (testable)

| # | Criterion | Test | Location |
|---|-----------|------|----------|
| V1-1 | Script can spawn entities and read world state | Automated test: script spawns a sprite, reads its transform | `kiln-script`, `kiln-ir`, `kiln-runtime` |
| V1-2 | Player shows a main menu / loader UI | Run player, menu appears, can load a project/scene | `kiln-player` |
| V1-3 | Editor scene view runs post effects (matches player) | Screenshot: editor scene view shows bloom/fog/outline like player | `kiln-render`, `kiln-editor` |
| V1-4 | Web end-to-end export test automated | CI builds WASM, verifies runnable output | `kiln-web`, `kiln-cli`, CI |
| V1-5 | Clean machine run documented + tested | Engine runs on machine without Rust toolchain | `docs/CLEAN_MACHINE_TEST.md` |
| V1-6 | SDF font pipeline automated | `just assets` imports a TTF and produces an SDF atlas | `kiln-assets`, `kiln-cli` |
| V1-7 | Brotli WASM size measured and under 1.5 MB | `wasm-opt -Oz` + `brotli`, recorded in BUDGETS.md | `docs/BUDGETS.md` |
| V1-8 | Frame budget measured on reference Low/Medium GPU | Frame time recorded in BUDGETS.md | `docs/BUDGETS.md` |
| V1-9 | All remaining TODO_LIST.md 2D items complete | Every `[ ]` in TODO_LIST.md that is 2D-related is `[x]` | `docs/TODO_LIST.md` |
| V1-10 | v0.5 gate re-verified | Vertical slice runs on web, desktop, Android, iOS | `samples/vertical_slice` |

---

## Task breakdown (in priority order)

### Phase A: Gameplay API (scripts can spawn/read)

**A-1: Script entity-spawning builtins**
Add KilnScript builtins (or host calls) to spawn a sprite entity, set its
transform, and read arbitrary world state back into the script.
- Crates: `kiln-ir` (host calls), `kiln-runtime` (slice/world access),
  `kiln-script` (builtins)
- Test: script spawns a sprite at runtime and reads its position
- Blocks: V1-1

**A-2: Script API surface documentation**
Document the full builtin list (spawn, read transform, read input, etc.)
in `docs/KILNSCRIPT.md`.
- Blocks: V1-1

### Phase B: Player menu / loader UI

**B-1: Player main menu + scene loader**
`kiln-player` shows a main menu on startup: "New Game", "Load Scene",
"Quit". Selecting a scene loads and runs it.
- Crate: `kiln-player` (kiln-ui widgets)
- Test: run player, menu appears, loads vertical slice
- Blocks: V1-2

### Phase C: Editor scene view post effects

**C-1: Wire frame graph post chain into editor scene view**
The editor's scene view uses the frame graph which names post passes but
doesn't run the post chain. Wire `Renderer::apply_post` into the editor
so bloom/fog/outline appear in the editor like the player.
- Crates: `kiln-render` (frame graph), `kiln-editor` (scene view)
- Test: screenshot comparison editor vs player with post on
- Blocks: V1-3

### Phase D: Web end-to-end export test

**D-1: Automated web export test in CI**
CI builds `kiln-web` WASM, runs `wasm-opt -Oz` + `brotli`, verifies
output is runnable (or at least valid WASM under size budget).
- Crates: `kiln-web`, `kiln-cli`, CI
- Test: `just size` passes on CI
- Blocks: V1-4, V1-7

### Phase E: Clean machine run

**E-1: Document clean-machine test procedure**
`docs/CLEAN_MACHINE_TEST.md` already exists as a manual procedure; verify
it is accurate and record results.
- Blocks: V1-5

### Phase F: SDF font pipeline

**F-1: Automate TTF → SDF atlas import**
Add a `kiln-cli` command (or asset importer) that takes a TTF, runs the
SDF/MSDF atlas builder already in `kiln-ui/src/text.rs`, and emits an
atlas texture + metadata.
- Crates: `kiln-assets`, `kiln-cli`, `kiln-ui`
- Test: `just assets` produces a usable SDF atlas from a TTF
- Blocks: V1-6

### Phase G: Frame budget on reference hardware

**G-1: Measure frame time on Low/Medium reference GPU**
Run the vertical slice on a Low (integrated 2015) and Medium (GTX 1050)
class GPU; record frame time in BUDGETS.md.
- Blocks: V1-8

### Phase H: Close remaining TODO_LIST.md items

**H-1: Mark all 2D-related `[ ]` items complete**
Go through `docs/TODO_LIST.md` and close every 2D-related incomplete item
(REN-2 frame graph post stubs, DT-1 device capabilities, etc.).
- Blocks: V1-9

### Phase I: v0.5 gate re-verification

**I-1: Verify vertical slice on web, desktop, Android, iOS**
Run the vertical slice on each platform and record results.
- Blocks: V1-10

---

## Out of scope for 2D v1.0

- 3D rendering (M29+) — separate roadmap
- Mobile store publishing (M16/M26) — Steam/itch plumbing exists, store
  submission is deferred
- Plugin API (M54) — separate
- Console/XR (M59/M60) — separate

---

## Done / In progress / Next

- **Done:** 2D gate (G-1 … G-6) complete; M0–M28 complete (Partial verdicts)
- **In progress:** None
- **Done this session (2D v1.0):**
  - Phase A-1: Script builtin ID alignment fixed in `kiln-runtime/src/slice.rs` — all host-call IDs (0–55) now match the `BUILTINS` table; scripts can spawn/read entities (V1-1).
  - Phase A-2: Complete script API surface documented in `docs/KILNSCRIPT.md` — audio, entity, animation, UI, input, localization, save/load builtins all listed (V1-1).
  - Phase B-1: Player main menu + scene loader already implemented in `kiln-player/src/main.rs` — `MenuState` enum with Main/LoadGame/Playing/Settings, keyboard navigation, `start_game()`, save/load slots (V1-2).
  - Phase C-1: Editor scene view post effects already implemented in `kiln-editor/src/app.rs` (lines 2128-2140) — the editor creates `scene_view_post_framebuffer`/`scene_view_post_pipeline` when tier != Low, renders the scene to the post framebuffer, then applies post via `pipeline.apply()`, matching the player's `Renderer::apply_post` (V1-3).
  - Phase D-1: Web end-to-end export test — added `wasm` CI job to `.github/workflows/ci.yml` that builds `kiln-web` for `wasm32-unknown-unknown --release`, runs `wasm-opt -Oz` + `brotli`, and fails if brotli-compressed size exceeds 1.5 MB (V1-4, V1-7). Updated `docs/BUDGETS.md` to document CI measurement.
- **Next:** Phase E-1 (clean machine run) — verify `docs/CLEAN_MACHINE_TEST.md` procedure and record results (V1-5).

---

## References

- `docs/STATE.md` — session handoff and known issues
- `docs/TODO_LIST.md` — remaining stubs and incomplete features
- `docs/ROADMAP.md` — M0–M17+ verdicts
- `ROADMAP_FULL_M14_PLUS.md` — M14–M61+ plan
- `docs/BUDGETS.md` — size and performance budgets
- `docs/KILNSCRIPT.md` — KilnScript language reference
- `docs/CLEAN_MACHINE_TEST.md` — clean-machine test procedure
- `docs/WEB_EXPORT_TESTING.md` — web export manual test procedure
- `docs/SDF_TEXT.md` — SDF font manual workflow
