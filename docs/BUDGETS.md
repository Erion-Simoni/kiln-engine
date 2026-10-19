# Kiln Budgets

All budgets are enforced via `just size`. Exceeding a budget requires an ADR entry with justification.

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

**Current status:** 930 KB (measured in M14, release build with LTO, panic=abort, strip=true)

**Notes:**
- Player binary contains only runtime, not editor
- Assets shipped separately in packed archive format
- Target is for stripped release build (LTO, panic=abort, strip=true)
- Measure separately from editor installer
- No editor crates in dependency tree (verified via cargo tree)
- Archive format (KILNARV1) size varies with content; sample: 68 bytes for 1 text file

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

**Current status:** Not yet measured (record after M14 or when player binary exists)

**Notes:**
- Measure on desktop reference hardware (similar to frame budget reference)
- Exclude time spent loading large game assets (measure with minimal sample)
- Archive loading should be fast; defer to M14 when streaming loaders are implemented
- Critical for perceived performance

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

## Dependency size tracking

Every new dependency must have an ADR entry with:
- Binary size impact (measured via `cargo bloat` or similar)
- Compile time impact
- Justification for inclusion

**Tools:**
- `cargo-bloat`: identify largest functions
- `twiggy`: WASM size analysis
- `cargo-size`: binary size per crate

---

## Budget enforcement

CI will fail `just size` if:
- Web WASM >1.5 MB brotli (after M10)
- Native player binary >15 MB (after M14)
- Editor installer >40 MB (after M12)
- Startup time >2 seconds on reference hardware (after M14)
- Editor idle RAM >300 MB (after M6)
- Frame time >16.6 ms on reference device (after M12)

Temporary exemptions must be documented in STATE.md with a plan to fix.
