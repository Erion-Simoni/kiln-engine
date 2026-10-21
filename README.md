# Kiln

A lightweight standalone desktop game engine: editor + runtime + CLI.

**Status:** Planning phase. See `docs/ROADMAP.md` for milestones.

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

Pre-built installers are available on the [GitHub Releases](https://github.com/kiln-engine/kiln/releases) page:

- **Windows:** MSI installer
- **macOS:** DMG installer
- **Linux:** AppImage package

The editor is also available via `cargo install`:
```bash
cargo install --git https://github.com/kiln-engine/kiln kiln-editor
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
- **docs/ROADMAP.md** - Milestones M0-M14+
- **docs/DECISIONS.md** - Architecture decision records
- **docs/BUDGETS.md** - Size and performance budgets
- **docs/STATE.md** - Session handoff (current status)

## Technology

- Rust workspace, stable toolchain
- Rendering: wgpu (WebGPU + WebGL2 fallback)
- UI: egui (pure-Rust native, no Electron)
- ECS: hecs 0.11.2
- Scripting: KilnScript (custom language)

## License

TBD
