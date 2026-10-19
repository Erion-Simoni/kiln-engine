# Fixes M1–M29 (2D engine correctness pass)

Created 2026-10-19 from a code audit (grep for TODO/stub, file reads, `cargo check`).
Goal: M1–M28 must be real, working 2D engine features with no stubs.
**M29 (3D core) is intentionally NOT started** — it is out of scope for a 2D engine
and `STATE.md` blocks it until the 2D gate is met. It is listed so the range is covered.

Status legend: `[ ]` open · `[x]` fixed + tested · `[~]` partly fixed · `[-]` deferred (reason given)

## Findings

| # | Milestone | Finding | Where | Status |
| --- | --- | --- | --- | --- |
| F1 | M21 UI | `UiRenderer` returns empty `Vec`s; widgets never draw | `kiln-ui/src/ui_renderer.rs` | [ ] |
| F2 | M21 UI | `NineSliceRenderer::render` is a no-op TODO | `kiln-ui/src/render.rs` | [ ] |
| F3 | M21 UI | `TextRenderer::measure` returns `(0,0)`; `render` no-op | `kiln-ui/src/render.rs` | [ ] |
| F4 | M4/M5 VM | `Value` only `I64`/`Bool`; no float → unusable for gameplay | `kiln-ir/src/value.rs` | [ ] |
| F5 | M9 Post | `PostPipeline::apply` and `set_tier` are TODO stubs | `kiln-render/src/post.rs` | [ ] |
| F6 | M14 Player | `kiln-player` loads `.kar` then exits | `kiln-player/src/main.rs` | [ ] |
| F7 | M6/M7 Editor | Open/Save/Export/Undo/Redo/Add entity/Add component/Import/Clear console are TODOs | `kiln-editor/src/app.rs` | [ ] |
| F8 | M7 Editor | Scene view is egui rects, PIE is a bool toggle | `kiln-editor/src/app.rs` | [ ] |
| F9 | M2 Tilemap | No tilemap module (only a sample with coloured sprites) | `samples/tilemap` | [ ] |
| F10 | M10/M14 Web | `kiln-web` is an isolation/worker shell, not a game loop export | `kiln-web/src/lib.rs` | [-] needs browser toolchain |
| F11 | M28 | Frame-graph post passes are timing-only clears | `kiln-render/src/frame.rs` | [-] tied to F5 |
| F12 | M11 Audio | Audio device management minimal (ADR-0014) | `kiln-runtime/src/audio.rs` | [-] ADR deferred |
| F13 | Docs | ROADMAP checkboxes overstate completion | `docs/ROADMAP.md`, `ROADMAP_FULL_M14_PLUS.md` | [ ] |
| F14 | M29 | 3D core | — | [-] out of 2D scope |
| F15 | R0/M0 | `cargo clippy -D warnings` fails on `kiln-core` (7 errors: derived-Default, `iter_map_values`, collapsible `if let`, `from_str` name clash) | `kiln-core/src/{project,prefab,device_tier}.rs` | [ ] |

## Fix log (in order)

### Fix 1 — F1/F2/F3: UI draw path (M21) — DONE

Replaced the three stubs with a real draw path, **no new dependencies**
(`AGENTS.md` rule 5 respected — a vector font crate would need an ADR):

- `kiln-ui/src/font.rs` (new): built-in 5×7 bitmap font (A–Z, digits,
  common punctuation); lowercase maps to uppercase, unknown → hollow box.
- `kiln-ui/src/text.rs` (new): `TextRenderer::measure` / `render` — real
  metrics + horizontal run-merged glyph quads, multi-line support.
- `kiln-ui/src/render.rs`: `NineSliceRenderer::render` now emits up to 9
  quads with correct UV rects and border-down-scaling when bounds are too
  small; `NineSlice` gained `tex_width`/`tex_height` for UV computation.
- `kiln-ui/src/ui_renderer.rs`: `UiRenderer` now returns `UiQuad`
  (texture + `SpriteInstance`) instead of empty vecs — panel, button
  (hover/pressed states + centred label), list (rows, selection, scroll
  culling), nine-slice and raw text; plus `submit()` to push into
  `SpriteRenderer`.

**Tests:** 44 pass in `kiln-ui`; whole-workspace `cargo test` 0 failures;
`cargo fmt -p kiln-ui` clean; no clippy findings in `kiln-ui`.

