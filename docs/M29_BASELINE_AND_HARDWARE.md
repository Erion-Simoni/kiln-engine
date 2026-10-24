# 3e & 3f — Size Baseline + Old-Hardware Test Target (Pre-M29)

**Goal:** Record the `3d`-off 2D web WASM size baseline and pick a reference
Low/Medium GPU for old-hardware testing. Both are M29 acceptance criteria.

---

## 3e — Size baseline (`3d`-off 2D web build)

**M29 acceptance:** "`3d` off: 2D web build size unchanged (measured)."

### How to measure (when `cargo` + `wasm-opt` + `brotli` are available)

```bash
# 1. Build the web runtime with 3d OFF (default)
cargo build --release --target wasm32-unknown-unknown -p kiln-web

# 2. Optimize + compress
wasm-opt -Oz -o kiln_web.opt.wasm \
  target/wasm32-unknown-unknown/release/kiln_web.wasm
brotli kiln_web.opt.wasm -o kiln_web.opt.wasm.br

# 3. Record the size
ls -lh kiln_web.opt.wasm.br
# → this is the baseline; M29 must not increase it
```

### Where to record

Add the measured value to `docs/BUDGETS.md` under "Web runtime WASM":

> **Current status:** <size> brotli-compressed (measured <date>, `3d` off).

### Acceptance

| Check | Pass condition |
|-------|---------------|
| `3d`-off WASM size | Recorded in BUDGETS.md |
| `3d`-on build compiles | `cargo check --features kiln-core/3d` passes |
| Size unchanged after M29 | Post-M29, re-measure; must match baseline |

---

## 3f — Old-hardware test target

**Principle 8** (from `ROADMAP_FULL_M14_PLUS.md`): "Test on real old hardware
at the end of every phase, not only on your development machine."

### Reference hardware (pick one Low + one Medium)

| Tier | GPU class | VRAM | Target FPS | Test focus |
|------|-----------|------|------------|------------|
| Low | Intel HD 520 / integrated 2015 laptop | 1–2 GB | 30–60 | 2D sprite batching, tilemap culling, post off |
| Medium | GTX 1050 / equivalent discrete | 2–4 GB | 60 | Bloom, fog, outline, particle systems |

### Test procedure

1. Build the release binary: `cargo build --release -p kiln-player`
2. Run the vertical slice: `cargo run -p kiln-player -- --archive samples/vertical_slice --release`
3. On the reference GPU, confirm:
   - 60 FPS (or at least 30 on Low) with the default tier
   - Sprite batching draws 100+ sprites in one draw call
   - Tilemap culling skips off-screen chunks
   - Save/load (F5/F9) works
4. Record results in `docs/BUDGETS.md` or `docs/STATE.md` "Known issues".

### If hardware is unavailable

Use a VM with a limited GPU, or warp the dev GPU via driver settings to
emulate a Low tier. Document the substitution in STATE.md.

---

## See Also

- `docs/BUDGETS.md` — size and performance budgets
- `ROADMAP_FULL_M14_PLUS.md` — Principle 8 (test on real old hardware)
- `M29_FEATURE_GATE_VERIFICATION.md` — `3d` feature gate check
- `M29_RENDERER_PLAN.md` — depth buffer, perspective camera, frustum culling
- `M29_EDITOR_3D_PLAN.md` — editor 3D view + gizmos
