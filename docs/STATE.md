# Kiln Session Handoff

**Current milestone:** M1 complete

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
- M1: Demo with ECS + game loop + keyboard input (arrow keys move marker)
- API fixes: wgpu 22.1, hecs entity ID conversion, winit 0.30 logical_key, const Duration

**In progress:**
- None

**Next 3 steps:**
1. Run `cargo run --bin demo` to verify window opens and input works
2. Begin M2 (2D renderer) if M1 acceptance verified
3. Or address any M1 issues found during testing

**Known issues:**
- Demo render() is placeholder (M2 will add proper 2D rendering)
- Browser build deferred
- InputState field unused in demo (placeholder for future use)

**Last commands run:**
- $env:PATH += ";$env:USERPROFILE\.cargo\bin"; cargo check --workspace (passed)
- git commit -m "M1: fix keyboard input using logical_key for winit 0.30"

**Files touched:**
- Cargo.toml (downgraded wgpu to 22.1)
- kiln-render/src/renderer.rs (API compatibility fixes)
- kiln-runtime/src/loop_.rs (const Duration fix)
- kiln-runtime/src/world.rs (entity ID conversion)
- kiln-runtime/src/input.rs (removed unused imports)
- demo/src/main.rs (keyboard input using logical_key)
- docs/STATE.md (M1 completion)
