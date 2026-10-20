# Kiln Budgets

All budgets are enforced via `just size`. Exceeding a budget requires an ADR entry with justification.

---

## Native GPU backend support (R0)

**wgpu version:** 22.1

**Supported backends:**
- **Vulkan:** Windows, Linux/Android, macOS/iOS via MoltenVK (requires `vulkan-portability` feature)
- **Metal:** macOS and iOS (first-class support)
- **Direct3D 12:** Windows 10 and later (first-class support)
- **OpenGL/GL ES:** Linux/Android, Windows, macOS/iOS via ANGLE (downlevel/second-tier support)
- **WebGPU:** WebAssembly (first-class support)
- **WebGL2:** WebAssembly (via gles backend, downlevel support)

**Configuration:**
- All backends enabled via `wgpu::Backends::all()` in renderer initialization
- Fallback order: PRIMARY (Vulkan/Metal/DX12/WebGPU) → SECONDARY (GL/WebGL2)
- Environment variables available for forcing specific backends:
  - `WGPU_BACKEND`: comma-separated list (`vulkan,metal,dx12,gl`)
  - `WGPU_ADAPTER_NAME`: substring match for specific GPU
  - `WGPU_POWER_PREF`: `high`, `low`, or `none`

**Current status:** Verified wgpu 22.1 backend support documentation (2026-10-20)
**Testing needed:** Test fallback on real old hardware (requires physical testing on old GPU)

**Location:** `kiln-render/src/renderer.rs` (line 48-50), `kiln-render/src/screenshot.rs` (line 24-26)
**References:** https://docs.rs/wgpu/22.1.0/wgpu/

---

## Audio device management (R0)

**Current implementation:** M11 stub using rodio's high-level API

**Status:** Basic playback works (default device only). Full device management deferred to M49 audio v2 per ADR-0014.

**Current capabilities:**
- Opens default audio output device via `DeviceSinkBuilder::open_default_sink()`
- Plays audio files (OGG/Opus via symphonia decoder)
- No device selection (always uses OS default)
- No hot-plug detection
- No sample-rate change handling
- Feature-gated via `audio` feature in `kiln-runtime`

**Deferred to M49:**
- Device enumeration and selection
- Hot-plug detection and handling
- Sample-rate change handling
- Per-device audio settings

**Location:** `kiln-runtime/src/audio.rs`
**ADR:** ADR-0014

---

## Web runtime WASM

**Target:** <1.5 MB brotli-compressed

**Measurement:**
```bash
wasm-opt -Oz -o output.wasm input.wasm
brotli output.wasm -o output.wasm.br
ls -l output.wasm.br
```

**Current status:** 36 KB uncompressed (measured in M12, release build); brotli compression not yet measured (wasm-opt not installed locally)

**Notes:**
- Measure with `wasm-opt -Oz` and brotli
- Include only runtime dependencies (no editor)
- Feature-gate physics/audio to stay under budget
- Current 36 KB is well under 1.5 MB budget

---

## Native player binary

**Target:** <15 MB (executable without assets)

**Measurement:**
```bash
cargo build --release --bin kiln-player
ls -lh target/release/kiln-player
```

**Current status:** 930 KB (verified 2026-10-20, release build with LTO, panic=abort, strip=true)

**Notes:**
- Player binary contains only runtime, not editor
- Assets shipped separately in packed archive format
- Target is for stripped release build (LTO, panic=abort, strip=true)
- Measure separately from editor installer
- No editor crates in dependency tree (verified via cargo tree)
- Archive format (KILNARV1) size varies with content; sample: 68 bytes for 1 text file

---

## Android APK

**Target:** <20 MB (includes player binary + assets)

**Measurement:**
```bash
cargo apk build --release -p kiln-player
ls -lh target/aarch64-linux-android/release/apk/kiln-player.apk
```

**Current status:** Not yet measured (record after M18)

**Notes:**
- APK includes player binary (same runtime as desktop) and game assets
- Use feature-gating to exclude unnecessary features (physics, audio, 3D) for 2D web/mobile games
- Signed AAB for Play Store will be similar size
- cargo-apk handles APK generation with Gradle project
- Install via ADB or manually on device

---

## iOS App Bundle

**Target:** <25 MB (includes player binary + assets in .app bundle)

**Measurement:**
```bash
cargo build --release --target aarch64-apple-ios -p kiln-player
du -sh target/aarch64-apple-ios/release/libkiln_player.a
# Or measure .app bundle size after Xcode packaging
du -sh YourGame.app
```

**Current status:** Not yet measured (requires macOS for actual build; record after M26)

**Notes:**
- iOS .app bundle includes player static library and game assets
- Use feature-gating to exclude unnecessary features (physics, audio, 3D) for 2D mobile games
- App Store limit is 4 GB (can be increased with approval)
- Static library approach: `libkiln_player.a` linked into Objective-C app
- Requires macOS with Xcode for build and signing
- See docs/IOS.md for complete build instructions

