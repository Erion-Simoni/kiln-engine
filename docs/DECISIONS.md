# Kiln Architecture Decision Records

Date format: YYYY-MM-DD. Status: proposed | accepted | rejected | deprecated.

**Current package-size policy:** ADR-0062 supersedes earlier size ceilings and
size-based release/completion gates, at the project owner’s request. Historical
measurements and earlier rationale are retained; sizes are informational.

---

## ADR-0028: Sprite texture paths are project-relative, `/`-separated (accepted)

**Date:** 2026-10-20
**Status:** accepted

**Decision:** `kiln_core::Component::Sprite` carries an optional `texture`
field holding an **asset path relative to the project root**, always
normalised to `/` separators (e.g. `"sprites/hero.png"`, never
`"sprites\\hero.png"`). The path is resolved against the assets the player
loaded from the project directory or `.kar` archive.

The player resolves a path by exact match first, then falls back to matching
the **file name only** (`hero.png`), because `.kar` archives historically
store entries by file name rather than by full path.

**Alternatives:**
- Store absolute paths: breaks portability and archive packaging.
- Store only file names: ambiguous as soon as two folders contain the same
  file name.
- Store asset handles/UUIDs: needs a manifest and an import step, which the
  2D gate does not have yet. Can be layered on later without changing the
  on-disk format of the scene files.

**Consequences:**
- Project directories are loaded **recursively**, so nested assets
  (`sprites/hero.png`) are reachable.
- A missing texture is not fatal: the player logs a warning and falls back
  to the built-in white tinted quad, so the game still runs.
- The editor asset browser lists project-relative paths with `/` separators,
  which is also what it shows in previews.
