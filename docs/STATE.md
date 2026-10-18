# Kiln Session Handoff

**Current milestone:** M10 complete

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
- M6: kiln-editor — native desktop editor with egui and docking panels
  - ADR-0001 accepted: pure-Rust native (winit + wgpu + egui)
  - ADR-0004 accepted: egui 0.29 with egui-dock 0.14 (compatible with wgpu 22.1)
  - Event loop with winit ApplicationHandler
  - egui-winit integration for input handling
  - egui-wgpu rendering backend
  - egui-dock for tabbed panel layout (SceneView, Inspector, Assets, Console)
  - Basic UI rendering with CentralPanel
  - Compile passes with only dead code warnings (unused Tab variants)
- M7: kiln-editor — scene view with entity rendering and inspector
  - SceneView panel displays ECS entities as colored rectangles using egui painter
  - Entity list below canvas for selection
  - Inspector panel shows Transform (X, Y) and Sprite (R, G, B) for selected entity
  - Demo scene with 5 entities created on startup
  - Inspector edits now persist to ECS using DragValue widgets
  - Play-in-editor (PIE) mode toggle added with Play/Stop button
  - All 44 tests pass
- M8: kiln-graph — node graph model and compiler
  - graph.rs: Graph with nodes HashMap and connections Vec, add/remove methods
  - node.rs: Node types (Event, Constant, Arithmetic, Comparison, Branch, HostCall, Output)
  - pin.rs: Pin with direction (Input/Output), type (Integer, Boolean, Event), name
  - connection.rs: Connection with source_pin and target_pin IDs
  - compiler.rs: Graph → IR compiler with recursive traversal from Output node
  - Tests: constant (1), arithmetic (1), branch (1), graph matches script (1) — all pass
  - ADR-0004 updated: custom node graph UI using egui primitives (not egui-snarl)
- M8: kiln-editor — graph editor UI
  - GraphEditor tab added with buttons to create Constant, Add, and Output nodes
  - Nodes rendered as rectangles with titles using egui painter
  - Graph stored in EditorApp state
  - Test verifies graph compiles to same IR as equivalent script (push 10, push 5, add)
- Workspace: added kiln-graph, ron dependency for graph serialization
- All 47 tests pass (2 kiln-core, 5 kiln-render, 10 kiln-assets, 11 kiln-ir, 16 kiln-script, 4 kiln-graph)
- M9: kiln-render — render tier system and post-processing structure
  - ADR-0009 accepted: three-tier system (Low, Medium, High) with auto-selection
  - tier.rs: RenderTier enum with GPU-based auto-selection (integrated → Low, discrete → Medium/High)
  - post.rs: PostConfig with per-tier presets, PostFramebuffer with bloom mip chain and depth buffer
  - post.rs: PostPipeline structure for effect passes (stubbed, shader impl deferred)
  - renderer.rs: Integrated tier into Renderer with set_tier() method for runtime switching
  - kiln-editor: Added Settings tab with tier override UI (Low/Medium/High buttons)
  - kiln-editor: EditorApp tracks render_tier and applies changes at runtime
  - Tests: tier defaults (1), tier effects (3), post config (3) — all pass
- M10: One-click web export from editor
  - kiln-web: WASM export shell with wasm-bindgen (stub runtime, no full game loop yet)
  - kiln-cli: Added export command — builds WASM, runs wasm-bindgen, generates HTML shell
  - kiln-editor: Added "Export to Web" button in Settings tab (spawns CLI export in background thread)
  - Web export produces: index.html, kiln_web.js, kiln_web_bg.wasm (27 KB uncompressed)
  - WASM size: 27 KB (well under 1.5 MB budget; full game loop will increase size)
  - Requires: wasm32-unknown-unknown target, wasm-bindgen-cli installed
  - Export workflow: cargo run --bin kiln-cli -- export --output web-export
- All 51 tests pass (2 kiln-core, 5 kiln-render, 10 kiln-assets, 11 kiln-ir, 16 kiln-script, 4 kiln-graph, 3 kiln-render tier/post)

**In progress:**
- None

**Next steps:**
- M11: Audio + physics (feature-gated)
- M12: Installers + size/perf pass
- M13: First sample game + tutorial + docs

**Known issues:**
- M7 deferred features:
  - Undo/redo for inspector edits (complex borrowing issues with egui closures)
  - Script hot-reload (requires runtime integration with VM)
  - Camera controls (pan/zoom) for scene view canvas
- M8 deferred features:
  - Live value preview during PIE (requires VM integration with graph runtime)
  - Node dragging and pin connections in graph editor (static rendering only)
  - Graph serialization to disk (RON format ready but not implemented)
- M9 deferred features:
  - Actual shader implementation for post-processing effects (bloom, LUT, fog, outline)
  - Tier system and framebuffer structure complete; shader passes stubbed for future milestone
  - Frame budget measurement on reference hardware (deferred to M12)
- M10 deferred features:
  - Full game loop and wgpu integration in kiln-web (currently stub runtime only)
  - Browser input handling (keyboard/mouse via web-sys)
  - wasm-opt optimization and brotli compression for size measurement
  - Actual game content export (currently exports empty shell)
  - Live preview in editor (auto-reload on export)
- Scene view uses egui painter for rendering (not actual GPU SpriteRenderer)
  — Simpler implementation for M7; full GPU rendering can be added later with framebuffer
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
- cargo check --workspace (builds successfully with 1 warning)
- cargo test --workspace (all 51 tests pass)
- cargo run --bin kiln-cli -- export --output web-export (successful export)
- rustup target add wasm32-unknown-unknown (added WASM target)
- cargo install wasm-bindgen-cli (installed wasm-bindgen tool)

**Files touched (M10):**
- kiln-web/Cargo.toml (updated dependencies: wasm-bindgen, web-sys)
- kiln-web/src/lib.rs (stub WASM runtime with panic hook)
- kiln-cli/src/main.rs (added Export command with WASM build and HTML generation)
- kiln-editor/src/app.rs (added Export to Web button in Settings tab)
- docs/STATE.md (updated progress)
