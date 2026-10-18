# Kiln Session Handoff

**Current milestone:** M6 in progress

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
- M4: kiln-ir — bytecode VM with host call interface
  - opcode.rs: 21 opcodes (stack, arith, comparison, control flow, host calls)
  - value.rs: Value enum (I64, Bool) with type-safe accessors
  - program.rs: Program with named labels and instruction sequence
  - vm.rs: Stack-based VM with HostContext trait for external functions
  - Hot path documented as allocation-free when stack stays within capacity
  - Tests: arith (5), branch (3), host call (1), comparison (2) — all pass
- M5: kiln-script — KilnScript frontend with diagnostics
  - ADR-0003 accepted: custom KilnScript language (no Rhai for size budget)
  - diagnostic.rs: Span and Diagnostic types for error reporting
  - lexer.rs: Token types (literals, keywords, operators, punctuation) with span tracking
  - parser.rs: Recursive descent parser with AST (Expr, Stmt) with span info
  - compiler.rs: AST → IR bytecode compiler with jump label fix-up for if statements
  - lib.rs: LanguageFrontend trait and KilnScriptFrontend implementation
  - Tests: lexer (6), parser (4), compiler (3), frontend (3) — all pass
  - Sample script test: if statement with arithmetic and host call compiles and runs
  - Parse/type errors return spanned diagnostics for editor squiggles
- Workspace: added kiln-script
- All 44 tests pass (2 kiln-core, 5 kiln-render, 10 kiln-assets, 11 kiln-ir, 16 kiln-script)

**In progress:**
- M6: kiln-editor — stub crate created
  - ADR-0001 accepted: pure-Rust native (winit + wgpu + egui)
  - ADR-0004 accepted: egui 0.36.x pinned for egui-dock compatibility
  - Basic EditorApp struct with window reference
  - wgpu version conflict resolved (stayed at 22.1 for workspace consistency)
  - Need to: implement full event loop, egui integration, docking panels

**Next 3 steps:**
1. Complete M6: implement basic app window and event loop with egui
2. Add docking panels using egui-dock
3. Implement scene view (framebuffer rendering)

**Known issues:**
- SpriteRenderer render() re-borrows self.gpu after the ECS query (two step borrow)
  — works correctly; clippy may flag it; not a bug
- Browser build deferred (kiln-web stub remains)
- No texture support yet (sprite batch is solid colour only); texture atlas planned for M6/M7
- Archive format uses full file paths for keys; may need canonicalization for cross-platform
- CLI pack command only packs immediate children of source dir (no recursive import)
- VM has no float type yet (only i64 and bool)
- VM has no string type yet
- VM has no arrays or collections yet
- KilnScript has no variable support yet (identifiers are only for function calls)
- KilnScript has no while/for loops yet
- KilnScript has no function definitions yet

**Last commands run:**
- cargo check -p kiln-editor (compiles with stub implementation)
- No commit yet (M6 in progress)

**Files touched (M6 so far):**
- kiln-editor/Cargo.toml (new crate with kiln-core/render/runtime/assets)
- kiln-editor/src/lib.rs (module exports)
- kiln-editor/src/app.rs (EditorApp stub with window reference)
- kiln-editor/src/panels.rs (stub for editor panels)
- docs/DECISIONS.md (ADR-0001 and ADR-0004 accepted)
