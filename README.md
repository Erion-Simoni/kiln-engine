# Kiln

A lightweight standalone desktop game engine: editor + runtime + CLI.

**Status:** Planning phase. See `docs/ROADMAP.md` for milestones.

## Goals

- **Tiny:** Web runtime <1.5 MB, editor installer <40 MB
- **Fast:** 60 FPS on 2015 laptops with integrated GPUs
- **Beautiful:** Art-directed 2D/2.5D with render tiers
- **One codebase:** Export to Web (WASM), desktop, and later mobile
- **Two ways to code:** Visual node graphs and text scripts compile to the same IR

## Quick start (after M0)

```bash
# Install just, cargo-nextest, trunk
just check    # fmt + clippy + build
just test     # run tests
just run      # editor or demo
just web      # wasm demo in browser
```

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
- ECS: bevy_ecs or hecs (decision in M1)
- Scripting: Rhai or custom (decision in M5)

## License

TBD
