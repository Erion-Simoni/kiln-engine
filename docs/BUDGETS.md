# Kiln Budgets

Package-size goals were removed at the project owner’s request on 2026-11-10
(ADR-0062). Editor, native player, WASM and mobile package sizes are informational
measurements, with no release ceiling or size-based completion gate. Frame-time,
startup, memory/resource budgets and bounded-input limits remain active.
`just size` only reports sizes; it does not enforce these other budgets.

## Task 8 authored sprite materials — 2026-11-10

Material pipelines are cached across unchanged frames. Preparation is bounded at
256 shaders/materials, 256 KiB per source, 1 MiB total authored sources, 64 typed
parameters per shader and four named textures per material. These are technical
resource limits, not package-size goals. Constants specialize pipelines on edit;
shader compilation/validation does not run every frame. Shader definition/texture
keys are compared for changes. Additional material runs can add draw calls while
preserving transparent painter order; no new render pass is added.

Actual native/browser pixels agree within one channel value of backend rounding
for the test fixture. This is functional software evidence, not a reference GPU
performance/startup certification. See SPRITE_MATERIALS_2D.md and the selected
material evidence. Package-size goals remain removed.

## Task 8 browser physics — 2026-11-10

Default WASM builds now enable the existing shared Rapier runtime. Simulation uses
existing 60 Hz fixed-step limits and bounded save documents; no new render pass is
added. Browser acceptance covers contacts/filtering/sensors, tile colliders,
kinematics and all three joint types, save/reload and scene replacement. This is
functional Chromium/SwiftShader evidence, not reference-hardware timing/memory
certification. Physics can be explicitly opted out; runtime capability checks
prevent silently exporting unsupported authored physics. Package sizes remain
informational only (ADR-0062). See [BROWSER_PHYSICS_2D.md](BROWSER_PHYSICS_2D.md).

## Task 8 clean Ubuntu userspace — 2026-11-10

The prepared editor and native export passed clean Ubuntu 24.04 container
acceptance without Rust, repository or development sysroot access. Xvfb/Mesa
software graphics and ALSA null were used; startup/frame-time/physical-device
budgets were not certified. The run found a missing runtime-loaded X11 dependency
and added `libxkbcommon-x11-0` to shared shipped/tested requirements. Runtime
packages/GLIBC and exact image identity are recorded under
[evidence/task8-clean-linux](evidence/task8-clean-linux/README.md). The older
relocation measurements below remain historical observations. Package size has
no release ceiling or completion gate (owner request, ADR-0062).

## Task 8 portable editor observations — 2026-11-10

The verified Linux development archive is **56,605,832 bytes** compressed,
containing **287,905,643 bytes across 1,530 listed files**, plus bundle.json.
It includes three debug native tools, debug WASM/bindings, templates/sample,
433-package conservative dependency notices, 17 verified MPL source archives and
the compiler standard-library copyright report. Native debug information was
disabled; debug assertions remain enabled. Package size is informational; there is no release-size target.

Installation verification caps the manifest at 8 MiB, file count at 20,000,
each file at 512 MiB and total listed contents at 2 GiB. Native hashing streams
64 KiB chunks. Verification is an explicit CLI preparation/installation check,
not an automatic editor startup cost. No new rendering pass or performance claim
is introduced. This build references GLIBC 2.39; direct ELF requirements are
observations, not clean-machine/system-plugin dependency closure. See
[PORTABLE_EDITOR.md](PORTABLE_EDITOR.md) and evidence/task8-portable/.

## Task 8 browser export observations — 2026-11-10

The editor-exported saved-UI fixture contains a **40,080,403-byte debug WASM**,
**118,759-byte generated JS** and **40,314,183 bytes across 15 output files**
(including runtime descriptor, receipt, shell and raw project resources). These
are development-profile size observations; startup requires separate measurement.
Export reuses the selected prepared bundle; it does not run Cargo or optimizers.
Package-size measurements are informational (ADR-0062). Runtime/hash verification runs off the editor UI
thread; resources load before the first gameplay tick. No new rendering pass or
frame-time performance claim is introduced.

Prepared runtime limits are 256 MiB WASM, 8 MiB JS, 64 KiB descriptor and 4 KiB
embedded metadata. Output receipts are capped at 16 MiB. Project copying retains
the native 2 GiB/100,000-file bounds. The browser export currently rejects authored
physics/joints and sprite material overrides instead of claiming unsupported
host parity. Physical-target and clean-machine acceptance remain distinct gates. See evidence/task8-browser/ for actual captures/results.

## Task 8 native export observations — 2026-11-06

The real editor-exported saved-UI fixture contains a **86,627,928-byte debug
player** (CARGO_PROFILE_DEV_DEBUG=0) and **109,324-byte KAR**, plus a small JSON
receipt. These development profile sizes are informational.
The selected prebuilt runtime's profile/features determine executable size; the
exporter does not invoke Cargo or silently optimize it. A release runtime and
clean-machine dependency/ABI closure still need separate measurement/acceptance.
Package-size measurements are informational (ADR-0062). Export adds no gameplay rendering pass or draw-call
budget; no new game frame-time claim is made.

Export caps copied resources at 2 GiB and 100,000 files. Editor diagnostics keep
at most 2,000 lines of up to 4,096 characters through a bounded channel. Archive
verification/media decoding runs in the CLI process. Selected GUI/relocated-game
acceptance is retained in evidence/task8-native; Xvfb/software graphics and ALSA
null output do not establish physical target performance.

## Task 7 local saved-UI observations — 2026-11-05

