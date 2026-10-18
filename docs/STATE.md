# Kiln Session Handoff

**Current milestone:** M1 complete (code compiles successfully)

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
- M1: Demo updated to use ECS + game loop
- API fixes: wgpu 22.1, hecs entity ID conversion, winit 0.30, const Duration

**In progress:**
- None

**Next 3 steps:**
1. Run `cargo run --bin demo` to verify window opens
2. Add keyboard input handling back to demo (simplified in API fix)
3. Begin M2 (2D renderer) if M1 acceptance criteria met

**Known issues:**
- Demo keyboard input simplified during API fix; needs restoration
- Demo render() is placeholder (M2 will add proper 2D rendering)
- Browser build deferred

**Last commands run:**
- $env:PATH += ";$env:USERPROFILE\.cargo\bin"; cargo check --workspace (passed)
- git add .
- git commit -m "Fix API compatibility: wgpu 22.1, hecs, winit 0.30"

**Files touched:**
- Cargo.toml (downgraded wgpu to 22.1)
- kiln-render/src/renderer.rs (API compatibility fixes)
- kiln-runtime/src/loop_.rs (const Duration fix)
- kiln-runtime/src/world.rs (entity ID conversion)
- kiln-runtime/src/input.rs (removed unused imports)
- demo/src/main.rs (simplified for winit 0.30, removed keyboard input)
