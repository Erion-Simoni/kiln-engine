# Kiln Session Handoff

**Current milestone:** M3 complete

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
- M2: Fixed wgpu surface/instance ownership error (pass instance to Renderer::new)
- Workspace: added log 0.4, env_logger 0.11, bytemuck derive feature
- M3: kiln-assets — content-hash cache and packed archive
  - hash.rs: SHA-256 content hashing (3 tests)
  - cache.rs: AssetCache with JSON persistence, incremental import (4 tests)
  - archive.rs: Binary archive format (KILNARV1 magic, entry table, file data) (3 tests)
  - Import test proves re-import skips unchanged files via hash comparison
- M3: kiln-cli — asset pipeline CLI
  - Commands: import, pack, verify
  - clap-based CLI with derive features
  - Verified: import skips unchanged files, pack creates archive, verify lists contents
- M3: justfile assets recipe updated to use kiln-cli
- Workspace: added sha2, hex, serde_json, thiserror, walkdir, clap, anyhow
- All 17 tests pass (2 kiln-core, 5 kiln-render, 10 kiln-assets)

**In progress:**
- None

**Next 3 steps:**
1. Verify texture atlas can be integrated with sprite batch (M4 prep)
2. Begin M4 (IR + VM) if approved
3. Consider texture loading from archive for M2 samples

**Known issues:**
- SpriteRenderer render() re-borrows self.gpu after the ECS query (two step borrow)
  — works correctly; clippy may flag it; not a bug
- Browser build deferred (kiln-web stub remains)
- No texture support yet (sprite batch is solid colour only); texture atlas planned for M4
- Archive format uses full file paths for keys; may need canonicalization for cross-platform
- CLI pack command only packs immediate children of source dir (no recursive import)

**Last commands run:**
- cargo test -p kiln-core -p kiln-render -p kiln-runtime -p kiln-assets (17/17 passed)
- cargo run --bin kiln-cli -- import/pack/verify (all commands verified)
- git commit -m "Fix wgpu surface/instance ownership error"
- git commit -m "M3: asset pipeline, content-hash cache, packed archive, CLI"

**Files touched (M3):**
- kiln-assets/Cargo.toml (new crate with sha2, hex, serde_json, thiserror, walkdir)
- kiln-assets/src/lib.rs (module exports)
- kiln-assets/src/hash.rs (SHA-256 hashing + 3 tests)
- kiln-assets/src/cache.rs (AssetCache with JSON persistence + 4 tests)
- kiln-assets/src/archive.rs (Binary archive format KILNARV1 + 3 tests)
- kiln-cli/Cargo.toml (new CLI binary with clap, anyhow)
- kiln-cli/src/main.rs (import/pack/verify commands)
- justfile (assets recipe updated)
- Cargo.toml (add kiln-assets, kiln-cli; add workspace deps)
- assets/test.txt (test asset file)
- .kiln-cache/kiln-cache.json (generated cache)
- assets.kar (generated archive)
