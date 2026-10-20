# Progress: F6 player loading, GPU tests, editor TODOs

Date: 2026-10-20

## Done

- **F6: kiln-player loading and spawning** (commit `9a7797b`, pushed)
  - `kiln-player/src/loader.rs` loads `.kar` archives and project directories (`kiln.toml` / `kiln.ron`).
  - `kiln-player/src/spawn.rs` spawns `Transform` and `Sprite` into `GameWorld`, including nested scenes with offsets and a cycle guard.
  - `kiln-player/src/main.rs` now loads the game, spawns the scene and runs the winit + wgpu render loop. It takes a `--scene` option.
  - Checks: `cargo check`, `cargo clippy` (0 warnings), 6 unit tests pass.
- `docs/STATE.md` marks F6 and Next-steps item 5 as done.
- GPU risk checked: `render-graph-sample` ran here and reported `TIMESTAMP_QUERY supported: true`, so a real adapter exists.

## Left to do

### 1. Spatial GPU regression tests (not done)
Attempted in `kiln-render/src/post_gpu.rs`, but the edits were corrupted by overlapping inserts and the file was **reverted to the committed version**. Nothing is half-applied.

Plan: add a `gpu_effect_tests` module next to the existing `post_chain_renders_on_gpu` test.
- Helper `test_device()` that skips when there is no adapter.
- Helper `run_case(device, queue, config, clear)` that renders offscreen and reads back RGBA8 bytes.
- Helper `pixel(data, width, x, y)` that handles the 256-byte row padding.
- Tests:
  - **fog**: blue scene with fog density 1.0 and a red fog colour; assert the centre pixel is red, and stays blue with fog off.
  - **bloom**: black scene with bloom on versus off; assert the pixels next to a bright area gain light.
  - **outline**: assert an edge pixel is tinted with the outline colour and a flat interior pixel is not.
- Caveat: a `clear` colour gives a flat image with no edges or bright spots. Bloom and outline need real geometry, e.g. draw sprites with `SpriteRenderer` into the scene target before `apply`.
- Tooling: write the code in small files of 6000 characters or less. Large inserts are what caused the corruption.

### 2. Editor TODOs in `kiln-editor/src/app.rs` (not started)
About 22 sites. Suggested order of what to implement:
- Add Entity, Add Component, Save Scene, Load Scene (RON/TOML via `kiln_core::Project`).
- Open / Save Project, Undo / Redo stack.
- Prefab create and instantiate, Create Scene, Add Nested Scene.
- Console Clear, Assets Import.
- Not worth implementing yet, leave as stubs: Export dialog, Package, Open docs, About.
- The "Proper FPS timing" TODO is also still open.

### 3. Validation (not done)
- `cargo fmt --all`, `cargo clippy --workspace --all-targets`, and the full `cargo test --workspace` run in chunks. The shell tool times out at 30s, so run long jobs in the background with output sent to a log file.
- Visual check of `B` / `O` / `F` post toggles in the demo on a real window.
- Rewrite `docs/STATE.md` at the end of the session.

## Notes
- Shell gotchas on this machine: no `grep`, `tee` or `tail`; PowerShell `2>&1` on cargo output reports false errors, so use `cmd /c "cargo ... > log 2>&1"`.
- One milestone per session per `AGENTS.md`; the GPU tests and editor TODOs should each get their own commit.
