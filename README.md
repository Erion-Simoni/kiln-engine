# Kiln

A lightweight standalone desktop game engine: editor + runtime + CLI.

**Status:** M0–M28 and the 2D-first gate (G-1 through G-6) are complete —
textured sprites, real tilemaps, VM maturity, a vertical-slice sample, and
roadmap reconciliation all ship in `samples/vertical_slice/`. M29 (3D
foundation) is in progress: the `3d` feature and mesh/material types landed
in `kiln-core` (commit `4556431`). See `docs/ROADMAP.md` for milestones and
`docs/STATE.md` for the current session handoff.

## Goals

- **Tiny:** Web runtime <1.5 MB, editor installer <40 MB
- **Fast:** 60 FPS on 2015 laptops with integrated GPUs
- **Beautiful:** Art-directed 2D/2.5D with render tiers
- **One codebase:** Export to Web (WASM), desktop, and later mobile
- **Two ways to code:** Visual node graphs and text scripts compile to the same IR

## Quick start

```bash
# Install just, cargo-nextest
just check    # fmt + clippy + build
just test     # run tests
just run      # editor or demo
just web      # wasm demo in browser
```

## Download

No releases are published yet. Packaging (installers, code-signing, store
upload) is planned for M15–M16 but has not been shipped; see
`docs/PACKAGING.md` and the packaging section of `docs/ROADMAP.md`.

Build from source instead:

```bash
cargo run --bin kiln-editor                        # editor
cargo run -p kiln-player -- --archive samples/vertical_slice   # vertical slice
```

## Try the vertical slice

`samples/vertical_slice/` is a small 2D game that exercises the whole stack:
art loaded from disk, game rules written in KilnScript, save/load and an
on-screen HUD. It runs in the standalone player and in the editor.

```bash
cargo run -p kiln-player -- --archive samples/vertical_slice
```

See [samples/vertical_slice/README.md](samples/vertical_slice/README.md).

## Tutorial

New to Kiln? Start with the [Tutorial](docs/TUTORIAL.md) to build your first game.

## Documentation

- **AGENTS.md** - Agent brief and coding rules
- **docs/ARCHITECTURE.md** - Crate map, data flow, IR design
- **docs/ROADMAP.md** - Milestones M0-M61+
- **docs/DECISIONS.md** - Architecture decision records
- **docs/BUDGETS.md** - Size and performance budgets
- **docs/STATE.md** - Session handoff (current status)

## Technology

- Rust workspace, stable toolchain (rust 1.87)
- Rendering: wgpu 22.1 (WebGPU + WebGL2 fallback)
- UI: egui 0.29 (pure-Rust native, no Electron)
- ECS: hecs 0.11.2
- Scripting: KilnScript (custom language)

## License

TBD
