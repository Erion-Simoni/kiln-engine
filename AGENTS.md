# Kiln — Agent Brief

Standalone desktop game engine (editor + runtime + CLI). Current completion scope,
ordering and gates live in `docs/ENGINE_COMPLETION_PLAN.md`. 2D release acceptance
is still open; full 3D remains gated on G2D. Existing 3D types and visual preview
are scaffolding, not a completed 3D runtime. Historical milestone claims below
must be checked against the authoritative plan and recorded evidence.

## Crate map (planned)

| Crate | Job |
| --- | --- |
| `kiln-core` | Math, time, shared types, ECS glue, feature-gated 3D mesh/material types |
| `kiln-render` | wgpu renderer, tiers, post, 2D batching → 3D, frame graph, GPU timestamps |
| `kiln-assets` | Import, hash cache, packed archives |
| `kiln-ir` | Gameplay IR + bytecode VM |
| `kiln-script` | Text language frontend → IR |
| `kiln-graph` | Node-graph model + compile → IR |
| `kiln-runtime` | Loop, scene, input, schedule/jobs, optional audio/physics/threads |
| `kiln-editor` | Native desktop editor app (egui shell, scene view, PIE, undo) |
| `kiln-cli` | Headless build/export/pack |
| `kiln-web` | WASM export shell |
| `kiln-plugin-api` | Sandboxed WASM plugin ABI |
| `kiln-player` | Thin standalone game launcher |
| `kiln-platform` | Platform services (store, Steam, device capabilities) |
| `kiln-ui` | UI widgets and renderer |
| `kiln-input` | Input handling |
| `kiln-anim` | Animation |
| `kiln-vfx` | VFX / particles |
| `kiln-save` | Save/load |
| `kiln-updater` | Auto-updater (feature-gated stubs) |
| `kiln-ai` | AI |
| `kiln-gameplay` | Gameplay |
| `samples/` | Tutorial + sample games |

## Commands (placeholders until M0)

```text
just check    # fmt + clippy + build
just test     # cargo nextest
just run      # editor or demo window
just web      # wasm demo in browser
just size     # optional artifact size report; no release-size target
just assets   # import / pack
just package  # experimental binary archives; complete bundles use portable-editor
```

## Coding rules

1. ONE milestone per session. Never start the next unasked.
2. Plan ≤10 bullets, then implement in small steps.
3. Tests alongside code; `just check` must pass before every commit.
4. Small functions; clear module boundaries.
5. No new dependency without an ADR entry in `docs/DECISIONS.md`.
6. Targeted search/grep; never re-read or paste whole large files; summarize.
7. Boring, explicit Rust; no clever macros; document public items.
8. Never refactor unrelated code. Architecture changes → `DECISIONS.md` first.
9. Commit after each completed step with a clear message.
10. End every session by rewriting `docs/STATE.md` (never only append).
11. If ambiguous: ask ONE concise question; else decide and log in ADR/`STATE.md`.
12. Stuck after 3 attempts: stop; describe blocker in `STATE.md`.

## Session protocol

1. Read `/AGENTS.md` and `/docs/STATE.md` first.
2. Read `/docs/ENGINE_COMPLETION_PLAN.md` for the authoritative milestone plan; older roadmap files are historical references.
3. Confirm current milestone; do only that milestone.
4. Update `STATE.md` before finishing (Done / Next / Known issues).

## Do not

- Use Electron. Prefer pure-Rust native; Tauri only via ADR.
- Add engine/runtime crates until M0 is approved. (All crates now exist; new ones need an ADR.)
- Start M(n+1) in the same session as M(n).
- Bypass active performance/resource budgets in `docs/BUDGETS.md` without an ADR. Package size is informational per ADR-0062.
- Commit secrets, large binaries, or generated `target/` artifacts.

## Context pointers

- Architecture: `docs/ARCHITECTURE.md`
- Active plan: `docs/ENGINE_COMPLETION_PLAN.md`
- Historical milestones: `docs/ROADMAP.md`, `ROADMAP_FULL_M14_PLUS.md`
- ADRs: `docs/DECISIONS.md`
- Budgets: `docs/BUDGETS.md`
- Handoff: `docs/STATE.md`
- Tutorial: `docs/TUTORIAL.md`
- Getting started: `docs/GETTING_STARTED.md`
- KilnScript: `docs/KILNSCRIPT.md`
- Tilemaps: `docs/TILEMAPS.md`
