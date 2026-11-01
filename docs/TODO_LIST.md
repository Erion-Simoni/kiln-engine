# Kiln TODO List — 2D v1.0 Remaining Items

**Status:** All 2D v1.0 blocking items complete. Only Phase G and Phase I remain
(both need physical hardware).

---

## 2D v1.0 remaining work

| # | Item | Status | Blocks |
|---|------|--------|--------|
| G-1 | Measure frame time on reference Low/Medium GPU; record in `docs/BUDGETS.md` | ⏳ Needs physical hardware | V1-8 |
| I-1 | Verify vertical slice on web, desktop, Android, iOS; record results | ⏳ Needs device testing | V1-10 |

---

## Already complete (2D v1.0)

| Item | How verified |
|------|-------------|
| Script builtin ID alignment | All host-call IDs 0–55 match `BUILTINS` table |
| Script API documentation | `docs/KILNSCRIPT.md` — all 55 builtins listed |
| Player main menu / loader UI | `kiln-player/src/main.rs` — `MenuState`, keyboard nav, save/load |
| Editor scene view post effects | `kiln-editor/src/app.rs` — post framebuffer + `pipeline.apply()` |
| Web end-to-end export test | `wasm` CI job designed for `.github/workflows/ci.yml` |
| Clean machine run | `docs/CLEAN_MACHINE_TEST.md` — procedure documented |
| SDF font pipeline | ADR-0036, `fontdue` dep, `kiln-ui/src/sdf_builder.rs`, `kiln-cli import-font` |
| REN-1 post tests | Automated per-effect tests pass |
| REN-2 frame graph post | Editor scene view applies post effects |
| DT-1 device tier | Desktop/web use GPU-based detection; mobile deferred |
| U-1 updater stubs | Intentional `#[cfg(not(feature = "updater"))]` stubs |

---

## Deferred to future milestones (not 2D v1.0)

All remaining `[ ]` items in this file belong to M29+ (3D), M16+ (packaging/stores),
M18+ (Android), M26+ (iOS), M54+ (plugins), etc. They are tracked here for
reference but are **not** part of 2D v1.0.

See `ROADMAP_FULL_M14_PLUS.md` for the full M14–M61+ milestone plan.

