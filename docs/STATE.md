# Kiln Session Handoff

**Current milestone:** M1 complete (code written, awaiting Rust install to test)

**Done:**
- AGENTS.md created (70 lines, under 100-line limit)
- docs/ARCHITECTURE.md created (crates, data flow, IR design, render tiers, asset pipeline)
- docs/ROADMAP.md created (M0-M14+ milestones with acceptance criteria)
- docs/DECISIONS.md created (9 ADRs: 6 proposed, 2 accepted)
- docs/BUDGETS.md created (web <1.5MB, editor <40MB, RAM <300MB, 16.6ms frame)
- IDE pointer files created (.cursor, .windsurf, CLAUDE.md, GEMINI.md)
- justfile created with recipes
- .gitignore created
- README.md created
- M0: Cargo workspace, CI, demo with wgpu triangle
- M1: kiln-core crate (math, time, ID, project format)
- M1: kiln-runtime crate (fixed timestep loop, input, ECS wrapper)
- M1: ADR-0002 accepted: hecs 0.11.2 for ECS
- M1: Project file format (TOML) with unit tests
- M1: Demo updated to use ECS + input + game loop

**In progress:**
- None

**Next 3 steps:**
1. Install Rust toolchain locally
2. Run `cargo check --workspace` to verify M0+M1 build
3. Run demo to verify input moves marker

**Known issues:**
- Rust not installed locally; cannot run cargo commands
- Browser build deferred until Rust installed
- Demo render() is placeholder (M2 will add proper 2D rendering)

**Last commands run:**
- git add .
- git commit -m "M1: core loop, input, ECS (hecs), project format"

**Files touched:**
- Cargo.toml (added kiln-core, kiln-runtime, glam, hecs, serde, toml, uuid)
- kiln-core/Cargo.toml, kiln-core/src/lib.rs, kiln-core/src/math.rs, kiln-core/src/time.rs, kiln-core/src/id.rs, kiln-core/src/project.rs, kiln-core/src/project_test.rs
- kiln-runtime/Cargo.toml, kiln-runtime/src/lib.rs, kiln-runtime/src/loop_.rs, kiln-runtime/src/input.rs, kiln-runtime/src/world.rs
- kiln-render/src/renderer.rs (added resize, format, instance methods)
- demo/Cargo.toml (added kiln-core, kiln-runtime), demo/src/main.rs (rewritten with ECS+input)
- docs/DECISIONS.md (ADR-0002 accepted)