---

## Editor installer

**Target:** <40 MB

**Measurement:**
```bash
cargo-dist build
ls -lh dist/kiln-*.msi  # or .dmg, .AppImage
```

**Current status:** 5.7 MB editor binary (measured in M12); installer not yet built

**Notes:**
- Use cargo-dist for packaging
- Strip symbols, use LTO, panic=abort
- Exclude debug symbols and large assets from installer

---

## Startup time to first frame

**Target:** <2 seconds on desktop reference hardware

**Measurement:**
```bash
# Instrument application with startup timing logs
# Measure from process launch to first rendered frame
```

**Current status:** Not yet measured (player binary exists at 930 KB; startup timing instrumentation needed)

**Notes:**
- Measure on desktop reference hardware (similar to frame budget reference)
- Exclude time spent loading large game assets (measure with minimal sample)
- Archive loading should be fast; memory-mapped loader implemented in R0-2
- Critical for perceived performance
- Requires instrumentation code to measure from process launch to first rendered frame

---

## Editor idle RAM

**Target:** <300 MB

**Measurement:**
```bash
# Open editor with empty project
# Check Task Manager / Activity Monitor
```

**Current status:** Not yet measured (record after M6)

**Notes:**
- Measure with empty project loaded
- No assets or scenes loaded
- Baseline before adding features

---

## Frame budget

**Target:** 16.6 ms (60 FPS) on Low reference device

**Reference device:** 2015 laptop with integrated GPU (to be specified in M12)

**Measurement:**
```bash
# Run sample game on reference device
# Use profiler or frame timing logs
```

**Current status:** Not yet measured (record after M9/M12)

**Notes:**
- Low tier: no post, baked light, no soft shadows
- Medium/High: more effects, but still meet budget on appropriate hardware
- No allocations in hot loops (enforced via profiling)

---

## Release profile settings

**Cargo.toml:**
```toml
[profile.release]
opt-level = "z"  # or "s" - benchmark both
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

**WASM-specific:**
```toml
[profile.release.package.kiln-web]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

---

## GPU pass timings (M28)

**Target:** Per-pass GPU cost via timestamp queries (when adapter supports it)

**Measurement:**
```bash
cargo run -p render-graph-sample --release
cargo test -p kiln-render --lib screenshot_low_tier_stable_hash
```

**Current status (M28, release, Windows desktop, TIMESTAMP_QUERY available):**
- High-tier stub passes (clear/load): main ~32 µs, bloom/fog/outline/lut/composite ~18 µs each
- Low-tier screenshot golden sha256 `d17424c3…9478` (64×64 RGBA8, clear 51/102/153/255)
- Re-run sample on reference Low/Medium GPUs to refresh; editor Settings → GPU Pass Timings

**Notes:**
- Requires `TIMESTAMP_QUERY | TIMESTAMP_QUERY_INSIDE_ENCODERS`; otherwise timings empty (editor shows unsupported)
- Transient aliasing reduces physical texture slots when lifetimes do not overlap

---

## Multithreading speedup (M27)

**Target:** Measurable speedup on 4+ cores for chunked CPU work via `TaskPool`

**Measurement:**
```bash
cargo run -p threads-sample --release
```

**Current status (M27, release, Windows desktop, 4 logical cores):**
- Chunked `sin/cos` over 2M f32 values (`samples/threads`)
- Serial: 199.52 ms · Parallel: 60.26 ms · **Speedup: 3.31× on 4 cores**
- Re-run with `cargo run -p threads-sample --release` on reference hardware to refresh

**Notes:**
- `threads` feature off: pool runs inline (1×, behavior unchanged)
- Web: Workers only when cross-origin isolated; otherwise single-thread fallback (no SAB)
- No rayon; custom `std::thread` pool (ADR-0024) keeps binary impact near zero

---

## Dependency size tracking

Every new dependency must have an ADR entry with:
- Binary size impact (measured via `cargo bloat` or similar)
- Compile time impact
- Justification for inclusion

**Tools:**
- `cargo-bloat`: identify largest functions
- `twiggy`: WASM size analysis
- `cargo-size`: binary size per crate

**Recent additions:**
- kiln-ui (M21): 830 KB release rlib (includes widgets, layout, navigation, localization, 9-slice support)
- M27 threads: no new crates (std::thread only); kiln-web adds js-sys + Worker/Blob/Url web-sys features (stub path, size impact expected small)

---

## Budget enforcement

CI will fail `just size` if:
- Web WASM >1.5 MB brotli (after M10)
- Native player binary >15 MB (after M14)
- Android APK >20 MB (after M18)
- iOS .app bundle >25 MB (after M26, requires macOS runner)
- Editor installer >40 MB (after M12)
- Startup time >2 seconds on reference hardware (after M14)
- Editor idle RAM >300 MB (after M6)
- Frame time >16.6 ms on reference device (after M12)

Temporary exemptions must be documented in STATE.md with a plan to fix.
