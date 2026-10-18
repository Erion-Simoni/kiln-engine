# Kiln — Agent Brief

Standalone desktop game engine (editor + runtime + CLI). Light like Godot's install story; art-directed looks; tiny web/native exports. **Planning docs only until M0 is approved.**

## Crate map (planned)

| Crate | Job |
| --- | --- |
| `kiln-core` | Math, time, shared types, ECS glue |
| `kiln-render` | wgpu renderer, tiers, post, 2D→3D |
| `kiln-assets` | Import, hash cache, packed archives |
| `kiln-ir` | Gameplay IR + bytecode VM |
| `kiln-script` | Text language frontend → IR |
| `kiln-graph` | Node-graph model + compile → IR |
| `kiln-runtime` | Loop, scene, input, audio hooks |
| `kiln-editor` | Native desktop editor app |
| `kiln-cli` | Headless build/export/pack |
| `kiln-web` | WASM export shell |
| `kiln-plugin-api` | Sandboxed WASM plugin ABI |
| `samples/` | Tutorial + sample games |

## Commands (placeholders until M0)

```text
just check    # fmt + clippy + build
just test     # cargo nextest
just run      # editor or demo window
just web      # wasm demo in browser
just size     # wasm/native size budgets
just assets   # import / pack
just package  # installers via cargo-dist
```

## Coding rules

1. ONE milestone per session. Never start the next unasked.
2. Plan ≤10 bullets, then implement in small steps.
3. Tests alongside code; `just check` must pass before every commit.
4. Files under 400 lines; small functions; clear module boundaries.
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
2. Confirm current milestone; do only that milestone.
3. Update `STATE.md` before finishing (Done / Next / Known issues).

## Do not

- Use Electron. Prefer pure-Rust native; Tauri only via ADR.
- Add engine/runtime crates until M0 is approved.
- Start M(n+1) in the same session as M(n).
- Bypass budgets in `docs/BUDGETS.md` without an ADR.
- Commit secrets, large binaries, or generated `target/` artifacts.

## Context pointers

- Architecture: `docs/ARCHITECTURE.md`
- Roadmap: `docs/ROADMAP.md`
- ADRs: `docs/DECISIONS.md`
- Budgets: `docs/BUDGETS.md`
- Handoff: `docs/STATE.md`
