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

## ADR-0013: Asset loader strategy (accepted)

**Date:** 2026-10-19
**Status:** accepted
**Implemented:** 2026-10-20

**Decision:** Implement two loaders for the same archive format: memory-mapped file (`memmap2` crate) for native platforms, and HTTP streaming with range requests for web (via `web-sys`). This keeps the archive format unchanged while optimizing for each platform's capabilities.

**Alternatives:**
- Memory-mapped native + HTTP web: optimal for each platform (chosen)
- Regular file I/O for both: simpler, but slower for large assets
- HTTP for both: adds unnecessary network overhead on native

**Consequences:**
- Archive format (KILNARV1/V2) remains unchanged
- Native games benefit from zero-copy memory mapping via `extract_mmap()` and `extract_mmap_slice()`
- Web games use streaming with range requests via `WebArchiveLoader::extract()` to avoid loading entire archive
- Added `memmap2` 0.9 dependency to `kiln-assets`
- Added web-sys features (Request, Response, Headers, ReadableStream) to `kiln-web`
- Web loader currently fetches full archive for metadata; TODO: optimize with HEAD + Content-Length to fetch only entry table
- All tests pass (14 tests in kiln-assets)

---

## ADR-0014: Audio device management (accepted - deferred)

**Date:** 2026-10-19
**Status:** accepted (deferred to M49)
**R0 audit:** 2026-10-20

**Decision:** Use rodio's cpal backend directly for device enumeration, selection, hot-plug detection, and sample-rate changes. The current M11 implementation is a minimal stub that only opens the default sink; full device management will be added in M49 audio v2.

**Alternatives:**
- rodio + cpal direct access: full control over devices (chosen for M49)
- rodio high-level API only: simpler, but limited device control (current M11 implementation)
- Custom audio backend: more control, but significant maintenance burden

**Consequences:**
- Current M11 implementation uses rodio's high-level `DeviceSinkBuilder::open_default_sink()`
- Device selection, hot-plug, and sample-rate changes are NOT implemented in M0-M13
- Deferred to M49 audio v2 (full device management not needed for basic 2D games)
- R0 audit documents this gap in TODO_LIST.md
- Current implementation is sufficient for basic audio playback (default device only)

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

---

## ADR-0017: Native player and export (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Create `kiln-player` crate as a thin launcher that loads games from packed archives and runs them. Use `dirs` crate for cross-platform save/settings/cache directories. Extend `kiln-cli export` to support native platforms (windows, macos, linux) in addition to web.

**Alternatives:**
- Separate player binary: minimal size, no editor code (chosen)
- Reuse editor binary: simpler, but includes unnecessary editor code
- Self-contained single binary: complex asset embedding