- Path normalisation must happen on every platform; Windows `\` separators
  are converted on load.

---

## ADR-0029: Editor previews are CPU-side `egui` images (accepted)

**Date:** 2026-10-20
**Status:** accepted

**Decision:** The editor asset browser decodes a PNG on the CPU and shows it
as an `egui::ColorImage` built with `ColorImage::from_rgba_unmultiplied`,
rather than uploading it to the editor's GPU as a `TextureId`.

**Alternatives:**
- Upload to the editor GPU: reuses the player path, but the editor does not
  yet have a live texture cache, and a preview texture would need cache
  invalidation when the file changes on disk.
- `from_rgba` (premultiplied): would darken the edges of images with
  transparency, since the decoder produces straight (unpremultiplied) alpha.

**Consequences:**
- Previews work without a GPU and cost only a CPU decode.
- The decoded pixels are straight RGBA8, so `from_rgba_unmultiplied` is the
  correct conversion.
- When the editor gains a real GPU scene view (G-6), the same decoded image
  can be uploaded there; the decode step is shared.

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

---

## ADR-0030: Script-driven vertical slice with a shared scene spawner (accepted)

**Date:** 2026-10-21
**Status:** accepted

**Decision:** The vertical slice (G-4) is a real Kiln project in
`samples/vertical_slice/` — a manifest, PNG art on disk and a KilnScript
holding the game rules — rather than a hard-coded Rust sample. Three engine
changes make that work:

1. **Scenes can name a script.** `kiln_core::project::Scene` gains a
   `script: Option<String>` field (path relative to the project root), and
   entities can declare a gameplay `Component::Role { role }`.
2. **One scene spawner for both hosts.** `kiln_runtime::scene` turns project
   data into runtime components. The player binds GPU textures through a
   sprite factory callback; the editor draws the textures with `egui`.
3. **A script-driven game loop in the runtime.** `kiln_runtime::slice` runs a
   KilnScript once per frame. The script reads *facts* (input axis, frame
   length, score, nearby pickups) and emits *commands* (move, collect, score,
   status); the runtime applies the commands to the world. The script never
   touches the ECS world directly, so it stays safe to run and easy to test.

The VM's host call gains an argument count (`Instruction::HostCall { id,
argc }`), so a call consumes only its own arguments and calls compose. The
script language gains `let` bindings, assignment, unary minus and line
comments, which a game needs to keep state.

**Alternatives:**
- Hard-coded Rust sample: fast to write, but proves nothing about the
  engine — the sample would be the game, not the engine running a game.
- Script drives the ECS world directly (queries, spawns, despawns): more
  powerful, but the script would need a mutable world borrow, which
  conflicts with the VM's own borrow, and every script becomes untestable
  without a world.
- One script per entity rather than per scene: finer grained, but the game
  rules of a slice belong together; per-entity scripts can come later.

**Consequences:**
- `kiln-runtime` depends on `kiln-ir` and `kiln-script`; `kiln-player` and
  `kiln-editor` depend on `kiln-runtime` only.
- The HUD is drawn in screen space with the top-left camera
  (`Camera::new_top_left`) and the built-in bitmap font, so no font assets
  are needed.
- Game state is a plain serialisable struct written to a save slot through
  `kiln-save`, so the player (F5/F9) and the editor share one format.
- The script API is a fixed list of builtins (`kiln_runtime::slice::BUILTINS`);
  growing it means adding an id and a host arm together.

---

## ADR-0031: Local variables in KilnScript (accepted)

**Date:** 2026-10-21
**Status:** accepted

**Decision:** KilnScript supports local variables: `let name = expr;` declares
one, `name = expr;` assigns to it, and a name reads its value. Slots are
allocated per program and stored in the VM (`GetLocal` / `SetLocal`), reset
when a program starts running.

**Alternatives:**
- Keep the language expression-only: a game script could not keep a score or
  a timer, so the rules could not live in the script.
- Store variables on the host: then the state of a script run would live in
  the host, and the same script could not be run twice with different state.

**Consequences:**
- `let` may not redeclare a name in the same scope; blocks (`if`) get their
  own scope.
- Variables live in the VM, so they are reset for every run of a program —
  a per-frame script has to keep its state in the host (the slice state).

---

## ADR-0032: The editor draws the scene with the engine (accepted)

**Date:** 2026-10-21
**Status:** accepted

**Decision:** The editor's scene view is drawn by the engine's sprite
renderer, not painted with egui. The panel's rectangle (in egui points) is
converted to physical pixels, letterboxed against the world's aspect ratio and
used as a viewport. The world is projected with the same camera the player
uses and the HUD is drawn over it in screen space. The project's textures are
uploaded to the GPU and bound to the sprites.

**Alternatives:**
- Keep painting the scene with egui: the editor and the player then show the
  same scene differently. The editor had no GPU textures (so the art was
  missing) and it flipped the vertical axis, so an entity was not where the
  player put it.
- Render the editor's scene into an offscreen texture and show it as an egui
  image: that needs a copy of the texture every frame and a second sampling
  path, for the same result.

**Consequences:**
- The editor and the player show the same pixels, so what the editor shows is
  what the game shows.
- The editor needs the project's textures on the GPU, so the textures are
  uploaded when a project is opened and bound to the sprites at spawn time.
- The editor's scene view has no post effects, because the frame graph still
  names the post passes without running the post chain. The player runs the
  real chain through `Renderer::apply_post`.
- The viewport math (points to pixels, letterboxing) lives in
  `kiln-editor/src/scene_view.rs` and is covered by unit tests.

---

## ADR-0033: Scene edits are undone from scene snapshots (accepted)

**Date:** 2026-10-21
**Status:** accepted

**Decision:** Undo and redo work on the scene. The editor keeps a stack of
scene snapshots, records one before an edit, and restores the scene when
undoing. The scene is written to disk when it is saved.

**Alternatives:**
- Undo each field edit separately: the inspector edits the world directly, so
  a field-level history would need a record per control and a way to apply it
  again, which is more code for the same result.
- Undo at the file level: the editor would have to write the project file
  after every change, which makes every edit slow and leaves the project file
  in states the user never asked for.

**Consequences:**
- One drag of a value is one undo step, because the scene is recorded when
  the drag starts.
- The scene data is the unit of undo, so a change that spans two components
  is undone as one step.
- The scene is saved from the world, so the world stays the source of truth
  while editing and the scene data is what is written to disk.

---

## ADR-0034: 3D types are feature-gated in `kiln-core` behind `3d` (accepted)

**Date:** 2026-10-24
**Status:** accepted

**Decision:** 3D mesh and material types (`Mesh`, `Material`, `Vertex`,
`SubMesh`, `MeshBounds`, `PrimitiveTopology`, `VertexFormat`, `AlphaMode`,
`Model`, `MeshHandle`) live in `kiln-core/src/mesh.rs` and are compiled only
when the `3d` Cargo feature is enabled (`3d = ["glam/serde"]`). The feature is
off by default, so a 2D-only build (including web) pulls in no 3D code and the
binary size is unchanged. The types are the shared data layer between the asset
pipeline (`kiln-assets`), the renderer (`kiln-render`) and the runtime
(`kiln-runtime`). When `3d` is off, the types still exist as transparent
stubs so the public API surface is stable.

**Alternatives:**
- Always compile 3D types: every 2D web/desktop build would pay for mesh and
  PBR material code it never uses, breaking the size budget.
- Put 3D types in a separate `kiln-3d` crate: adds a crate boundary and a new
  dependency graph for data that the 2D path also references (e.g. a future
  `Model` component); kept simple by gating inside `kiln-core` instead.
- Put 3D types directly in `kiln-render`: would couple the asset pipeline and
  runtime to the render crate; `kiln-core` is the right shared layer.

**Consequences:**
- M29 (3D core) can add glTF import, depth buffer, perspective camera and
  frustum culling behind `3d` without touching the 2D default build.
- The `3d`-off build-size check in M29's acceptance verifies the 2D web build
  is unchanged.
- Any new crate added for 3D (e.g. a glTF loader) still needs its own ADR
  entry (size, license, maintenance) per rule 5.

---

## ADR-0035: glTF 2.0 import approach for M29 (accepted)

**Date:** 2026-10-24
**Status:** accepted

**Decision:** M29 glTF 2.0 import will be implemented as a new feature-gated
loader crate (tentatively `kiln-gltf`, behind its own Cargo feature) rather
than baking a glTF dependency into `kiln-assets` or `kiln-core`. The loader
parses glTF JSON + binary buffers, extracts meshes/materials/textures/
hierarchy, and converts them into `kiln-core` mesh types (`Mesh`,
`Material`, `SubMesh`) behind the `3d` feature. meshopt compression is
supported via the `meshopt` crate (already in `Cargo.toml`).

**Rationale:** glTF is a large, complex format. Pulling `gltf` + dependencies
into `kiln-assets` unconditionally would bloat every 2D build and break the
web size budget. A separate feature-gated crate keeps the cost opt-in.

**Alternatives considered:**
- Embed glTF parsing in `kiln-assets`: rejected — unconditional size cost.
- Use `bevy_gltf` / `rapier_gltf`: rejected — heavy, pulls in game-engine
  and physics dependencies we don't need.
- Write a minimal custom glTF subset parser: accepted as the implementation
  approach within `kiln-gltf` (only the subset M29 needs: meshes, PBR
  materials, textures, node hierarchy).

**Consequences:**
- `kiln-gltf` needs its own dependency size justification in this ADR.
- The `meshopt` crate (already pinned at 0.7.0 in `Cargo.toml`) is reused
  for mesh compression — no new dependency.
- M29 acceptance includes "glTF 2.0 import (meshes, materials, textures,
  hierarchy) with meshopt."
- Any future extension of `kiln-gltf` beyond M29's subset needs a follow-up
  ADR.

---

## ADR-0036: TTF font parsing dependency for SDF atlas generation (accepted)

**Date:** 2026-11-01  
**Status:** accepted  

**Decision:** Add the `fontdue` crate (pure-Rust TTF/OTF parser and rasterizer)
as a dependency of `kiln-ui` for SDF font atlas generation. `fontdue` parses
TTF files, extracts glyph outlines, and rasterizes them to bitmap at a given
pixel size. Kiln then computes signed-distance fields from the bitmaps and
packs them into a texture atlas with `SdfFont` metadata. The `fontdue` crate
is feature-gated behind a `sdc`/`fonts` Cargo feature in `kiln-ui` so a
2D-only web build that doesn't use SDF fonts pays no size cost.

**Rationale:** TTF parsing is complex (glyph outlines, hinting, kerning,
subtables). Implementing it from scratch would be error-prone and
unmaintained. `fontdue` is a small, pure-Rust, MIT/Apache-2.0 licensed crate
with no C dependencies, making it safe for WASM/web builds.

**Alternatives considered:**
- Write a custom TTF parser: rejected — error-prone, large code surface,
  would need ongoing maintenance for hinting, variable fonts, etc.
- Use `ttf-parser`: considered — lower-level than `fontdue`, requires more
  manual rasterization work; `fontdue`'s built-in rasterization is a better
  fit for Kiln's needs.
- Use `ab_glyph` + custom rasterizer: considered — more crates, more code;
  `fontdue` is a single dependency that does both parsing and rasterization.

**Consequences:**
- `kiln-ui` gains a `fonts` feature (default off) that pulls in `fontdue`.
- When `fonts` is off, `SdfTextRenderer` still works with manually-authored
  SDF atlas data (as before).
- A `kiln-cli import-font` command is added to automate TTF → SDF atlas
  generation (Phase F-1 of 2D v1.0).
- Binary size impact of `fontdue` is small (~100–200 KB) and only when the
  `fonts` feature is enabled.



## ADR-0037: Browser sound bridge and executable web smoke coverage (accepted)

**Date:** 2026-11-03
**Status:** accepted

Browser sound playback callbacks may capture JavaScript audio handles and must
remain on the browser thread. SliceHost exposes its sound-player setter without
requiring the native Rodio audio feature. The callback type retains Send + Sync
on native targets; only wasm32 relaxes these bounds. This avoids exposing VM
internals or enabling an OS audio backend in a browser. This bridge currently
covers play_sound; complete browser music/mixer behavior remains in task 7.

The web shell uses the shared runtime scene spawner to preserve roles, source
IDs, and animation setup. WebGPU availability is probed before claiming the
canvas; when no adapter exists, use a separate WebGL instance with downlevel
limits. Startup rejects its JavaScript promise on failure.

Development-only browser verification uses pinned Playwright in tests/web,
with its npm lockfile. It adds no engine or exported-game runtime dependency.
Playwright is Apache-2.0 licensed; its browser binaries are test tooling only.
The smoke test serves real generated bindings and sample assets, detects page
and asset errors, checks rendered hero pixels, and verifies keyboard movement.
A no-error console alone is insufficient evidence. WebGPU, touch devices,
complete audio playback, and other browsers require separate checks.

Browser execution also revealed that std::time::Instant panics on
wasm32-unknown-unknown. kiln-core's Clock uses web-time 1.1 on wasm32 only,
retaining std::time::Instant on native targets. web-time is MIT OR Apache-2.0
licensed and already appears in the dependency graph; it supplies the browser
monotonic clock rather than a timer stub. Its maintenance/size cost is limited
to the WASM target. The browser movement regression exercises the real clock.

## ADR-0038: Plugin entry points return opaque handles (accepted)

**Date:** 2026-11-03
**Status:** accepted

Strict compilation found a C entry point returning a Rust trait-object pointer,
whose fat-pointer/vtable layout is not a C ABI contract. The creation macro also
allocated a boxed box while declaring a trait-object pointer. Plugin API version
2 returns an opaque void pointer to plugin-owned storage and supplies a matching
destroy entry point. The host must never dereference that handle or reconstruct
a Rust trait object across module boundaries. Creation/destruction use the same
module allocator. Null destruction is allowed; other handles must come from the
matching creator and must be released at most once.

This repairs the allocation/lifetime boundary; it does not implement the future
plugin loader or a cross-module method dispatch protocol. The in-process registry
continues to accept ordinary Rust trait objects. No dependencies are added.

## ADR-0039: Preserve authored scene metadata and isolate project file operations (accepted)

**Date:** 2026-11-03
**Status:** accepted

The serialized Project/Scene remains the source of authoring metadata. Runtime
worlds are projections, not a complete replacement document: they omit names,
parent links, unresolved asset references, and include nested-scene instances.
Editor snapshots reconcile editable runtime components by SourceId with the
original root scene, preserving unprojected metadata and filtering nested
instances. This is an incremental bridge; a shared edit-command model is still
required before task 2 can be considered complete.

Tilemaps augment an existing entity with the same SourceId instead of spawning
a duplicate entity. Sprite layer/material metadata is applied by the shared
scene spawner so opening and saving cannot silently reset it.

File operations live in an editor-only module independent of the window/GPU,
allowing real create/save/reopen regression tests. Use tempfile (MIT OR
Apache-2.0, already present in the repository dependency graph) for same-directory
staging and atomic manifest replacement, including Windows replacement support.
It is not added to the engine runtime or browser target. Creation never reuses
an existing destination directory. Project path/root state changes only after
successful I/O. Save As copies the project tree into a new or empty destination
directory, preserving relative references; it rejects occupied directories and
symlinks instead of overwriting unrelated files or copying data from outside the
project root. Same-root Save As writes an additional manifest in place.

This checkpoint does not claim complete undo coverage, scene-tab switching,
project relocation, native file pickers, or autosave/recovery.

## ADR-0040 — Explicit editor scene selection and isolated scene history

**Status:** accepted

The editor's active scene is separate from the project's startup scene. A native
editor workspace owns open scene tabs, selection, saved scene baselines, and one
undo/redo history per scene. Switching first reconciles the outgoing world into
the project document, then loads the selected scene and clears transient entity
selection/gizmo state. UI requests execute after dock rendering, so no frame can
edit one world's entities under another scene's metadata. All editor snapshots
receive the active scene ID explicitly; the legacy public snapshot helper keeps
its default-scene behavior for existing callers.

Scene creation inserts a real document and opens it. Closing a dirty tab is
blocked until the project is saved; closing a clean tab never deletes its scene.
At least one tab stays open. Saved baselines update only after successful writes.
Play still starts the project startup scene; before replacing the editor world,
capture the active authoring scene, then restore that scene on Stop. Switching,
creating, closing, undo, redo, and scene reload are blocked during Play.

This extends ADR-0039's bridge rather than claiming the complete command model.
Global project-close prompts, autosave/recovery, complete gizmo undo, and
parent/local transform semantics remain subsequent Task 2 work. No dependency
or runtime/browser architecture changes are required.

Visible acceptance also exposed two UI composition defects: the FPS row consumed
remaining height, and scene rendering cleared the frame after the UI. Constrain
status/entity rows, derive the viewport from the current frame, and draw scene
pixels before the UI with transparent scene-canvas background. This preserves
menus and floating dialogs above the rendered world. Reserve all live SourceIds
when adding entities; texture/resource maps are not identity registries.

## ADR-0041 — Guard destructive project transitions with the saved document

**Status:** accepted

Use an editor-only lifecycle guard for Open, Create-and-open, File Exit, and the
window-manager close event. Compare the complete authoring Project, including the
live active-scene snapshot, with the last successfully loaded/saved document;
scene-tab dirty flags alone miss project settings, resources, and closed scenes.
Retain one pending action and require Save, Discard, or Cancel before executing
it. Save failure retains both the document and pending action for retry/cancel.
Cancel changes neither disk nor document. Discard authorizes the pending action,
but a failed target load/create still leaves the current document open.

Opening a picker or New Project dialog is reversible and does not yet replace
the project; guard only the actual Open/Create request. Stop Play before checking
an outgoing document so runtime mutations never enter a scene save. Prompting
pauses editor interaction and clears transient gizmo drag state. Route exits
through winit's event loop rather than process::exit, allowing normal cleanup.
Keep low-level loading available for startup and already-resolved transitions.

This checkpoint covers project transitions; individual dirty scene-tab prompts,
autosave/recovery, unsaved external script buffers, and the complete authoring
command model remain separate work. No new dependency is required.

## ADR-0042 — Shared transform commands and gesture transactions

**Status:** accepted

Position inspector edits and viewport translation use the same validated
SetTransform command over the serialized root scene plus its runtime projection.
Commands address scene/entity SourceIds, preserve unrelated metadata, reject
missing/ambiguous runtime identities and non-finite/nonuniform values before
mutation, and insert an authored Transform when a default projection is edited.
A gesture captures the scene before its first change and records exactly one
snapshot on completion. No-op gestures preserve redo. Escape cancels viewport
translation without recording history. Complete pending gestures before scene,
save, Play, project-transition, and undo/redo operations.

Use egui canvas interactions rather than raw window mouse events, respecting
floating windows and input capture. Convert drag displacement using the rendered
letterboxed viewport, DPI, and camera zoom; compute from the initial transform
rather than accumulating per-frame deltas. Share the camera with rendering.
Inspector typing must be undoable as well as dragging. Translation snapping
uses world grid units. This does not implement parent/local transforms,
multi-selection, rotation/scale gizmos, or complete component command coverage.
No dependency or serialized format change is required.

## ADR-0043 — Component presence belongs to the authored scene

**Status:** accepted

Add/remove Transform, Sprite, Role, and Tilemap through root-scene component
commands. Validate scene/entity identity and new values/resources before changing
the document. Record one history snapshot only for an actual change, then rebuild
the existing runtime projection and remap selection by SourceId. A no-op or failed
command must preserve redo. Parent remains a hierarchy operation.

Inspector component presence and add/remove controls use authored data rather
than ECS membership: loading supplies default transforms, and unresolved map
references may have no runtime Tilemap. Adding Tilemap requires an existing
project map; never author an empty placeholder reference. Removal of an unresolved
authored map is intentional and must survive snapshots/reopen. Expose root
entities without sprites in the selection list so removing a visual component
does not make the entity unreachable after reopening.

Keep low-level runtime component helpers for their existing callers/tests, but
editor inspector operations use commands and projection rebuilds. This completes
component presence edits, not all property edits, entity/hierarchy commands,
map-resource creation/import, or parent/local transform semantics. No dependency
or serialized format change is required.

## ADR-0044: Scene entity commands are document-first

Hierarchy create, delete, duplicate, and reparent operations mutate the authored scene through validated commands and record one scene snapshot for undo. Runtime entities are rebuilt from that document after a successful command, keeping all authored components and metadata intact. Entity IDs remain unique across the project; duplication creates a sibling copy with a fresh ID while retaining the source's authored component values and parent link. Deleting an entity detaches its direct children to avoid dangling parent IDs. Reparenting rejects missing targets, self-parenting, and cycles.

## ADR-0045: Inspector property gestures share scene transactions

Inspector edits for Transform and Sprite update the authored scene and runtime projection together, then commit one undo snapshot per widget gesture. This extends the existing position transaction to rotation, uniform scale, Sprite tint, and layer. Typed edits and drags use the same path; separate widgets produce separate history entries, and edits preserve unrelated authored component fields.

## ADR-0046: Sprite instances carry world rotation

The shared sprite instance includes a per-instance rotation consumed by the default WGSL sprite shader. Sprite size is scaled on the CPU from the entity Transform before submission. Editor, player, web, mobile, and sample world-sprite paths use the same helper; UI, tilemap cells, and screen/world decoration default to zero rotation and unit scale. The instance buffer layout change is internal to this renderer crate; custom material shader authors must accept the added vertex attribute layout when they need rotation.

## ADR-0047: Authored sprites carry dimensions

Scene Sprite components store width and height as world-unit dimensions. Projects written before these fields existed deserialize with a 32×32 size. Inspector width/height edits share the validated Sprite property command and per-widget undo transaction with tint and layer, keeping the serialized scene and runtime Sprite synchronized.

## ADR-0048: Transform stores local values; renderers read GlobalTransform

Authored/runtime `Transform` remains local so gameplay and Inspector edits share one value. Runtime `ParentLink` components connect spawned entities, and `GlobalTransform` is recomputed from local values before scene rendering. Editor transform commands update affected global projections immediately; viewport drags convert their world-space pose back to local space. Reparenting and parent deletion preserve child world poses. Nested-scene root offsets are represented separately so propagation does not bake them into saved local values.

## ADR-0049: Scene documents are authoritative; runtime worlds are projections

**Date:** 2026-11-04
**Status:** accepted

`Project.scenes[active_scene_id]` is the single authoritative editable scene
document. The editor ECS world is a runtime projection and must not become an
independent source of authored state. Hierarchy commands, inspector commands,
and viewport `SetTransform` previews all target the same scene identity and
stable entity ID. Continuous widget/gesture previews use a `SceneTransaction`;
the transaction records one undo snapshot on completion and restores its
document snapshot on cancellation. Discrete edits use document-first commands,
then rebuild the runtime projection and remap selection by SourceId. Project
save reconciles only the active projection into that document before writing.

Keep authoring commands independent of egui and GPU state so validation,
history, save/reopen, and runtime projection can be tested headlessly. Any new
scene-editing UI must queue a command or use the shared transaction path; direct
runtime/project mutation is reserved for non-scene resources and transitional
panels outside Task 2 scope. This clarifies and supersedes ADR-0039's temporary
snapshot bridge. No serialized format or runtime API change is required.

## ADR-0050: Prefab definitions are project resources

**Date:** 2026-11-04
**Status:** accepted

Store reusable prefab definitions in a serde-defaulted `Project.prefabs` map,
keyed by stable prefab ID. The Prefab Editor state is a transient view over
those definitions; successful project load replaces the view, and panel
create/edit/delete operations synchronize back into the project document so
ordinary Save/Save As persists them. Existing manifests without the field load
with an empty prefab library. Scene instance links and override serialization
are handled separately as documented in ADR-0051; they must not be inferred
from definition persistence alone.

## ADR-0051: Prefab instance links belong to authored scene entities

**Date:** 2026-11-04
**Status:** accepted

Represent each scene instance with a serialized `Component::PrefabInstance`
containing its stable project prefab ID and override record. The enclosing
scene entity ID is the instance identity; the override record's entity ID must
match it. Instantiate through the document-first entity command, allocate a
project-unique ID, copy the prefab's authored components, place it as a root,
and append the link marker. Rebuilding an editor scene attaches the runtime
`PrefabInstance` projection by SourceId, while scene snapshots preserve the
authored marker. Duplicating an instance copies its link and retargets the
override record to the new entity ID; deleting the entity naturally removes its
link. This preserves save/reopen and history without adding runtime-only state
to the serialized project format.

Functional override editing/revert, definition propagation to serialized
instances, and drag/drop creation remain follow-up Task 3 work.

## ADR-0052: Task 4 gameplay execution and Play isolation

**Date:** 2026-11-04
**Status:** accepted; implemented for native editor/player

Editor and player use the same runtime and fixed 60 Hz gameplay step. Pausing
discards accumulated wall time; frame step advances exactly one gameplay tick;
catch-up after a stalled host is bounded to 250 ms. Losing editor focus pauses
Play and text fields consume gameplay keyboard input. Restart creates a fresh
runtime world and state rather than automatically restoring a save slot.
Current-scene Play uses the active authored scene; project-start Play uses the
same default-scene selection as the player.

Authoring scenes remain separate from runtime worlds. Stop reconstructs the
authored world. Applying runtime changes is an explicit paused operation;
the initial implementation copies valid local transforms for existing authored
entities in the active scene only and records an undo entry. Runtime spawns,
deletions, names, and resource changes are not applied by this action.

The remaining entity-script contract is:

- An authored attachment identifies an entity by stable scene/entity IDs and a
  project-relative script resource; it stores typed exposed property overrides.
- Each attachment has an independent persistent state store and `self` entity
  reference. Runtime state never modifies its authored property defaults.
- A script instance receives start once, update per gameplay tick, collision
  enter/exit and queued signal callbacks, and stop before destruction. Dispatch
  order is stable by entity/attachment ID; signals emitted during dispatch are
  queued to avoid reentrant mutation and recursive callback loops.
- Named functions, resource-relative modules, and bounded collections must be
  added to the frontend/VM before enabling entity attachments in the inspector.
  Existing top-level scene scripts continue running once per gameplay frame;
  they must not silently change to callback-only execution.
- Both editor and player must construct the same scene gameplay scheduler.
  Breakpoints/stepping must operate on the VM that actually mutates the world,
  and compile/runtime diagnostics must carry script source locations.

Implementation uses `Component::Script { path, properties }` with numeric
exposed properties. A per-entity VM retains `state_get`/`state_set` values;
`self_entity()` resolves the stable source ID. Lifecycle dispatch executes the
program with `script_phase()` set to start/update/signal/collision_enter/
collision_exit/stop. Existing scene programs retain their per-frame execution.
Functions use bounded, non-recursive inlining with isolated parameter/local
slots. Top-level relative import directives expand once per compilation,
reject cycles/project escapes, and retain diagnostic file/line origins.

The live debugger provides pause/resume, fixed-frame step, restart, and actual
entity-state inspection. The previous detached instruction-debugger UI is no
longer presented as controlling live gameplay. Source/instruction breakpoints
and arbitrary watch expressions are not exposed by this initial live debugger.
Physics collision production remains the backend's responsibility (Task 5);
the entity scheduler delivers backend events once per tick without stepping
world systems again for each callback. Native graphical acceptance still
requires a GUI-capable runner.

## ADR-0053: Task 5 physics synchronization and query ownership

**Date:** 2026-11-04
**Status:** accepted

Rapier owns physical world poses during fixed gameplay ticks. Authored/local
transforms remain scene data; the bridge resolves hierarchy/world offsets before
registration and converts physical poses back to local transforms afterward.
The bridge tracks ECS entities and generational body handles, removes orphaned
bodies/colliders/joints, and updates externally edited poses/velocities instead
of re-registering a body every frame. Collision membership/filter values are
bitsets, and sensor pairs and solid contacts feed deterministic enter/exit
events into the existing script scheduler. Queries use Rapier's actual scene
geometry and expose stable source IDs with distance/normals rather than arena
indices. Optional physics builds remain supported.

Physics components and per-scene camera settings live in `kiln-core` so their
serialized data survives without the simulation feature. Inspector commands
use scene history; camera settings use project-wide save/dirty tracking.
Native editor/player share the camera controller and aspect-fit viewport,
including integer physical upscaling in pixel-perfect mode (small panels
downscale to fit). Joint anchors use authored local units, including entity
scale. The prior standalone Physics panel is explicitly a sandbox preview;
saved scene physics is authored through the Inspector.

## ADR-0054: Shared Task 6 visual authoring and playback

**Date:** 2026-11-04
**Status:** accepted

Keep scene/project data authoritative. Add a defaulted SpriteLayout component for
pivot/flips/pixel scale and a Particles component containing shared emitter data;
retain existing Sprite serialization. Move emitter configuration into kiln-core
and re-export it from kiln-vfx, avoiding a dependency cycle or new dependencies.
Use the same clip playback, texture bindings, sorted scene draws and particle
simulation in native editor/player. Clips use actual sheet dimensions, crossed
frame events and optional non-looping next-clip transitions. Tile authoring
operates on selected map/layer data, with bounded flood fill and persisted
four-neighbor terrain rules. Resource panels invalidate the runtime projection
when authoring changes. Native graphical acceptance remains separate from
headless authoring/save/reopen/runtime acceptance.

The player test target depends on the existing editor crate and tempfile only
for acceptance: it authors through the editor commands, saves and packs a real
archive, then uses the player's loader and shared runtime. This adds no shipped
runtime dependency. Painter sorting is stable within a layer; only adjacent
texture/material runs batch, since grouping transparent sprites by texture
changes the image. Import filtering and mip generation reach GPU uploads.

Solid tilemaps without an explicit rigid body register a static Rapier compound
shape using the same tile/layer offsets and solid flags as grid queries. Adjacent
solid cells merge into horizontal row runs. Geometry edits rebuild the compound;
translation and removal synchronize with the existing fixed tick, stable source
IDs, contact transitions and ray queries. Automatic map colliders use membership
bit 1 and accept all body memberships. Axis-aligned map rendering and physics
share the same translation/world-tile-size convention.

## ADR-0055: Task 7 audio and player data boundaries

**Date:** 2026-11-05
**Status:** accepted

Persist audio assets and named mixer buses in kiln-core's Project. The Audio
Editor edits that data directly; it has no second asset library. Import validates
and copies project-relative resources. Preview owns its output independently of
Play. Native editor/player enable the existing optional audio backend by default;
headless/no-audio builds remain available with --no-default-features. No new
external dependency is introduced.

Decode each registered resource once into a shared PCM buffer. Each SFX trigger
creates a separate voice; music has one replaceable voice and an explicit loop
flag. A bounded voice pool prevents unbounded script allocation. Rodio's same
mixer feeds either an explicitly opened device or offline sample tests. Creating
a script session must never open a device. Child entity scripts borrow the same
audio context as their scene script, including error paths. Resources in packed
players are decoded from archive bytes without temporary file extraction.

Keep versioned player preferences outside the authored project and game save
slots. Project mixer values are author defaults; per-player overrides survive
scene transitions/restart. Settings writes replace a complete temporary file;
corrupt/unsupported data is reported and never silently overwritten. Generic
game data and saved UI are separate Task 7 checkpoints; this audio checkpoint
does not certify world/script restoration, UI, browser audio parity or audible
device acceptance.

## ADR-0056: Saved UI screens and shared native interaction

**Date:** 2026-11-05
**Status:** accepted

Store screens, stable widget IDs, parent-relative anchors/pivots/stretch, styles,
navigation links, button actions and font resource paths in Project.ui in
kiln-core. kiln-ui resolves and clips this data, manages edge-triggered pointer/
keyboard/controller focus and generates the same quads for author preview,
native editor Play and player. Both editor UI tabs edit this document; retire
their unsaved parallel widget models. Changes use project dirty tracking and
document-level undo/redo within the UI panel, without touching scene histories.

Use a responsive logical canvas scaled into the actual gameplay viewport. Input
uses the inverse of that same mapping, excluding letterbox bars and other editor
panels. Hidden/disabled ancestors affect children; topmost hit testing, release
activation, focus repair and held-key suppression survive screen transitions.
Screens may pause gameplay, but UI callbacks continue. Existing projects without
saved screens retain their scheduling. New scripts guard gameplay with
script_phase()=="update" and handle "ui_event" separately; root and attached
callbacks receive event/widget/screen IDs. Screen changes do not reset worlds.

Retain the built-in bitmap font and add real alpha-atlas TTF/OTF text through the
existing optional fontdue dependency. Native applications enable fonts by
default; no new external dependency. Atlases contain ASCII, authored text and
explicit extra characters; missing dynamic characters use a fallback glyph.
Clipped textured glyph quads share the ordinary sprite renderer. UI-only renderers
release prior atlas textures before reloading them, so Play restarts do not
accumulate font allocations. This does not
extend the older SDF renderer's unimplemented sampling path. Controller polling
uses the existing gilrs dependency through kiln-input; native apps enable it by
default, with no-default-features builds retained. Linux CI installs ALSA/udev
development headers before checking/building native packages.

Validate documents before Play (duplicate/missing IDs, parent cycles/depth,
invalid geometry/actions/fonts/navigation). Bound widgets, glyphs and draw
quads. The player acceptance target uses the existing workspace egui crate only as a
dev-dependency to verify that opening the authoring panel does not dirty data.
Tests author/save/reopen/pack the actual document, execute the shared
session, compare editor/player geometry/events and exercise a real font atlas.
A Linux native-player smoke job uses test-only Xvfb/xdotool/ImageMagick and
software graphics, retaining screenshots/logs as CI artifacts. The same script
is executed locally. Full editor GUI and physical-controller acceptance remain
separately recorded; generic world saves, browser parity and export UI are
subsequent checkpoints.

## ADR-0057: Native editor design and isolated 3D visual preview

**Date:** 2026-11-05
**Status:** accepted

Apply the user's uploaded Kiln Editor HTML as a visual reference for the existing
Rust/egui editor: compact charcoal chrome, a global authoring/transport toolbar,
a central scene workspace, hierarchy/inspector sidebars and an assets/console/
profiler shelf. Keep existing commands, histories and project persistence behind
these controls. Use DockArea::show_inside inside the application's CentralPanel;
a second nested CentralPanel conflicts with layout and input. Reserve status and
toolbar panels before laying out the dock. Hierarchy rows use one click/drag
response so a drag wrapper cannot intercept selection. Collect Play input edges
with a precomputed display scale: re-entering Context from its input closure
deadlocks the native editor on the first frame after pressing Play.

The user's explicit request also authorizes an editor-only 3D visual preview.
Draw bounded, lit perspective primitives with egui's existing triangle painter;
allow orbit/zoom, selection and transient preview properties. Clearly label the
mode and properties as preview-only. Keep this state separate from Project,
scene history, runtime Play and exports. No 3D scene serialization, mesh import,
physics or gameplay is claimed. Switching back must retain real 2D data and
selection; Play continues to use the actual 2D scene. No dependency is added.

Retain egui's standard color-format handling: some fallback adapters cannot use
alternate swapchain views. Verify the palette and interaction with native
screenshots and input smoke checks, alongside the workspace quality gate.
Task 7 gameplay save/load work remains paused for this explicitly requested UI
checkpoint.

## ADR-0058: Versioned gameplay saves and shared platform storage

**Date:** 2026-11-05
**Status:** accepted

Finish Task 7 with one bounded, versioned gameplay document shared by native
editor Play, player and browser. Persist project/active-scene identity, stable
entity definitions and live transforms/sprites/parents/physics/tilemaps,
root/attached script memories and locals, game-specific typed values, elapsed
simulation and UI screen/focus/back-stack/widget overrides. Rebind GPU resources
and reconstruct physics from logical data; never persist handles or pointers.
Audio voices, input captures, collision contacts and particle instances are
transient. Resume logical animation state; current player preferences remain
independent of save slots. Reject incompatible script layouts and invalid or
newer documents before replacing any live world or overwriting a file.

Queue script save/load requests and service them at completed tick boundaries,
after command application. Their boolean result means accepted; expose completion
status/error separately. The same generic session APIs also serve native F5/F9
and browser controls. Stage load into a candidate session/world, validate resource
and scene references, and install only on success. Saves capture the active scene;
game-wide values can record progress in other levels. Transfers retain game-wide
values when changing scenes. Keep the legacy sample serializer as an explicit
compatibility API, never the gameplay save path.

Introduce a small shared storage interface: native slot metadata/atomic writes
and preferences files; browser project-scoped localStorage with quota/access
errors surfaced. Use existing serde/serde_json and platform dependencies. No new
third-party dependency. Harden existing slot version/migration checks and writes.
Use tagged runtime save values to preserve integer/float/boolean/string/array
types without adding serialization dependencies to kiln-ir.

Move the browser onto the shared scene/module/attachment/UI runtime, the existing
font-atlas renderer and the same storage contract. Web Audio owns bounded voices,
looping music and named gain buses on the browser thread. Runtime commands expose
this backend without enabling native audio devices in WASM. Validate real native
and browser menus/settings/save/resume and saved UI authoring, while recording
hardware/device and remote CI acceptance separately. Task 8 remains unstarted.

## ADR-0059: Task 8 native export transaction and editor process supervision

**Date:** 2026-11-06
**Status:** accepted (native checkpoint; browser/distribution follow)

Replace simulated Build/Export with a supervised `kiln-cli export` process. Start
with the host desktop platform, a saved project and a matching prebuilt player.
Locate exporter/player beside the editor by default; allow explicit paths. Export
and playing the result must not invoke Cargo or require the player's Rust toolchain.
Other targets and installer/signing options must report unsupported until their
real implementation/target-machine acceptance exists.

The editor queues Save & Export through its existing project-save transaction,
including scene tabs, then starts a nonblocking child. Drain bounded stdout/stderr,
poll actual exit status even when the panel closes, and validate a job-specific
receipt and output before showing success. Cancellation uses a private marker
observed by the CLI, with bounded kill/reap fallback and cleanup of job staging.
No shell command interpolation. Drop/exit cancels/reaps active work.

CLI export snapshots visible project resources with relative paths, excludes
editor saves/caches/build outputs, rejects escaping/symlink resources, validates
scene/scripts/UI/resource references, packs and verifies a real KAR and copies a
compatible prebuilt runtime. A no-GPU `--runtime-info` handshake identifies the
runtime's host/version/features. Publish a complete staged directory with receipt
by rename; refuse a nonempty existing destination. Retain existing output after
failure or cancellation. The player defaults to `assets.kar` beside its executable,
so a moved export launches directly from another working directory.

Use existing serde/serde_json/uuid and standard process/filesystem APIs. Add the
existing internal kiln-runtime crate to CLI for shared scene/module validation
and device-free audio decoding; use the existing CPU font/image decoders too;
no new third-party dependency. Shared versioned runtime/receipt types belong to
kiln-core. Native editor UI and exported-game execution on Xvfb are required;
no-Rust PATH and relocation checks are evidence, not a claim of clean-machine,
other-OS/GPU or installer certification. Browser export, portable editor bundles
and clean-machine/physical target checks remain Task 8 follow-up work.

## ADR-0060: Prepared browser runtime bundles and relocated web exports

**Date:** 2026-11-06
**Status:** accepted (Task 8 browser checkpoint)

Extend Task 8's staged export/process/receipt contract to Browser. Prepare the
existing wasm-bindgen JS/WASM pair once, then export from prebuilt files without
Cargo, Node or wasm-bindgen on the author's export PATH. Embed a small versioned
custom metadata section in kiln-web; a CLI `web-runtime` preparation command reads
it and records hashes for the JS/WASM pair. Export and editor output verification
reject missing/changed/incompatible bundles. Reuse existing SHA-256 content_hash in kiln-assets (adding existing core protocol types);
no new third-party dependency. Native receipts remain readable.

Browser outputs contain the normalized project/resource tree, runtime files,
job-specific receipt, and a self-contained loading HTML/JS shell using relative
URLs and the real run(canvas_id, project_path) API. Display loading and actionable
failure/retry state, and require a user gesture after loading for explicit audio
unlock before exposing the playable canvas. Prevent the start gesture from
activating an underlying game button, including a held Enter repeat after input
installation. Add minimal WASM prepare/start entry points so loading never advances gameplay and
audio is resumed from the start gesture before simulation begins;
retain normal keyboard/pointer/touch/focus behavior. Serve over HTTP(S), including
nested deployment paths; no HTML file:// launch claim. Failures and cancellation
retain the prior export and clean staging as for native.

Keep the native player/KAR branch intact. Add Browser choice/runtime-directory
controls in the native editor and pass the selected target to output verification.
Run artifact/hash verification off the editor UI thread so large WASM files do
not pause rendering; cancellation after publication may retain complete output.
Exercise actual GUI Browser export and relocate the result, then serve only that
artifact in Chromium: loading/errors, game UI/fonts, keyboard/controller/touch,
Web Audio unlock/overlap, localStorage settings/save/reload/resume and failed
resource loading. Reject authored physics/joints and material overrides that the existing browser
host has not integrated, rather than exporting silently degraded behavior. Other
browsers/devices, clean-machine desktop dependency closure,
release budgets and portable editor packaging remain separate Task 8 gates.

## ADR-0061: Relocatable editor bundles and installed-tool identity

**Date:** 2026-11-10
**Status:** accepted (Task 8 portable development bundle checkpoint)

Prepare a portable editor archive from matching prebuilt editor/exporter/player
binaries and a verified browser runtime. Add a no-window `--tool-info` protocol to
all three tools and a CLI `verify-editor-bundle` command. Record host/version,
features and native debug-assertion flags; validate file sizes/SHA-256/executable
permissions and the browser descriptor before publication and after extraction.
Keep the browser and native export paths relative to the installed editor.

Use a Python standard-library packaging tool for preparation only, with normalized
tar/gzip metadata, a private staging directory, checksums and no replacement of
existing archives. Include working templates, the saved-UI example, instructions,
Cargo.lock and dependency notices. Inventory the union of Cargo's host/WASM
resolved workspace graphs (a conservative superset, not an exact link analysis).
Preserve package declarations and original notices/readmes; include pinned SPDX
reference texts and MPL dependency source archives with verified Cargo checksums.
Record workspace licensing as declared, without rewriting package license grants.
No new executable third-party dependency is introduced.

Add minimal template initialization when a CLI template has no scene so the
installed CLI produces a playable starter without finding repository files. Test
archive extraction, moved installation verification/tamper rejection, template
initialization, native/browser GUI exports and resulting gameplay with no tools
on PATH and an unrelated working directory. No generated binary/archive enters
Git. Local debug bundles remain development evidence; release budgets, dependency
closure/clean-machine execution, other OS/GPU/device checks and signing remain
Task 8 gates. Include direct Linux ELF/glibc observations rather than claiming a
self-contained system-library closure.

## ADR-0062: Remove package-size release goals; verify a clean Linux userspace

**Date:** 2026-11-10
**Status:** accepted (explicit project-owner request)

The owner removed release size as a project goal. Retire editor installer,
native executable, WASM, Android and iOS package-size ceilings and associated
completion gates. Size may be reported for information; no size result blocks
completion, requires an exception or justifies removing working functionality.
Keep frame-time, startup, memory/resource and bounded-input requirements.
Earlier ADR/evidence size observations are historical, superseded by this decision.
`just size` becomes an informational measurement command with no threshold checks.

Continue Task 8 by testing the prepared installation in a clean Ubuntu 24.04
container. Build a pinned-base test image with documented runtime system libraries
and native GUI test drivers, copy only the required acceptance scripts, and run
unprivileged/offline with only the archive and evidence mounted. No repository,
Rust/Cargo, development sysroot or preinstalled editor may be present. Exercise
installed-tool verification, real editor authoring/save/native export, relocated
gameplay/save/restart/Continue and library resolution. Record base/image IDs,
installed package versions and explicit absent-tool/source checks. Docker and
Python are test infrastructure, not new engine/runtime dependencies. Containers
share the host kernel/software graphics: other OS and physical-device validation
remain separate, as does browser physics/material parity. The test image and
installation share an explicit Ubuntu runtime package list, including dynamically
loaded X11 libraries that native ELF dependency inspection alone cannot detect.

## ADR-0063: Capability-checked browser 2D physics

**Date:** 2026-11-10
**Status:** accepted (Task 8 browser physics checkpoint)

Enable the existing kiln-runtime/Rapier physics feature in default kiln-web builds.
Reuse the shared scene, tilemap collider, fixed-step, collision/joint/script and
logical save/restore paths; do not introduce a separate browser physics engine.
Expose a `physics` Cargo opt-out alongside fonts. No new third-party dependency:
Rapier already powers the native editor/player and is an existing optional runtime
crate dependency. Browser minimal builds must reject authored physics explicitly.

Record a backward-compatible optional physics capability in RuntimeInfo and the
prepared WASM custom section. Legacy three-line metadata/JSON has physics=false;
new metadata adds a strictly validated physics=0/1 line. Gate physics/joint export
against the selected prepared runtime, including prefab definitions, so older
runtimes still support nonphysics projects but cannot silently lose simulation.
Keep material override rejection until the browser material host is integrated.

Add shared WASM-compatible session regressions and real relocated browser export
acceptance for simulation, collisions/filtering/raycast, kinematics/joints, logical
save/reload and scene changes. Re-run existing browser UI/input/audio/storage and
native/workspace checks. Software browser evidence does not certify physical
hardware/other browsers. Package-size goals remain removed (ADR-0062).

## ADR-0064: Shared authored sprite material pipelines

**Date:** 2026-11-10
**Status:** accepted (Task 8 material checkpoint)

Replace the unused placeholder sprite material pipeline with one shared contract
for editor/native/browser. Keep Kiln's vertex transform/UV/tint shader; authors
supply a WGSL fragment entry taking VertexOutput (color, uv) and returning RGBA.
Expose typed shader-default/material-override constants (Float/Vec2/Vec3/Vec4/Color)
and named Texture parameters with generated texture/sampler bindings. Parameters
are specialized per material; project edits rebuild cached pipelines, not every
frame. Cap source/definition/parameter/texture counts as resource bounds, not
package-size targets. Validate references/types/finiteness/WGSL/stages/bindings
before export and capture backend pipeline failures through wgpu error scopes.

Add kiln-core as an internal kiln-render dependency (core does not depend on
render), plus direct naga 22.1 with wgsl-in, already present through wgpu 22.1
(MIT/Apache-2.0, same compiler version, no new compiler implementation). Centralize
material preparation/texture references and use actual render-target formats,
quad/instance layouts and explicit compatible bind layouts. Bind per-material
textures and preserve transparent painter order across default/custom/default runs.
Missing/invalid authored material definitions fail export/startup; editor drafts
surface errors and visibly mark unresolved material draws.

Wire shared preparation/draw IDs into all three hosts and correct the shader
editor's invalid new-shader template/entry defaults. Extend prepared runtime
capabilities so old runtimes cannot silently ignore materials. Test pure validation,
real native/browser pixels (including numeric and texture overrides, alpha/order,
rotation, scene/save restoration), failure feedback and existing workflows.
No release-size goal is reintroduced. Physical hardware/other OS and provenance
checks remain separate Task 8 gates.
### ADR-0065 — Captured portable builds and local build records (2026-11-10)

- Task 8's prebuilt packager records preparation observations, which cannot link
  arbitrary supplied binaries to source. Add an explicit Linux build driver that
  snapshots tracked/nonignored files plus Cargo.lock, rejects links/special files,
  hashes and archives that snapshot, and uses it as the build working directory.
- Build native/default-WASM hosts offline and locked into a fresh target directory;
  capture compiler/Cargo/wasm-bindgen versions, fixed commands, logs and artifact
  hashes. Clear inherited Cargo profile/target/compiler-wrapper/rustflags overrides
  rather than letting ambient flags silently alter the selected profile. Preserve
  normal package-manager/proxy/CA and platform SDK discovery; record only selected
  build settings and configuration hashes, never credential contents.
- An optional local build record is checked by the packager against the snapshot,
  lockfile, compiled profile and exact six native/browser payload files. Include it
  in the verified installation. Prebuilt packaging remains supported without a
  record, explicitly labeled as preparation-only observations.
- This is an unsigned local execution record, not remote attestation, a hermetic
  toolchain/sysroot/dependency audit or proof of byte-reproducible binaries. Keep
  source/lock archive and logs beside the candidate for inspection. External Cargo
  configuration/SDK/system dependencies still affect builds. Repeated builds and
  trusted CI/signing remain release gates; size has no acceptance threshold.
- No new dependency; Python standard library and existing Rust/build tools only.
### ADR-0066 — Replaying captured builds and comparing actual payloads (2026-11-11)

- Task 8 needs executed repeated-build evidence, not just recording one build.
  Extend the captured builder with --source-from: validate the prior completed
  record, source inventory/archive and lockfile, then copy the same source into a
  new directory and use fresh compilation outputs. Require the prior profile,
  host, compiler/bindgen, selected SDK settings and observed configuration hashes.
- Build commands remain fixed by our driver; never execute commands supplied by
  a build record. Replaying older engine source can use a newer build driver;
  record the driver independently from the engine source inventory and retain
  the actual executing driver beside its logs.
- Add a comparison command that rehashes actual on-disk source/artifacts/logs and
  completed archive/record receipts before comparing compatible input contracts
  and all six native/prepared-browser payloads. Missing/changed files, different
  inputs or unequal output must fail. Reports are exclusively created, so a stale
  success cannot be silently overwritten or reused.
- Compare payload bytes rather than claiming whole archive equality: build
  records/log observations can legitimately differ between executions. Report
  archive equality separately. Same-host/toolchain repetition does not certify
  hermetic dependencies, independent builders, signatures or physical hardware.
- No new dependency. Package sizes remain informational, never an equality or
  release-size threshold. A changed byte is a reproducibility observation, not a
  package-size failure.
### ADR-0067 — Locked CI release candidates and separate provenance signing (2026-11-11)

- Task 8's local builds are tested, but the existing CI only triggers on main /
  master and cannot establish a hosted build for the active feature branch.
  Track the exact root Cargo.lock already used by the release evidence; nested
  standalone-workspace lockfiles retain their existing ignore behavior.
- Add a Linux release-candidate workflow on selected feature/default-branch build
  changes and manual dispatch. Pin the tested Rust/nextest versions and immutable
  official Actions commits. Resolve bindgen from the tracked lockfile, use locked
  dependency preparation, then fixed offline source capture/replay into fresh
  target directories. Run quality/tests, payload comparison, moved installation /
  browser behavior and clean Ubuntu acceptance before uploading a candidate.
- Build/test has contents:read only. A separate job, after successful acceptance,
  downloads fixed artifact files and invokes pinned official provenance actions
  with id-token:write / attestations:write. It checks out/executes no engine code.
  This produces GitHub build-origin attestation, not OS installer signing or a
  licensing/physical-device certification. Failed gates cannot publish a candidate
  as successful; diagnostic evidence is retained independently.
- Candidate contents and attestation bundles are Actions artifacts. Existing Git
  branch publication is sufficient authorization for this CI workflow; no public
  release or external message is sent by the driver. Scope is Linux x86_64 and
  hosted software acceptance. Remote completion must be observed before claiming
  that a hosted build/attestation passed.
- No runtime dependency. actionlint v1.7.7 (official release/checksum) is a local
  development validator; Rust/Node/browser tooling already exists in the project.
  No package-size ceiling is introduced.

### ADR-0068 — Early artifact-service verification and native Node 24 actions (2026-11-11)

- The owner supplied run 43088213774's actual annotations: both artifact uploads
  failed with CreateArtifact HTTP 404, "workflow run not found", after successful
  engine acceptance. This does not establish quota exhaustion or an expired
  token. The same run warned that Node 20 actions were forced onto Node 24.
- Update the release workflow to immutable official Node 24 action commits,
  including the current artifact client. Preserve zipped multi-file uploads and
  strict publication errors. Updating the client is a remediation attempt, not
  proof that runtime migration caused the service's 404.
- Add a short read-only preflight job before engine builds: upload a run/attempt/
  source identity record, download it, and compare the actual bytes. A failed
  artifact service stops expensive builds early. Successful preflight does not
  certify the later candidate upload or separate OIDC attestation; observe those
  jobs too. Retain the small probe for one day and isolate reruns by attempt.
- Cache Cargo's installation manifests alongside cached tool binaries, with a
  new cache key. Restored executables alone do not preserve cargo install's
  version tracking. No engine/runtime dependency, permission expansion, public
  release, upload-error suppression or package-size ceiling is introduced.

### ADR-0069 — Native Windows portable candidate preparation (2026-11-12)

- Task 8 has an accepted hosted Linux candidate; the owner's Windows laptop
  needs actual Windows binaries and platform evidence. Extend the prebuilt
  packager with deterministic ZIP output for Windows, preserving the existing
  Linux archive format and offline installation verifier. Include matching
  editor/exporter/player, browser runtime, templates/examples and original notices.
- Build on a hosted Windows x86_64 MSVC runner with pinned Rust/official actions
  and the root Cargo.lock. Prepare the browser runtime with matching bindgen.
  Keep this separate from the captured Linux builder: Windows preparation uses
  preparation-observations-only provenance, not a captured/replayed build claim.
- Move the installation outside the checkout into a path with spaces, verify it,
  reject corruption, initialize all templates and export Windows/browser games
  with Rust/Node absent from PATH. Exercise visible editor/player startup and
  normal window close using Windows APIs, retaining logs and screenshots where
  available. Failures stay strict; no candidate is accepted from failed gates.
- Publish passing Windows candidates as Actions artifacts, with a separate
  no-checkout provenance job. This establishes hosted preparation and startup,
  not full Windows GUI authoring/gameplay, physical Intel HD 520 performance,
  audible/controller testing, Authenticode signing or public-release licensing.
- No engine/runtime dependency. ZIP and acceptance use Python's standard library,
  Windows APIs and existing hosted tools. Package sizes remain informational.

### ADR-0070 — Project MIT OR Apache-2.0 license and bundled grants (2026-11-12)

- The owner delegated the release license choice. Retain the workspace's existing
  MIT OR Apache-2.0 dual license: permissive commercial use, familiar Rust
  ecosystem terms and the option of Apache's explicit patent grant.
- Add full root LICENSE-MIT and LICENSE-APACHE texts, replace README:TBD and make
  all missing project crate/sample declarations inherit the workspace license.
  Existing third-party/font licenses and attribution remain their own grants.
- Portable packages must include the project license texts under licenses/project
  and identify them in the inventory. Reject missing project grants or unlicensed
  path-package metadata rather than inventing a declaration during packaging.
- Historical artifacts/evidence preserve their original declarations. New licensed
  candidates require new packaging from the updated source; this does not imply
  prior archived downloads contained the newly added project license files.
- No engine/runtime dependency or release-size ceiling.

### ADR-0071 — Windows UTF-8 metadata and observable packaging failures (2026-11-12)

- Windows run 43206825541 passed native/WASM release compilation and static-CRT
  DLL inspection, then failed packaging before acceptance. Public annotations
  expose only exit code 1. Existing .gitattributes already preserves pinned
  license bytes; do not claim a newline-conversion cause or change those hashes.
- Cargo/tool JSON is UTF-8. Read it explicitly as UTF-8 rather than a Windows
  locale default, which can corrupt names or fail on valid Unicode. Exercise a
  Unicode repository value with a simulated CP1252 default and check its actual
  inventory roundtrip. The detailed remote exception still needs observation.
- Emit escaped packaging exceptions as public workflow annotations and retain
  build/package diagnostics. Keep failed preparation from publishing a candidate.
- Save successful tool installation and compiler-output caches before later
  acceptance can fail. Compiler caches are keyed by Rust, tracked crate inputs,
  lockfile and hosted runner image; disable output caching if image identity is
  absent. These Windows outputs are prebuilt preparation observations, not fresh
  captured/replayed or hermetic builds. Cache hits never bypass DLL, installation,
  export, desktop or publication gates. The captured Linux builder stays fresh.
- No new engine dependency, permission expansion, error suppression or size goal.

### ADR-0072 — Runtime-matched notices in native and browser exports (2026-11-12)

- Task 8 currently requires manually copying the installation's notice directory
  into exported games. Automatically copy it from the actual selected runtime's
  verified portable installation, including project grants, original dependency /
  font/Rust notices, inventory and included source archives. Reject incomplete or
  changed installations; never attach an unrelated exporter's license inventory
  to a custom runtime. Loose developer runtimes without an installation remain
  supported, with a visible missing-notices warning and no verified-notice claim.
- Add an optional notice-manifest SHA-256 to the export receipt. The attached
  manifest inventories exact copied notice bytes and selected exported runtime
  payload hashes. Native receipts allow only core payloads plus the declared
  notice subtree. Existing receipts without notice metadata retain their prior
  shape. No project/save format or runtime execution protocol changes.
- Keep hashing in kiln-assets using its existing dependencies. Both exporter and
  editor's background output verifier check the manifest hash, exact notice tree,
  copied bytes and runtime binding. A copy/verification/cancellation failure must
  preserve existing output and clean staging rather than publish partial success.
- This verifies consistency with the matched installation's conservative notice
  inventory, not legal completeness, signatures or SDK provenance. Project asset
  notices remain the game author's responsibility. Full matching tool bundles are
  required; mixing developer snapshots is not a supported distribution workflow.
- Windows may restore compiler/dependency outputs from the same Rust/runner-image
  cache prefix across source changes; Cargo invalidation and every downstream
  acceptance gate still run. The captured Linux release builder remains fresh.
- No new dependency or release-size threshold. Existing bounded-input limits
  remain; no resource budget is weakened.

### ADR-0073 — Native editor polish and Quick Actions (2026-11-12)

- User-requested editor polish is a focused native workspace task; Task 8 release
  acceptance remains separate and no additional 3D milestone begins.
- Refine the existing charcoal theme and toolbar rather than introduce a UI
  framework. Keep current docking, documents, undo history and project formats.
- Add a bounded, session-only Quick Actions search (Ctrl/Cmd+K), backed by the
  existing menu actions and dock tabs. Reuse existing tabs instead of duplicating
  panels. Save/undo/redo follow their current state restrictions; lifecycle
  prompts close the palette and prevent commands behind them.
- Borrow familiar workflow ideas from Unity, Godot and Unreal: visible transform
  tools, searchable actions, scene/hierarchy/inspector organization and an asset
  shelf. This does not claim feature parity with those engines.
- No new dependency, runtime feature, project format or resource-budget exception.

### ADR-0074 — In-editor character scripting (2026-11-12)

- Complete one focused editor scripting workflow, retaining existing entity
  Script components and the shared SliceSession runtime. No new dependency.
- Add a native text editor for project-relative .ks files, explicit atomic Save,
  runtime compiler validation (including imports), external-change protection,
  and a non-overwriting movement template attached through component history.
- Unsaved script text stays separate from scene undo. Opening another file,
  switching/closing projects and Play require Save or explicit Discard Edits.
  Closing the script tab retains its buffer. Source edits are not hot reloaded.
- Keep source bounded to the runtime's 1 MiB limit and enforce project-root
  containment. Remove the rapidly changing FPS readout from the status bar.

### ADR-0075 — KilnScript gameplay control flow (2026-11-12)

- Expand one language milestone: short-circuit &&/||/!, compound arithmetic
  assignments, else-if chains and while loops with break/continue. Reuse existing
  IR jumps, locals and arithmetic; do not change bytecode, host ABI or add crates.
- Boolean operators require bool operands when evaluated. The right side of &&
  and || is skipped when unnecessary, preserving side-effect behavior.
- Loops have lexical block scope; break/continue target the innermost loop in
  the same function. Inlined functions must not inherit caller loop targets.
  Existing VM instruction limits bound runaway loops; no budget is weakened.
- Unary operators bind before multiplication/division, which associate left.
  This corrects the old right-associative division parser and is documented.
- Bound added block/unary nesting and retain lexical errors with byte spans.
  New while/break/continue keywords can require renaming legacy identifiers.
- Validate frontend semantics, malformed input, runtime attachment execution and
  editor validation through tests. Full engine/3D/release work remains separate.

### ADR-0076 — Editor-authored Task 9 reference games (2026-11-12)

- The owner explicitly starts Task 9 on a new branch before the remaining Task 8
  platform/release checks. Limit this session to the three reference games and
  editor blockers they expose; hardware, independent tutorial and release review
  stay open. Task 8's new Linux/Windows notice candidates both passed.
- Author projects through native editor interactions, including project creation
  or Save As, scene editing, gameplay source editing, Save/Reopen/Play and Export.
  Automation may supply ordinary script text/assets and inspect resulting files;
  it must not generate game manifests behind the editor or add game-specific
  engine buttons/loaders to bypass authoring. Keep reproducible interaction
  drivers and actual evidence with the games.
- Add a general Script Editor for project-relative .ks files, the missing ordinary
  authoring operation. Bound source to 1 MiB, reject traversal/links and external
  modification, save atomically, and include drafts in Save/Discard/Cancel
  protection. Compilation diagnostics use the existing shared runtime compiler.
  This is not a new language/runtime/dependency or a project/save format change.
- Reference games may start from existing example projects through Save As and
  import their assets, then replace gameplay through the editor. Give games
  independent project identities and document controls, goals and constraints.
  Do not equate three games with completion of all Task 9 release gates.

### ADR-0077 — Reconcile master and reference-game editor integrations (2026-11-12)

- Merge current master into the reference-game PR without replacing either
  branch's work. Retain Quick Actions, workspace polish, movement-script creation
  and new language control flow alongside the editor-authored games.
- Use one ScriptEditor buffer in EditorShell for inspector, assets, dock tabs,
  project saving, Play and export. Preserve generic Open/New and guarded atomic
  saves, adding master's compiler/import validation and movement starter.
- Normal project replacement/exit uses Save/Discard/Cancel for script drafts;
  failed saves retain both draft and prompt. Inspector file switching and
  movement creation still require saving/discarding the existing draft.
- Keep both branches' regression tests and rerun actual editor authoring/export
  against the integrated UI. No new dependency, format or size/resource exception.
- Reuse an existing dock panel when menus reopen it. Master adds default animation
  and source tabs; duplicate copies cause egui widget-ID collisions and ambiguous
  authoring. Keep the default layout and allow a floating panel after closing it.

### ADR-0078 — Explicit experimental binary archive distribution (2026-11-13)

- The legacy cargo-dist.toml filename is not loaded by cargo-dist 0.33.0. Put
  distribution configuration in supported Cargo workspace/package metadata.
- Opt in only kiln-editor, kiln-cli and kiln-player; exclude sample/demo binaries.
  Configure desktop binary archives and upload target/distrib, the tool's actual
  output. Do not invoke the nonexistent `cargo dist upload` command.
- Archives are experimental separate tools, not complete editor installations.
  Keep complete matched tools/assets/templates/notices/browser runtime in the
  existing portable Linux/Windows candidate workflows. DMG/AppImage/MSI installers
  remain planned; do not call executable archives installers or release acceptance.
- No new dependency, runtime architecture or package-size goal. A future installer
  must wrap the full portable contract and pass target-specific install/uninstall,
  signing and clean-machine acceptance before publication is enabled.

### ADR-0079 — Recover unsaved gameplay source alongside project drafts (2026-11-13)

- The owner requested continuing all tasks in batches. While A01 hosted verification
  runs, implement the independent A02 draft-recovery substep; keep both milestone
  acceptance gates open until their own checks pass. Full 3D remains gated on G2D.
- Store a versioned recovery envelope containing Project and an optional script
  draft (loaded relative path, unsaved text, original disk baseline). Continue to
  read legacy project-only snapshots. This changes editor sidecars, not project,
  game-save or export formats, and adds no dependency.
- Recovery never writes gameplay source. Restore into the shared Script Editor
  buffer after validating existing path/link/size rules; preserve the original disk
  baseline so external edits still block Save. If disk already equals recovered
  text, treat that text as saved. Failed restore retains the prompt and sidecar.
- Preserve script drafts even if the project manifest was saved more recently;
  retain that newer saved project rather than restoring stale scene contents.
- Write sidecars through unique same-directory temporary files and atomic replace.
  Keep recovery until successful Save or explicit Discard. Escape must not delete
  unsaved recovery data. Validate actual force-close/restart/widget recovery as
  well as roundtrip, external-change, invalid-path and legacy-format regressions.

### ADR-0080 — Complete authoring, asset and script work in staged commands (2026-11-14)

- The owner explicitly requested continuing through A04, overriding the one-
  milestone session default. Keep individual acceptance gates and evidence honest;
  full 3D remains gated on G2D. Implement and test small steps across A02–A04.
- Entity clipboard operations use the authoritative scene command/history model,
  not direct ECS edits. Capture selected subtrees, preserve external-parent world
  poses, remap stable internal references and prefab instance identities, and
  reject invalid/missing dependencies before mutation. Paste is one undo step.
- Keep project/game formats compatible where practical; new persisted authoring
  metadata needs explicit backward defaults and must not alter game visibility.
- Reuse existing asset IDs, import settings, prefab and language/IR facilities;
  do not invent parallel scene models or duplicate schedulers. New language/debug
  capabilities must retain bounded execution and native/browser semantics.
- No new dependency or package-size goal is authorized by this decision.

### ADR-0081 — Bounded dictionary values in the shared gameplay VM (2026-11-14)

- Add first-class string-keyed dictionary values to the existing IR value model,
  with deterministic key order and recursive equality. Expose value-returning
  dict_new/get/set/has/remove/keys through the shared host registry, like arrays.
- Enforce existing state-value depth/item/string limits and 256-byte dictionary
  keys. Extend tagged saved values so dictionaries survive save/load and runtime
  snapshots without losing numeric types. Older engines reject unknown variants
  rather than silently converting them. No parallel interpreter or dependency.
- Counted for loops lower to existing jump/local instructions; continue reaches
  the increment, break exits the innermost loop, and initializer locals are scoped
  to the loop (including shadowing an outer name). Existing VM budgets still apply.
- The VM owns configured and remaining instruction limits; both normal execution
  and bytecode debugger steps consume the same budget. Resume skips only the
  breakpoint just reported; step-over/out follow actual bytecode call depth.
  This repairs the bytecode debugger and does not establish source-level/live
  gameplay debugging or evaluation of conditional watches.

### ADR-0082 — Finish A01–A04 with persistent authoring controls (2026-11-14)

- The owner's explicit instruction to finish A01 through A04 authorizes staged
  work across these milestones. Keep existing scope and acceptance gates; do not
  relabel omitted features as optional or claim a pending target has passed.
- Store editor-only locked/hidden flags as an optional EditorFlags component in
  the authoritative scene document. Missing flags mean unlocked/visible; parent
  flags inherit down the hierarchy. Players ignore these authoring flags.
- Locks reject direct property/component/hierarchy edits through command paths;
  unlocking is always possible from hierarchy controls. Hiding affects edit-mode
  drawing/picking only, including inherited children; Play uses normal visibility.
- Viewport picking/marquee and alignment reuse runtime sprite geometry and stable
  scene IDs. Alignment is explicitly alignment of world-space transform positions,
  not sprite edges. Each group alignment is one undo transaction; descendants of
  selected ancestors retain their local transforms rather than moving twice.
- New controls require real widget/save/reopen and negative-command regressions.
  Continue using the existing dependencies, scene history and host/IR systems.
