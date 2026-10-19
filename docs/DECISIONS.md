# Kiln Architecture Decision Records

Date format: YYYY-MM-DD. Status: proposed | accepted | rejected | deprecated.

---

## ADR-0001: Editor UI framework (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use pure-Rust native (`winit` + `wgpu` + `egui`) for the editor. Do not use Electron. Tauri only if justified by a future ADR.

**Alternatives:**
- Electron: large install size, heavy resource usage
- Tauri: web tech overhead, adds complexity
- Pure Rust: smallest binary, best performance, single toolchain

**Consequences:**
- Editor installer stays under 40 MB target
- Native UI may have less polish than mature web frameworks initially
- Need to evaluate node-graph UI (egui-snarl vs custom) in M8
- Size impact measured in M6: egui + egui-dock minimal overhead

---

## ADR-0002: ECS choice (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use `hecs 0.11.2` for ECS. Smaller binary, faster compile time, simpler API sufficient for Kiln's needs.

**Alternatives:**
- bevy_ecs: feature-rich, larger binary, slower compile
- hecs: simpler, smaller, faster compile (chosen)
- Custom: minimal, but maintenance burden

**Consequences:**
- hecs added to workspace dependencies
- Minimal binary impact for web target
- Fast compile times for editor development

---

## ADR-0003: Scripting language (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Build a minimal custom **KilnScript** language for M5. Simple expression language targeting our IR directly, keeping binary size minimal and semantics tightly coupled to the VM.

**Alternatives:**
- Rhai: fast to integrate, mature, but large binary footprint (rejected for size budget)
- Custom KilnScript: optimized for engine, smaller binary, semantics match VM (chosen)
- Other (Lua, Wren, etc): evaluation deferred

**Consequences:**
- Minimal binary impact (no external scripting runtime)
- Language designed to map 1:1 to our IR opcodes
- Simpler diagnostics (full control over lexer/parser)
- Trait `LanguageFrontend` allows swapping if needed later
- Size impact measured in M5: ~0 KB beyond kiln-ir dependency

---

## ADR-0004: egui version and node-graph UI (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use **egui 0.29** with **egui-dock 0.14** for M6–M7. For M8 node-graph, build a **simple custom node graph UI** using egui primitives rather than egui-snarl (which requires egui 0.36+). This keeps the editor on a stable egui version while we build minimal node-graph functionality.

**Alternatives:**
- egui-snarl 0.12.0: requires egui 0.36+, would force version upgrade
- Custom node graph: more code, but full control and no version lock
- Upgrade to egui 0.36: risks breaking other UI, delays M6/M7

**Consequences:**
- Custom node graph for M8: simple nodes, wires, pins using egui painting
- Keep egui 0.29 for now; evaluate upgrade for advanced features later
- Node graph model decoupled from UI for easy UI replacement
- Graph data model (nodes, connections, pins) serializable to RON

---

## ADR-0005: Project file format (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Use **RON** or **TOML** for manifests and scene data. Discourage YAML for merge noise.

**Alternatives:**
- RON: Rust-native, type-safe, good merge behavior
- TOML: widely understood, good for configs
- JSON: verbose, no comments
- YAML: merge noise, complex spec

**Consequences:**
- Git-friendly project files
- Stable UUIDs for entities and asset refs
- One entity/resource per file where practical

---

## ADR-0006: Asset compression formats (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Textures → KTX2/Basis Universal; meshes → meshopt; audio → OGG/Opus. Archive compression: zstd for native, brotli for web (measured).

**Alternatives:**
- PNG/JPEG: larger, no GPU-ready formats
- WAV: uncompressed audio
- Other compressors: gzip, lz4

**Consequences:**
- GPU-ready formats for fast load
- Content-hash cache for incremental imports
- Web size measured against <1.5 MB budget

---

## ADR-0007: Plugin system (proposed)

**Date:** 2026-10-18
**Status:** proposed

**Decision:** Rust plugins compiled to **WASM**, loaded via `kiln-plugin-api`, sandboxed. No native `cdylib` in default product path.

**Alternatives:**
- WASM: sandboxed, safe, portable
- Native cdylib: faster, but unsafe, larger attack surface
- No plugins: simpler, but less extensible

**Consequences:**
- M14+ feature; not in initial product
- Extension kinds: custom nodes, importers, editor panels, host calls
- Capability-gated host calls for security

---

## ADR-0008: Async runtime for demo (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use `pollster 0.4` for blocking async in demo binary. Minimal dependency, no runtime overhead.

**Alternatives:**
- pollster: simple, blocking, no runtime
- tokio/async-std: heavy, overkill for demo
- Custom executor: unnecessary complexity

**Consequences:**
- Small binary impact
- Demo uses blocking async for simplicity
- M1 will evaluate proper async runtime for engine

---

