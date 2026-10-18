# Kiln Session Handoff

**Current milestone:** M1 partial (keyboard input blocked by winit 0.30 API)

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
1. Investigate winit 0.30 keyboard API (PhysicalKey enum changed)
2. Restore keyboard input handling to demo
3. Complete M1 acceptance criteria

**Known issues:**
- winit 0.30 keyboard API: PhysicalKey::Named and PhysicalKey::KeyCode variants not found - needs investigation
- Demo render() is placeholder (M2 will add proper 2D rendering)
- Browser build deferred

**Last commands run:**
- $env:PATH += ";$env:USERPROFILE\.cargo\bin"; cargo check --workspace (passed)

**Files touched:**
- Cargo.toml (downgraded wgpu to 22.1)
- kiln-render/src/renderer.rs (API compatibility fixes)
- kiln-runtime/src/loop_.rs (const Duration fix)
- kiln-runtime/src/world.rs (entity ID conversion)
- kiln-runtime/src/input.rs (removed unused imports)
- demo/src/main.rs (simplified for winit 0.30, keyboard input removed due to API issue)
