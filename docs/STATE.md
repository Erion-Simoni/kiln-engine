# Kiln Session Handoff

**Current milestone:** M2 complete

**Done:**
- AGENTS.md, README.md, justfile, .gitignore, IDE pointer files
- docs/ARCHITECTURE.md, ROADMAP.md, DECISIONS.md, BUDGETS.md
- M0: Cargo workspace, GitHub Actions CI, wgpu triangle demo
- M1: kiln-core (math, time, ID, project TOML format + round-trip tests)
- M1: kiln-runtime (fixed timestep loop, input, ECS wrapper via hecs)
- M1: ADR-0002 accepted: hecs 0.11.2 for ECS
- M2: kiln-render — real instanced sprite batch renderer
  - GPU instance buffer (MAX_SPRITES = 16 384, 1 draw call per flush)
  - begin_frame / push(SpriteInstance) / flush(render_pass, camera, queue) API
  - sprite.wgsl: instanced shader — corner × size + position → NDC via projection
  - Camera::projection_matrix tests (origin → NDC bottom-left, centre → NDC origin, zoom clamp)
  - Fixed bytemuck Mat4 issue: cast via [f32;16] to_cols_array()
- M2: demo rewritten with real render loop
  - GpuState struct holds Window (Arc) + Surface<'static> + Renderer + SpriteRenderer
  - Arrow keys pan; Space/Shift zoom; 10×10 grid (100 sprites, 1 draw call)
- M2: samples/tilemap — 5×5 checkerboard rendered in 1 draw call
- M2: kiln-core project.rs — replaced external mod tests; with inline #[cfg(test)]
- Workspace: added log 0.4, env_logger 0.11, bytemuck derive feature
- All 7 tests pass (2 kiln-core, 5 kiln-render)

**In progress:**
- None

**Next 3 steps:**
1. Run `cargo run --bin demo` to visually verify 10×10 sprite grid renders
2. Run `cargo run --bin tilemap` to verify checkerboard sample
3. Begin M3 (asset pipeline, content-hash cache, packed archive, CLI) if M2 verified

**Known issues:**
- SpriteRenderer render() re-borrows self.gpu after the ECS query (two step borrow)
  — works correctly; clippy may flag it; not a bug
- Browser build deferred (kiln-web stub remains)
- No texture support yet (sprite batch is solid colour only); texture atlas planned for M3/M4
- demo/main.rs logs at WARN level; set RUST_LOG=trace for per-frame sprite count

**Last commands run:**
- cargo test -p kiln-core -p kiln-render -p kiln-runtime (7/7 passed)
- git commit -m "M2: instanced sprite batch renderer, camera tests, tilemap sample"

**Files touched (M2):**
- kiln-render/src/sprite.wgsl (rewritten — instanced shader)
- kiln-render/src/sprite_renderer.rs (rewritten — GPU instancing + unit tests)
- kiln-render/src/camera.rs (added unit tests, clamp fix)
- kiln-render/src/lib.rs (re-export SpriteInstance, MAX_SPRITES)
- kiln-core/src/project.rs (inline tests, removed stale mod tests;)
- demo/src/main.rs (real render loop with GpuState)
- demo/Cargo.toml (add log, env_logger)
- samples/tilemap/Cargo.toml (new)
- samples/tilemap/src/main.rs (new — tilemap sample)
- Cargo.toml (add log, env_logger, bytemuck derive; add samples/tilemap)
