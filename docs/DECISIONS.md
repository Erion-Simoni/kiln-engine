# Kiln Architecture Decision Records

Date format: YYYY-MM-DD. Status: proposed | accepted | rejected | deprecated.

---

## ADR-0001: Editor UI framework (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Use pure-Rust native (`winit` + `wgpu` + `egui`) for the editor. Do not use Electron. Tauri only if justified by a future ADR.

**Alternatives:**
- Electron: large install size, heavy resource usage
- Tauri: web tech overhead, adds complexity
- Pure Rust: smallest binary, best performance, single toolchain

**Consequences:**
- Editor installer stays under 40 MB target
- Native UI may have less polish than mature web frameworks initially
- Need to evaluate node-graph UI (egui-snarl vs custom) in M8

---

## ADR-0002: ECS choice (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Evaluate standalone `bevy_ecs 0.20.0` vs `hecs 0.11.2` vs tiny custom. Decision criterion: binary size + compile time for `kiln-web` and editor.

**Alternatives:**
- bevy_ecs: feature-rich, larger binary, slower compile
- hecs: simpler, smaller, faster compile
- Custom: minimal, but maintenance burden

**Consequences:**
- Must be resolved in M1 before ECS integration
- Size impact must be recorded in BUDGETS.md

---

## ADR-0003: Scripting language (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Ship **Rhai 1.26.1** behind the `LanguageFrontend` trait for M5 speed-to-editor, with a path to a purpose-built **KilnScript** if Rhai size/semantics fight budgets.

**Alternatives:**
- Rhai: fast to integrate, mature, but may be too large/heavy
- Custom KilnScript: optimized for engine, but slower to build
- Other (Lua, Wren, etc): evaluation deferred

**Consequences:**
- Trait allows swapping later without breaking IR/VM
- Size impact must be measured in M5 and recorded

---

## ADR-0004: egui version pin for node graph (proposed)

**Date:** 2026-10-18  
**Status:** proposed

**Decision:** Pin **egui 0.36.x** for M6–M8 to use `egui-snarl 0.12.0` and `egui_dock 0.21.1`, which require egui ^0.36. Upgrade to 0.37+ when node-graph ecosystem catches up.

**Alternatives:**
- Pin 0.36: stable node-graph UI, but miss latest egui features
- Custom node graph: more control, but more code to maintain
- Wait for 0.37 support: delays M8

**Consequences:**
- Temporary version pin until ecosystem matures
- Document upgrade path in STATE.md when ready

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
