# 3a — `3d` Feature Gate Verification (Pre-M29)

**Goal:** Confirm the `3d` feature gate in `kiln-core` is watertight — a 2D build (default features, no `3d`) pulls in zero 3D code, and `cargo check` passes both with and without `3d`.

**Why:** M29's acceptance includes "`3d` off: 2D web build size unchanged (measured)." If the gate leaks 3D types into the 2D path, the web WASM size budget breaks.

## Current state (verified from source)

| File | `3d`-gated? |
|------|-------------|
| `kiln-core/Cargo.toml` | `3d = ["glam/serde"]`, `default = []` ✅ |
| `kiln-core/src/lib.rs` | `pub mod mesh;` unconditional; `pub use mesh::{...}` unconditional |
| `kiln-core/src/mesh.rs` | Every type has `#[cfg(feature = "3d")]` real impl + `#[cfg(not(feature = "3d"))]` stub ✅ |
| Other crates | No crate references `kiln-core::mesh` types yet |

## What to verify (when `cargo` is available)

```
# 1. Default (no 3d) build — must compile, no 3D code
cargo check --workspace

# 2. With 3d feature — must compile
cargo check --workspace --features kiln-core/3d

# 3. Confirm nothing in the 2D path imports mesh types
cargo tree -i kiln-core --features kiln-core/3d  # inspect reverse deps
grep -rn "kiln_core::mesh\|use kiln_core::.*Mesh\|kiln_core::Material" --include="*.rs" kiln-render kiln-runtime kiln-assets kiln-player kiln-editor
```

## Expected result

| Check | Pass condition |
|-------|---------------|
| `cargo check --workspace` | ✅ compiles, no errors |
| `cargo check --features kiln-core/3d` | ✅ compiles |
| 2D path imports `mesh` types | ❌ zero matches |
| `mesh` module compiles real types only with `3d` | ✅ `#[cfg(feature = "3d")]` on all real impls |

## If a leak is found

If any 2D crate imports `kiln_core::Mesh`/`Material` etc. without `#[cfg(feature = "3d")]`, move that import behind the feature gate or delay it to M29. Do not start M29 renderer work until this is clean.
