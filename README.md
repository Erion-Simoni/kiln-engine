# Kiln

A lightweight standalone desktop game engine: editor + runtime + CLI.

**Status:** M0–M28 and the 2D-first gate (G-1 through G-6) are complete —
textured sprites, real tilemaps, VM maturity, a vertical-slice sample, and
roadmap reconciliation all ship in `samples/vertical_slice/`. M29 (3D
foundation) is in progress: the `3d` feature and mesh/material types landed
in `kiln-core` (commit `4556431`). See `docs/ROADMAP.md` for milestones and
`docs/STATE.md` for the current session handoff.

For remaining production workflow gaps and test evidence, see [the 2D completion plan](docs/2D_COMPLETION_PLAN.md). Earlier milestone completion does not establish that every authoring/export workflow is ready.

## Goals

- **Portable:** Native and browser exports use prebuilt runtimes; package size is not a release goal
- **Fast:** 60 FPS on 2015 laptops with integrated GPUs
- **Beautiful:** Art-directed 2D/2.5D with render tiers
- **One codebase:** Export to Web (WASM), desktop, and later mobile
- **Two ways to code:** Visual node graphs and text scripts compile to the same IR

## Quick start

```bash
# Install just, cargo-nextest
just check    # fmt + clippy + build
just test     # run tests
just run      # native editor
just demo     # standalone demo
just web      # wasm demo in browser
```

## Download

Windows x64 portable candidate: [successful build and startup/export tests](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982526),
source `3ff72d3`. Follow [WINDOWS_PORTABLE.md](docs/WINDOWS_PORTABLE.md) to download,
extract the complete ZIP, verify it and launch `kiln-editor.exe` without Rust.
This is a hosted startup/CLI-export checkpoint; physical-device and full Windows
GUI authoring acceptance remain separate.

No public releases are published yet. The Linux release-candidate workflow builds
and tests a portable editor with its exporter, player, browser runtime, templates
and examples. Use only a completed successful run and follow the
[verified download instructions](docs/BUILD_PROVENANCE.md#hosted-release-candidate-workflow).
The installed editor needs no Rust, Cargo, Python or Node. Its current Linux
requirements and remaining platform checks are in
[PORTABLE_EDITOR.md](docs/PORTABLE_EDITOR.md). Installers, OS code signing and
other-platform distribution remain in progress; see [PACKAGING.md](docs/PACKAGING.md).

Accepted Linux candidate: [licensed candidate](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982528),
source `3ff72d3`. Candidate and attestation artifacts are retained for 14 days;
select a later successful run if those downloads have expired.

To build from source:

```bash
cargo run --bin kiln-editor                        # editor
cargo run -p kiln-player -- --archive samples/vertical_slice   # vertical slice
```

## Play the reference games

[Emberwatch, Skybridge and Switchyard](games/README.md) are three small editor-authored
2D games: top-down action, a physics platformer and a circuit puzzle. Open their
`kiln.toml` in the editor, then use Play Project Start or Build / Export. Their
authoring steps, native/browser playthroughs and remaining release gates are documented.

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
- **[Engine completion plan](docs/ENGINE_COMPLETION_PLAN.md)** - Authoritative 2D, 3D, production and platform roadmap
- **docs/ROADMAP.md**, **ROADMAP_FULL_M14_PLUS.md** - Historical milestones and design references
- **docs/DECISIONS.md** - Architecture decision records
- **docs/BUDGETS.md** - Performance/resource goals and optional size observations
- **docs/STATE.md** - Session handoff (current status)
- **docs/TUTORIAL.md** - Build your first game
- **docs/GETTING_STARTED.md** - Make a 2D game from scratch
- **docs/KILNSCRIPT.md** - KilnScript language reference
- **docs/TILEMAPS.md** - Chunked tilemaps with collision

## Technology

- Rust workspace, stable toolchain (rust 1.87)
- Rendering: wgpu 22.1 (WebGPU + WebGL2 fallback)
- UI: egui 0.29 (pure-Rust native, no Electron)
- ECS: hecs 0.11.2
- Scripting: KilnScript (custom language)

## License

Kiln source code is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. Commercial use is permitted under
those terms. Dependencies, fonts and other third-party materials retain their
own licenses and notices; portable installations include their license inventory.
