# Kiln Session Handoff

**Current milestone:** M0 complete (code written, awaiting Rust install to test)

**Done:**
- AGENTS.md created (70 lines, under 100-line limit)
- docs/ARCHITECTURE.md created (crates, data flow, IR design, render tiers, asset pipeline)
- docs/ROADMAP.md created (M0-M14+ milestones with acceptance criteria)
- docs/DECISIONS.md created (8 ADRs: 7 proposed, 1 accepted)
- docs/BUDGETS.md created (web <1.5MB, editor <40MB, RAM <300MB, 16.6ms frame)
- IDE pointer files created (.cursor, .windsurf, CLAUDE.md, GEMINI.md)
- justfile created with recipes
- .gitignore created
- README.md created
- Cargo workspace created (kiln-render, kiln-web, demo)
- GitHub Actions CI workflow created
- Demo binary with winit + wgpu triangle rendering
- ADR-0008 accepted: pollster for async runtime

**In progress:**
- None

**Next 3 steps:**
1. Install Rust toolchain locally
2. Run `cargo check --workspace` to verify build
3. Run demo to verify native window + triangle

**Known issues:**
- Rust not installed locally; cannot run cargo commands
- Browser build deferred until Rust installed

**Last commands run:**
- git add .
- git commit -m "M0: workspace, CI, demo with wgpu triangle"

**Files touched:**
- Cargo.toml (workspace root)
- kiln-render/Cargo.toml, kiln-render/src/lib.rs, kiln-render/src/renderer.rs
- kiln-web/Cargo.toml, kiln-web/src/lib.rs
- demo/Cargo.toml, demo/src/main.rs, demo/src/shader.wgsl
- .github/workflows/ci.yml
- justfile
- docs/DECISIONS.md (added ADR-0008)
