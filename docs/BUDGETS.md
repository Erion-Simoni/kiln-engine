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

**Current status:** Not yet measured (record after M0)

**Notes:**
- Measure with `wasm-opt -Oz` and brotli
- Include only runtime dependencies (no editor)
- Feature-gate physics/audio to stay under budget
- Record real numbers in STATE.md after M0

---

## Editor installer

**Target:** <40 MB

**Measurement:**
```bash
cargo-dist build
ls -lh dist/kiln-*.msi  # or .dmg, .AppImage
```

**Current status:** Not yet measured (record after M12)

**Notes:**
- Use cargo-dist for packaging
- Strip symbols, use LTO, panic=abort
- Exclude debug symbols and large assets from installer

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
- Editor installer >40 MB (after M12)
- Editor idle RAM >300 MB (after M6)
- Frame time >16.6 ms on reference device (after M12)

Temporary exemptions must be documented in STATE.md with a plan to fix.
