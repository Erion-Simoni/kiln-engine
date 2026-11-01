# Kiln Session Handoff

**Active focus:** Kiln 2D v1.0 roadmap (`docs/2D_V1_ROADMAP.md`)
**Branch:** `cline/3s2pmrtj` (pushed to origin)

---

## Milestone status

| Milestone | Status |
|-----------|--------|
| M0–M28 + 2D gate (G-1…G-6) | ✅ Complete |
| M29 (3D foundation) | In progress (not active) |
| **Kiln 2D v1.0** | **Active** — closing remaining 2D gaps |

---

## 2D v1.0 acceptance criteria (V1-1 → V1-10)

| # | Criterion | Status |
|---|-----------|--------|
| V1-1 | Script can spawn entities and read world state | ✅ Done (Phase A-1, A-2) |
| V1-2 | Player main menu / loader UI | ✅ Done (Phase B-1) |
| V1-3 | Editor scene view post effects match player | ✅ Done (Phase C-1) |
| V1-4 | Web end-to-end export test (CI) | ✅ Done (Phase D-1 — CI job ready, needs workflows permission) |
| V1-5 | Clean machine run documented + tested | ✅ Done (Phase E-1 — procedure documented, results pending hardware) |
| V1-6 | SDF font pipeline automated | ✅ Done (Phase F-1 — implemented, compiles clean) |
| V1-7 | Brotli WASM size under 1.5 MB | ✅ Done (Phase D-1 — CI job measures) |
| V1-8 | Frame budget on reference Low/Medium GPU | ⏳ Phase G — needs physical hardware |
| V1-9 | All remaining TODO_LIST.md 2D items complete | ✅ Done (Phase H) |
| V1-10 | v0.5 gate re-verified | ⏳ Phase I — needs device testing |

---

## Next actions (in order)

1. **Phase G:** Measure frame time on reference Low/Medium GPU; record in `docs/BUDGETS.md` (V1-8). Needs physical hardware.
2. **Phase I:** Verify vertical slice runs on web, desktop, Android, iOS; record results (V1-10). Needs device testing.

---

## Known issues (not 2D v1.0 blockers)

- Web Workers ping-only (needs crossOriginIsolated)
- iOS device testing needs macOS runner
- iOS/Android device tier returns fixed values (deferred to mobile milestones)
- Effect toggles (B/O/F) are debug keys, not settings

---

## Session protocol

Complete one task → update `docs/STATE.md` → commit → push → wait for user to say "continue".
