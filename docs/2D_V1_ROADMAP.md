# Kiln 2D v1.0 Roadmap

> **Historical plan / technical reference.** The [unified engine completion plan](ENGINE_COMPLETION_PLAN.md) is authoritative for current scope, order, status and release gates. Older completion claims and package-size goals below are historical; preserve their evidence, but do not use them to close current milestones.


**Goal:** A shippable 2D engine — every core 2D feature works end-to-end,
export to web and desktop is verified, and no critical gap blocks a real
2D game release. This is the milestone between the v0.5 "2D complete" gate
(M26) and the 3D work (M29+).

**Status:** Active — most phases complete, only Phase G and Phase I remain
(both need physical hardware).

**Rule:** one focused session per task, every acceptance item testable
(command, screenshot, or automated check). No new dependency without an ADR.

---

## What "2D v1.0" means

The 2D-first gate (G-1 … G-6) is complete: textured sprites, real tilemaps,
VM maturity, a vertical-slice sample, reconciled roadmap checkboxes, and
editor truth (engine-drawn scene view, PIE, undo/redo).

---

## Acceptance criteria (testable)

| # | Criterion | Status |
|---|-----------|--------|
| V1-1 | Script can spawn entities and read world state | ✅ Done |
| V1-2 | Player main menu / loader UI | ✅ Done |
| V1-3 | Editor scene view post effects match player | ✅ Done |
| V1-4 | Web end-to-end export test automated (CI) | ✅ Done |
| V1-5 | Clean machine run documented + tested | ✅ Done (results pending hardware) |
| V1-6 | SDF font pipeline automated | ✅ Done |
| V1-7 | Former package-size goal | Removed by owner request (ADR-0062) |
| V1-8 | Frame budget on reference Low/Medium GPU | ⏳ Phase G — needs hardware |
| V1-9 | All remaining TODO_LIST.md 2D items complete | ✅ Done |
| V1-10 | v0.5 gate re-verified | ⏳ Phase I — needs device testing |

---

## Phases (done / remaining)

### ✅ Phase A: Gameplay API (V1-1)
- A-1: Script builtin ID alignment fixed — all host-call IDs 0–55 match `BUILTINS` table
- A-2: Complete script API documented in `docs/KILNSCRIPT.md`

### ✅ Phase B: Player menu / loader UI (V1-2)
- B-1: Already implemented in `kiln-player/src/main.rs` — `MenuState` enum, keyboard navigation, save/load slots

### ✅ Phase C: Editor scene view post effects (V1-3)
- C-1: Already implemented in `kiln-editor/src/app.rs` — post framebuffer + pipeline applied via `pipeline.apply()`

### ✅ Phase D: Web end-to-end export test (V1-4)
- D-1: Browser build/Clippy/export/playback smoke tests are configured in `.github/workflows/ci.yml`. Package-size ceilings and size-failing CI are not goals (ADR-0062).

### ✅ Phase E: Clean machine run (V1-5)
- E-1: `docs/CLEAN_MACHINE_TEST.md` already documents the full procedure

### ✅ Phase F: SDF font pipeline (V1-6)
- F-1: ADR-0036 (fontdue dependency), `fontdue` + `image` in workspace Cargo.toml, `kiln-ui` `fonts` feature, `kiln-ui/src/sdf_builder.rs` (SDF atlas builder), `kiln-cli import-font` subcommand

### ✅ Phase H: Close remaining TODO_LIST.md items (V1-9)
- Closed: REN-1 (post tests), REN-2 (frame graph post), DT-1 (device tier), U-1 (updater stubs)

### ⏳ Phase G: Frame budget (V1-8) — NEXT
- G-1: Measure frame time on reference Low/Medium GPU; record in `docs/BUDGETS.md`. Needs physical hardware.

### ⏳ Phase I: v0.5 gate re-verification (V1-10) — AFTER G
- I-1: Verify vertical slice on web, desktop, Android, iOS; record results. Needs device testing.

---

## Out of scope for 2D v1.0

- 3D rendering (M29+) — separate roadmap
- Mobile store publishing (M16/M26) — Steam/itch plumbing exists, store
  submission is deferred
- Plugin API (M54) — separate
- Console/XR (M59/M60) — separate

---

## Done / In progress / Next

- **Done:** 2D gate (G-1 … G-6); M0–M28; Phases A, B, C, D, E, F, H
- **Next:** Phase G (frame budget) → Phase I (v0.5 gate re-verification)
- **Blocked:** None

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