## ADR-0009: Render tier system and post-processing (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Implement a three-tier render quality system (Low, Medium, High) with auto-selection based on GPU capabilities. Post-processing effects (bloom, LUT, fog, outline) are tier-gated and degrade gracefully on lower-end hardware.

**Alternatives:**
- Single quality tier: fails on low-end hardware, poor user experience
- Many granular settings: complex UI, hard to test, user confusion
- Three-tier system: simple, testable, auto-selects, manual override available

**Consequences:**
- Auto-selection heuristic: integrated GPUs → Low, discrete GPUs → Medium/High
- Editor UI provides tier override in Settings panel
- Post-processing framebuffer structure supports bloom mip chain, depth buffer
- Effect cost metadata guides tier-based degradation
- Shader implementation deferred to future milestone (tier system complete)

**Effect breakdown by tier:**
- Low: No post-processing, baked lighting, minimal effects
- Medium: Bloom (3 passes), screen-space fog, soft shadows
- High: Bloom (5 passes), LUT color grading, fog, outline, real-time lighting

---

## ADR-0010: Audio crate choice (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use **rodio 0.23** with **symphonia** for audio playback. Rodio provides a simple Source-based API, supports OGG/Opus via symphonia's optional opus feature, and is widely used in the Rust game dev ecosystem.

**Alternatives:**
- rodio + symphonia: mature, widely used (12M+ downloads), simple API, supports OGG/Opus (chosen)
- kira: more features (spatial audio, tweens), but larger dependency tree
- Pure opus decoders (opus-pure, opuscule): decode-only, no playback/OS integration
- Custom: minimal, but significant maintenance burden

**Consequences:**
- Audio feature-gated in kiln-runtime to exclude from web builds by default
- Rodio's symphonia feature enables OGG/Opus via `symphonia-all-codecs` and `symphonia-ogg`
- Binary size impact measured in M11 (expected ~200-500 KB uncompressed)
- Simple API: load file → decode → play via Sink
- cpal backend for OS audio output

---

## ADR-0011: Physics engine choice (accepted)

**Date:** 2026-10-18
**Status:** accepted

**Decision:** Use **rapier2d 0.36.1** for 2D physics. Rapier is a modern Rust physics engine with good performance, comprehensive features (colliders, joints, collision events), and active maintenance.

**Alternatives:**
- rapier2d: modern, fast, comprehensive 2D physics (chosen)
- nphysics2d: predecessor to rapier, deprecated
- custom: minimal collision detection only, maintenance burden
- box2d-rs: Box2D port, less actively maintained

**Consequences:**
- Physics feature-gated in kiln-runtime to exclude from web builds by default
- Binary size impact measured in M11 (expected ~500-800 KB uncompressed)
- Provides rigid body dynamics, collision detection, joints, queries
- 2D-specific version to avoid 3D overhead
- Integration point in kiln-runtime via PhysicsWorld wrapper

---

## ADR-0012: wgpu backend selection (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Use `wgpu::Backends::PRIMARY` (Vulkan, Metal, DX12, WebGPU) by default for best performance on modern hardware. Document that OpenGL/GLES fallback is available via `Backends::all()` or `Backends::GL` for old hardware testing, but is not enabled by default due to "best effort" support status in wgpu 22.1.

**Alternatives:**
- PRIMARY only: fastest on modern hardware, but no fallback on old GPUs (current choice)
- all() includes GL: fallback available, but GL backend is "best effort" and may have issues
- GL only: would work on old hardware, but misses modern GPU features

**Consequences:**
- Current code uses PRIMARY in kiln-editor, demo, and samples/collector
- OpenGL/GLES fallback exists in wgpu but is SECONDARY tier (best effort, not first-class)
- For old hardware testing, can change to `Backends::all()` or set `WGPU_BACKEND=gl` env var
- Document in README that OpenGL/GLES is available but not default
- wgpu 22.1 supports: Vulkan (Windows/Linux/Android), Metal (macOS/iOS), DX12 (Windows 10+), GL 3.3+/GLES 3.0+ (fallback), WebGPU/WebGL2 (web)

---

## ADR-0013: Asset loader strategy (proposed)

**Date:** 2026-10-19
**Status:** proposed

**Decision:** Implement two loaders for the same archive format: memory-mapped file (`memmap2` crate) for native platforms, and HTTP streaming with range requests for web (via `reqwest` or `web-sys`). This keeps the archive format unchanged while optimizing for each platform's capabilities.

**Alternatives:**
- Memory-mapped native + HTTP web: optimal for each platform (proposed)
- Regular file I/O for both: simpler, but slower for large assets
- HTTP for both: adds unnecessary network overhead on native

**Consequences:**
- Archive format (KILNARV1) remains unchanged
- Native games benefit from zero-copy memory mapping
- Web games use streaming with range requests to avoid loading entire archive
- Deferred to M14 or later (not in M0-M13)

