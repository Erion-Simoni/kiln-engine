# Kiln Session Handoff

**Current milestone:** M28 complete (Render graph + GPU profiling)

**Done:**
- AGENTS.md, README.md, justfile, .gitignore, IDE pointer files
- docs/ARCHITECTURE.md, ROADMAP.md, DECISIONS.md, BUDGETS.md
- M0–M27: see prior session history
- M28: Render graph + GPU profiling
  - ADR-0025 accepted: owned frame graph (no Bevy); TIMESTAMP_QUERY(+INSIDE_ENCODERS); pass enables from data
  - kiln-render: `RenderGraph` / `CompiledGraph` with lifetimes + transient aliasing
  - kiln-render: `PassEnableConfig` (TOML) — tiers enable passes from data, not code
  - kiln-render: `Frame2dGraph` ports M2 main + M9 post stubs through the graph
  - kiln-render: `GpuProfiler` per-pass timings; `Renderer` requests features when available
  - kiln-render: screenshot regression (`GOLDEN_LOW_64_SHA256`, sha2 of RGBA readback)
  - kiln-editor: Settings → GPU Pass Timings; frame runs graph then egui with timestamps
  - samples/render_graph: compile orders, data disable, screenshot hash, live µs
  - BUDGETS.md: sample timings recorded; ROADMAP_FULL M28 acceptance checked
  - Tests: kiln-render lib 32 passed (graph, pass_config, frame, screenshot, profiler)

**In progress:**
- None

**Next steps:**
- M29: 3D core (glTF, depth, perspective camera, frustum culling)
- See ROADMAP_FULL_M14_PLUS.md for detailed plan through M61

**Known issues:**
- M28 gaps:
  - Post passes (bloom/fog/outline/lut) are graph-scheduled stubs (Load/clear); full shaders still deferred (ADR-0009)
  - Transient aliasing assigns slots; GPU texture pool reuse not yet bound to physical wgpu textures at execute time
  - Screenshot golden is machine-validated on Windows; WebGL/old GPUs may lack TIMESTAMP_QUERY_INSIDE_ENCODERS
  - Editor scene view still clears via graph main, not live sprite batch through the graph
- M27 gaps:
  - hecs World systems still run with exclusive `&mut World` (no concurrent column writes); parallelism is TaskPool jobs + conflict-free stage planning
  - Web Workers path is a blob Worker ping proving COI gating, not a full SharedArrayBuffer atomics/WASM thread pool
  - kiln-web `threads` feature is documentary; Workers path always compiled with auto fallback
  - Speedup measured on one Windows 4-core machine; re-measure on reference Low/Medium hardware
  - Static file open of index.html cannot set COOP/COEP; needs a real server (documented)
- M26–M14 gaps: see prior STATE / ROADMAP_FULL (iOS device testing, gameplay runtime wiring, etc.)

**Blocked:**
- None for M28