The four-screen fixture uses a 22,352-byte renamed TTF subset; the font's 1024²
RGBA atlas costs 4 MiB on the GPU. Metrics are retained, while the native upload
buffer is temporary. The atlas is rebuilt at session startup, not per frame, and
prior UI atlas textures are released on reload. The document caps fonts at 16,
widgets at 1024/screen and quads at 16384; bitmap text has conservative startup
budget validation to prevent oversized screens.

A 1000-tick CPU-only title update/layout/draw observation (26 quads, 1280×720
viewport, dev build without debug symbols) measured 0.016422 ms mean / 0.016425 ms
p95 on this cloud runner. Reproduce with `KILN_UI_PROFILE=1 cargo test -p kiln-player
--test ui_workflow authored_screens_survive_restart_archive_and_editor_player_gameplay
-- --nocapture`. These numbers exclude VM/audio/physics, GPU upload and GPU frame
time; they are not reference-hardware release performance evidence.

Real native rendering and mouse/keyboard menu flows also passed on Xvfb/software
graphics. Full editor interaction, physical controller/audio hardware and release
size/performance acceptance remain open.

## Task 7 local audio observations — 2026-11-05

The generated native audio fixture contains 62,488 bytes of WAV data, decoded to
124,800 bytes of mono f32 PCM shared by overlapping voices. The voice pool is
capped at 128, including music; long clips still decode fully into memory.
No device latency/frame-time claim is established by offline
mixing and ALSA null-output checks. Native desktop debug binaries built with
debug symbols disabled for cloud disk limits; these are not release-budget
measurements. Real device, GUI and release performance acceptance remains open.

## Task 6 local observations — 2026-11-04

The visual authoring sample runs in Chromium WebGL with SwiftShader at 640×480.
A 120-frame `requestAnimationFrame` observation on the cloud runner measured
16.67 ms mean / 16.80 ms p95 cadence. This is debug-WASM browser scheduling
evidence, not GPU pass timing or a hardware performance guarantee. The real
native editor has no display server in this runner.

Debug artifacts measured locally: editor 389524776 bytes, player 325188200 bytes,
WASM 197188114 bytes. These include debug build overhead and are not release
release measurements. Package size is informational (ADR-0062).
The fixture's PNG sheets are 163 and 191 bytes. Mips require additional texture
storage; per-asset generation is optional. The existing 16384-quad renderer
budget is unchanged. Native visual/device acceptance is pending.

Reproduce with `WEB_SMOKE_PROJECT=samples/visual2d_editor WEB_SMOKE_PROFILE=1`
and `node tests/web/smoke.cjs target/web-smoke` after building WASM and running
wasm-bindgen; this runner also uses `CHROMIUM_PATH=/usr/bin/chromium`.

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

**Current implementation:** Task 7 decoded PCM mixer using Rodio (ADR-0055)

**Status:** Offline decoding/mixing and packed-player integration tested. Real audible device acceptance is pending. Full device management remains deferred to M49.

**Current capabilities:**
- Opens default audio output device via `DeviceSinkBuilder::open_default_sink()`
- Decodes WAV/OGG Vorbis/MP3/FLAC via Symphonia; independent SFX voices and optional looping music
- At most 128 active voices; replaces the oldest effect on overflow
- Shared decoded buffers cost 4 bytes per sample; long music currently decodes fully into memory
- Headless playback advances silently when no device is available
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

**Size policy:** informational only; no release-size goal (ADR-0062).

**Measurement:**
```bash
wasm-opt -Oz -o output.wasm input.wasm
brotli output.wasm -o output.wasm.br
ls -l output.wasm.br
```

**Measurement status:** optional local reporting uses `cargo build --release
--target wasm32-unknown-unknown -p kiln-web`, then `wasm-opt -Oz` and `brotli`
if installed. There is no size threshold and no size-failing CI job.

**Notes:**
- Measure with `wasm-opt -Oz` and brotli
- Include only runtime dependencies (no editor)
- Feature-gate unused capabilities where appropriate; functionality drives selection
- Earlier 36 KB measurements are historical, not current full-runtime evidence

---

## Native player binary

**Size policy:** informational only; no release-size goal (ADR-0062).

**Measurement:**
```bash
cargo build --release --bin kiln-player
ls -lh target/release/kiln-player
```

**Current status:** 930 KB (verified 2026-10-20, release build with LTO, panic=abort, strip=true)

**Notes:**
- Player binary contains only runtime, not editor
- Assets shipped separately in packed archive format
- Record the selected build profile with any measurement
- Measure separately from editor installer
- No editor crates in dependency tree (verified via cargo tree)
- Archive format (KILNARV1) size varies with content; sample: 68 bytes for 1 text file

---

## Android APK

**Size policy:** informational only; no release-size goal (ADR-0062).

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

**Size policy:** informational only; no release-size goal (ADR-0062).

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

**Size policy:** informational only; no release-size goal (ADR-0062).

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
cargo run -p render_graph --release
cargo test -p kiln-render --lib screenshot_low_tier_stable_hash
```

**Current status (M28, release, desktop, TIMESTAMP_QUERY available):**
- High-tier passes (clear/load): main ~32 µs, bloom/fog/outline/lut/composite ~18 µs each
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
- Purpose, licensing and maintenance implications
- Optional size observations when useful
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

## Active performance and resource goals

Package size does not block completion or release (ADR-0062). `just size` is an
optional report and does not claim to enforce runtime performance. Continue to
measure these existing goals on appropriate reference hardware:

- Startup time <2 seconds on reference hardware (after M14)
- Editor idle RAM <300 MB (after M6)
- Frame time <16.6 ms on reference device (after M12)

Record evidence and any performance/resource exceptions in STATE.md. File-count,
input-size and archive validation bounds are technical resource protections, not
release-size goals; they remain enforced.