---

## ADR-0014: Audio device management (proposed)

**Date:** 2026-10-19
**Status:** proposed

**Decision:** Use rodio's cpal backend directly for device enumeration, selection, hot-plug detection, and sample-rate changes. The current M11 implementation is a minimal stub that only opens the default sink; full device management will be added in a future milestone.

**Alternatives:**
- rodio + cpal direct access: full control over devices (proposed)
- rodio high-level API only: simpler, but limited device control
- Custom audio backend: more control, but significant maintenance burden

**Consequences:**
- Current M11 implementation uses rodio's high-level `DeviceSinkBuilder::open_default_sink()`
- Device selection, hot-plug, and sample-rate changes are NOT implemented in M0-M13
- Deferred to future milestone (M49 audio v2 or earlier if needed)
- R0 audit documents this gap

---

## ADR-0015: Window settings in project (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Add `WindowSettings` struct to `kiln-core::project` with fields for width, height, fullscreen, borderless, vsync, high_dpi, and resizable. Settings are stored in project TOML and have sensible defaults. Window creation code must read these settings when initializing winit Window (deferred to future milestone).

**Alternatives:**
- Store in project TOML: user-configurable, version-controlled (chosen)
- Store in separate settings file: more flexible, but harder to manage
- Hardcode in binary: no flexibility for users

**Consequences:**
- WindowSettings struct added to project.rs with defaults (1280x720, vsync=true, high_dpi=true)
- Structure is part of Project TOML format
- Window creation code (demo, samples, editor) does NOT yet apply these settings
- Deferred: actual application of settings to winit WindowBuilder to M14 or later
- R0 audit documents that structure exists but is not yet used

---

## ADR-0016: R0 Retrofit audit findings (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Document R0 audit findings for M0-M13. All checks performed; gaps identified and recorded in STATE.md.

**Audit findings:**

1. **wgpu backend support**: Changed from `Backends::PRIMARY` to `Backends::all()` to enable OpenGL/GLES fallback for old hardware. wgpu 22.1 supports Vulkan, Metal, DX12 (PRIMARY) and GL 3.3+/GLES 3.0+ (SECONDARY). Documented in ADR-0012.

2. **Asset loaders**: Archive format (KILNARV1) exists but has only regular file I/O loader. Memory-mapped loader for native and HTTP streaming for web are NOT implemented. Deferred to M14. Documented in ADR-0013.

3. **Audio device management**: M11 implementation is a minimal stub using rodio's high-level API. Device selection, hot-plug detection, and sample-rate changes are NOT implemented. Deferred to M49 audio v2. Documented in ADR-0014.

4. **Native player size budget**: Added to BUDGETS.md with target <15 MB (executable without assets). Cannot measure yet (kiln-player does not exist; will be created in M14).

5. **Startup budget**: Added to BUDGETS.md with target <2 seconds to first frame. Cannot measure yet (requires player binary and proper asset loading).

6. **Window options in project settings**: Added WindowSettings struct to project.rs with fields for width, height, fullscreen, borderless, vsync, high_dpi, resizable. Structure exists but is NOT yet applied to winit WindowBuilder. Deferred to M14 or later. Documented in ADR-0015.

7. **M13 sample "N minutes" placeholder**: Replaced with "≤2 minutes" in ROADMAP.md. TUTORIAL.md already states "1-2 minutes" for Coin Collector game.

8. **M0-M13 checkboxes**: Updated ROADMAP.md to mark completed items. Deferred items documented in STATE.md Known issues section.

**Gaps identified (deferred features):**
- M7: undo/redo for inspector edits
- M7: script hot-reload
- M7: camera controls for scene view
- M8: live value preview during PIE
- M8: node dragging and pin connections in graph editor
- M8: graph serialization to disk
- M9: actual shader implementation for post-processing effects
- M9: frame budget measurement on reference hardware
- M10: full game loop and wgpu integration in kiln-web
- M10: browser input handling
- M10: wasm-opt optimization and brotli compression
- M10: actual game content export
- M10: live preview in editor
- M11: audio device selection, hot-plug, sample-rate changes
- M11: physics sample visualization
- M12: wasm-opt and brotli installation
- M12: frame budget measurement on reference hardware
- M12: RAM usage measurement
- General: scene view uses egui painter (not GPU SpriteRenderer)
- General: no texture support yet
- General: archive format uses full file paths (may need canonicalization)
- General: CLI pack command non-recursive
- VM: no float, string, arrays, or collections
- KilnScript: no variables, loops, or function definitions

**Consequences:**
- All R0 acceptance items either completed or documented as deferred
- STATE.md will be updated to reflect R0 completion
- No breaking changes to existing M0-M13 functionality
- All deferred features are tracked in STATE.md Known issues