**Consequences:**
- kiln-player depends on kiln-core, kiln-runtime, kiln-assets, kiln-render (no editor crates)
- kiln-player uses dirs crate for platform-specific paths (AppData, Application Support, XDG)
- kiln-cli export command now accepts --platform flag (web, windows, macos, linux)
- Player binary size: 930 KB (well under 15 MB budget)
- Archive format (KILNARV1) used for game data
- Player is currently a stub (loads archive, creates directories, but doesn't run game loop)
- Full game loop implementation deferred to future milestone

---

## ADR-0018: Linux packaging format (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Use **AppImage** as the primary Linux packaging format for games. Provide **tar.gz** as a fallback for users who prefer manual extraction. Do not use Flatpak initially (deferred to later milestone if needed). AppImage is a single-file, distro-agnostic format that runs on most Linux distributions without requiring root or complex dependencies.

**Alternatives:**
- AppImage + tar.gz: simple, distro-agnostic, no root required (chosen)
- Flatpak: good sandboxing and centralized distribution, but requires Flathub setup and runtime installation
- Snap: requires Snap daemon and Ubuntu/Canonical infrastructure, not distro-agnostic
- deb/rpm: distro-specific, requires package management integration, more complex build process
- tar.gz only: simplest, but no desktop integration or icon handling

**Consequences:**
- AppImage is the primary format for Linux game exports
- tar.gz provided as fallback for manual installation
- Flatpak deferred to future milestone if community demand or store distribution requires it
- AppImage packaging uses `appimage-builder` or manual tools (cargo-dist may add support)
- Desktop integration (desktop file, icon) embedded in AppImage
- No Flatpak runtime overhead in initial implementation
- Simpler build process for early M15 milestone

---

## ADR-0019: Storefront distribution strategy (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Support **itch.io as the primary storefront** for initial distribution. Provide **Steam integration behind a Cargo feature flag** (`steam`) for developers who want Steamworks features (achievements, cloud saves, overlay). Use a **platform-services trait** to abstract storefront-specific functionality so games are not locked to a single store.

**Alternatives:**
- itch.io only: simpler, but misses Steam's large audience and features
- Steam only: $100 fee and complex approval process, not suitable for early indie projects
- Both platforms: chosen - itch.io for quick publishing, Steam for production features
- GOG/Epic: deferred to later milestone (requires separate approval processes)

**Consequences:**
- itch.io: Free to publish, uses butler CLI for uploads, 10% revenue share, no code signing required
- Steam: $100 one-time fee, Steamworks SDK integration, 30% revenue share, requires code signing
- Platform-services trait: Achievements, leaderboards, cloud saves, IAP abstracted behind trait
- Steamworks behind `steam` feature: Stub backend when feature is off, real Steamworks when on
- itch.io upload via `kiln-cli publish itch` (butler CLI integration or documented manual flow)
- Steam depot/build script generation via `kiln-cli publish steam`
- Documentation will cover fees, review requirements, and account setup for both platforms

---

## ADR-0020: Update strategy and delta patching (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Use **qbsdiff 1.4** for delta patch generation. Create `kiln-updater` crate behind an `updater` Cargo feature flag for in-game updates. Store builds rely on storefront updaters (itch.io butler, Steam) and compile out the in-game updater. Version data archives and support DLC packs as additional archives with version dependencies.

**Alternatives:**
- qbsdiff 1.4: fast, memory-saving, bsdiff 4.x compatible, MIT license (chosen)
- bsdiff: simpler API, BSD-2-Clause, but less efficient (rejected)
- bidiff: file-backed mmap, parallel, but more complex (rejected)
- bsdiff6: 20% smaller patches, but newer and less tested (rejected)
- Full re-download: simplest, but wastes bandwidth for small changes (rejected)

**Consequences:**
- kiln-updater crate created with `updater` feature flag (optional for size)
- Delta patches generated via `kiln-cli patch --old <old.kilnar> --new <new.kilnar> --output <patch.qbsdiff>`
- In-game updater: check for updates via HTTP, download patch, verify SHA-256 hash, apply patch, rollback on failure
- Store builds (itch.io, Steam) use storefront updaters; in-game updater compiles out with `updater` feature disabled
- Archive format upgraded to KILNARV2 with version field in header
- DLC packs are separate archives with `requires_version` field in metadata
- Crash-report hook added as opt-in stub (full implementation deferred to M56)
- Binary size impact: ~200-300 KB for qbsdiff + updater code (feature-gated)

---

## ADR-0021: Android export approach (accepted, implemented)

**Date:** 2026-10-19
**Status:** accepted, implemented

**Decision:** Use **winit's Android backend** (android-activity 0.6) for the NDK glue layer. Use **cargo-apk** for APK/AAB building. Build `kiln-player` as a `cdylib` for Android to keep the same player binary approach as desktop. Add touch input support via winit's touch events and handle Android lifecycle (suspend/resume, rotation) through winit's ApplicationHandler.

**Alternatives:**
- winit + cargo-apk: Native Android approach, minimal dependencies, uses existing winit (chosen)
- Tauri: Cross-platform mobile support, but adds WebView overhead and complexity
- Custom NDK glue: Full control, but significant maintenance burden
- Crossbow: Higher-level tooling, but adds dependency and complexity

**Consequences:**
- winit 0.30 already has android-activity 0.6 as a dependency
- kiln-player will have `crate-type = ["cdylib"]` for Android builds
- Android targets: aarch64-linux-android (arm64), armv7-linux-androideabi (32-bit ARM)
- cargo-apk will handle APK/AAB generation with Gradle project
- Touch input exposed through winit's WindowEvent::Touch events
- Surface creation on resume, surface cleanup on suspend (per wgpu Android docs)
- Android lifecycle handled via ApplicationHandler::resumed/suspended
- APK size budget: <20 MB (includes player binary + assets)
- AAB (Android App Bundle) for Play Store release with CI signing
- Same kiln-player code runs on desktop and Android (feature-gated Android code)

**Implementation (M18):**
- Added cargo-apk configuration to kiln-player/Cargo.toml
- Implemented full Android player in kiln-player/src/android.rs with wgpu, game loop, and demo scene
- Touch input integrated via kiln-runtime InputState (already had touch support)
- Added `just android` recipe for APK building
- Extended kiln-cli export command to support Android platform
- Created docs/ANDROID.md with comprehensive build and deployment guide
- APK builds successfully; signed AAB process documented (manual keystore configuration required)

---

## ADR-0022: 2D skeletal animation approach (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Use a **custom simple format for spritesheet clips** and basic 2D animation. Do not implement full skeletal animation in M19. The format will store:
- Spritesheet texture reference
- Frame rectangles (x, y, width, height) with origin pivot
- Frame duration per frame
- Frame events (optional, e.g., "footstep", "hit")
- Looping flag

Full skeletal animation (bones, IK, blend trees) is deferred to M33 (3D skeletal animation) or a future M19-b milestone. Import support for Spine/DragonBones is deferred to M51 (asset pipeline v2) or later if needed.

**Alternatives:**
- Custom spritesheet format: Simple, minimal dependencies, full control (chosen for M19)
- Spine import: Powerful, but requires Spine runtime license, complex to parse
- DragonBones import: Open source, but complex JSON format, heavy parsing
- Full skeletal animation in M19: Too complex for single milestone, deferred

**Consequences:**
- M19 implements simple spritesheet clips with frame events
- Format is text-based (RON or JSON) for easy editing and version control
- Animation state machine (ASM) can drive clip selection via graph nodes
- Tween library (ease curves) works for property interpolation in scripts and graphs
- Full skeletal animation (bones, IK, skinning) deferred to M33 (3D)
- Import support for commercial formats deferred to M51 or later
- Binary size impact: Minimal (simple data structures, no heavy parsing libraries)

---

## ADR-0023: iOS export approach (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Use **winit's iOS backend** for the UIKit integration. Build `kiln-player` as a **static library** (`staticlib`) that exports a C ABI entry point `start_winit_app()`. Use **Xcode** for packaging, signing, and App Store submission. The iOS build requires macOS, Xcode, and an Apple Developer account. Extend `kiln-cli export` to support iOS platform with `.app` bundle generation and documentation for Xcode project setup.

**Alternatives:**
- winit + Xcode staticlib: Native iOS approach, follows Rust best practices, uses existing winit (chosen)
- Tauri: Cross-platform mobile support, but adds WebView overhead and complexity
- cargo-apk alternative for iOS: No mature cargo-apk equivalent for iOS exists
- Crossbow: Higher-level tooling, but adds dependency and complexity
- Custom NDK glue: Full control, but significant maintenance burden

**Consequences:**
- winit 0.30 has iOS platform support (UIKit integration via winit::platform::ios)
- kiln-player will have `crate-type = ["staticlib"]` for iOS builds
- iOS targets: aarch64-apple-ios (device), aarch64-apple-ios-sim (simulator on Apple Silicon)
- Xcode project required for signing, provisioning profiles, and App Store submission
- Static library approach: `#[no_mangle] pub extern fn start_winit_app()` called from Objective-C main
- iOS lifecycle handled via winit's ApplicationHandler (resumed/suspended events)
- Surface creation on resume, surface cleanup on suspend (per wgpu iOS docs)
- iOS-specific project settings: permissions, privacy declarations, icons, splash screen, orientation, notch safe areas
- Device-tier detection: Low/Medium/High based on device capabilities (GPU, memory, CPU)
- App Store build validation: Info.plist, entitlements, signing, bundle identifier
- In-app purchases and ads NOT built in; exposed via platform-services trait (M54)
- iOS size budget: <25 MB (includes player binary + assets, stricter than Android due to App Store)
- Build requirements: macOS with Xcode 12+, Apple Developer account for device testing and App Store
- Same kiln-player code runs on desktop, Android, and iOS (platform-specific modules feature-gated)
- CI/CD: iOS builds require macOS runner in GitHub Actions (deferred to later milestone)

**Implementation notes:**
- iOS requires Xcode for actual device testing and App Store submission
- Windows/Linux cannot build iOS binaries directly (requires macOS cross-compilation or macOS runner)
- M26 will implement the infrastructure and document the Mac-specific requirements
- Actual device testing deferred to Mac user or CI with macOS runner
- Build scripts and Xcode project templates will be provided for easy setup

---

## ADR-0024: Job system and multithreading (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Implement a thin custom `TaskPool` on `std::thread` (native) with no rayon. Parallel ECS uses declared `AccessSet` (Read/Write by `TypeId`) in `kiln-core`; `Schedule` in `kiln-runtime` builds conflict-free stages. Feature `threads` enables parallel stage job execution and the worker pool; when off, the same APIs run serially (web size unchanged). Web: use Web Workers only when `crossOriginIsolated` is true (requires COOP/COEP headers); otherwise automatic single-thread fallback.

**Alternatives:**
- rayon / hecs `parallel`: mature, but heavier binary and forces rayon into the graph
- bevy_ecs schedule: rich, too large for Kiln budgets (rejected by ADR-0002)
- Always-on threads: breaks web size and non-isolated pages

**Consequences:**
- No new crates; `std::thread` + channels only on native
- `AccessSet` conflict tests run without the `threads` feature
- kiln-web detects isolation and exercises a Worker ping path or sync fallback
- Export HTML documents COOP/COEP; writes `_headers` for static hosts
- Measured 4+ core speedup recorded in BUDGETS.md
- wasm32 never uses `std::thread`; Workers path is isolation-gated

---

## ADR-0025: Render graph + GPU profiling (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Own a lightweight frame graph in `kiln-render` (no Bevy/rendergraph crate). Passes declare named reads/writes; compile topologically sorts, computes transient lifetimes, and aliases non-overlapping textures into shared physical slots. GPU cost uses wgpu `TIMESTAMP_QUERY` when the adapter supports it; otherwise timings are empty and the editor shows unsupported. Per-tier pass enablement comes from data (`PassEnableConfig`, TOML/RON), not hard-coded `if tier` branches in pass registration.

**Alternatives:**
- Hand-ordered encoder passes: simple, but blocks 3D (M29+) and aliasing
- Vendored Bevy render graph: rich, too heavy for Kiln size budgets
- Always-on GPU timestamps: breaks adapters without the feature; web/GL often lack it

**Consequences:**
- No new runtime crates; `serde` + `toml` only for pass config deserialize
- Screenshot regression hashes RGBA readback (sha2); no `image` crate
- Editor Settings shows per-pass GPU µs when queries resolve
- Default 2D graph ports M2 (sprites/main) + M9 (tier-gated post stubs) through the graph

---

## ADR-0027: PNG decoding via the `png` crate (accepted)

**Date:** 2026-10-20
**Status:** accepted

**Decision:** Decode PNG assets to RGBA8 on the CPU using the `png` crate (0.18),
in a new `kiln-assets::image` module. Decoding stays in `kiln-assets`; GPU upload
stays in `kiln-render` (`create_rgba8_texture` / `SpriteRenderer::create_texture_rgba`).
The two are joined at the player/editor call site.

**Alternatives:**
- `image` crate: broad format support, but pulls in far more dependencies and
  image processing we do not need for a 2D engine.
- Hand-rolled PNG decoder: no dependency cost, but zlib inflate alone is a large
  amount of subtle code to get right. Not worth it.
- Pre-baking textures to a custom format at import time: plausible later for
  shipping (and still worth it for size budgets), but the source PNG must be
  decodable anyway, so the `png` dependency does not go away.

**Consequences:**
- New dependency `png = "0.18"` in `kiln-assets` (see `Cargo.toml`).
  Transitive deps are small (`flate2`, `crc32fast`, `bitflags`, `fdeflate`, `miniz_oxide`).
- Only PNG is supported for now. JPEG/WebP are out of scope for the 2D gate.
- Decoded images are plain `Vec<u8>` RGBA8 + dimensions; no GPU types in
  `kiln-assets`, so it stays usable on WASM and in tests without a device.
- Alpha is preserved; PNG colour types 0/2/3/4/6 are all handled by the crate,
  and 16-bit inputs are downconverted to 8-bit per channel.

---

## ADR-0026: File length soft limit + AI exception (accepted)

**Date:** 2026-10-19
**Status:** accepted

**Decision:** Keep **prefer under 400 lines per file** as a soft guideline for
clarity. **AI agents may exceed 400 lines** when a split would be artificial,
duplicate glue, or slow a milestone. Prefer a natural module boundary when one
exists; never refuse needed work only to stay under 400. Humans may still ask
for a split in review.

**Alternatives:**
- Hard 400-line cap for everyone: keeps files small, but causes brittle splits
  and unfinished milestones when a coherent module is larger
- No guideline: files grow without pressure to modularize

**Consequences:**
- `AGENTS.md` coding rule 4 updated to match
- Existing large files (`kiln-editor/src/app.rs`, `kiln-cli/src/main.rs`, etc.)
  are acceptable until a natural split is warranted
- Splits still preferred when a clear submodule boundary appears
