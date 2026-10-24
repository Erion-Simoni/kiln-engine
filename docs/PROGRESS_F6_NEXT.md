# Progress: F6 player loading, GPU tests, editor TODOs

Date: 2026-10-20 (session)

**Status: All items complete.** This page records work from the F6 session. Everything listed below has since landed and is reflected in `docs/STATE.md`, `docs/TODO_LIST.md` and the 2D-first gate (G-1 … G-6) checklists.

## What Landed (from this session's plan)

### F6: kiln-player loading and spawning (done)
- `kiln-player/src/loader.rs` loads `.kar` archives and project directories (`kiln.toml` / `kiln.ron`).
- `kiln-player/src/spawn.rs` spawns `Transform` and `Sprite` into `GameWorld`, including nested scenes with offsets and a cycle guard.
- `kiln-player/src/main.rs` loads the game, spawns the scene and runs the winit + wgpu render loop with a `--scene` option.
- Texture decoding and GPU upload via `TextureLibrary` (PNG → `SpriteRenderer::create_texture_rgba`).

### Editor truth (G-6, done)
- Scene view drawn by the engine's sprite renderer (not egui): `kiln-editor/src/scene_view.rs`.
- Inspector edits saved to disk with undo/redo: `kiln-editor/src/history.rs`.
- All previously-stubbed editor panels now work: add/remove entities and components, prefabs, nested scenes, export/package, asset import, console clear, about/docs.

### Post-processing (F5, done)
- Bloom, fog, outline and LUT grading, each with its own GPU test in `kiln-render/src/post_effects_test.rs`.

### 2D gate (G-1 … G-6, complete)
All six gate items are `[x]` in `docs/TODO_LIST.md`. M29 (3D foundation) is in progress — see `ROADMAP_FULL_M14_PLUS.md`.

## Left to Do

None from this session. The next milestone is M29 (3D core). Before continuing M29, finish the "Prepare before M29" checklist in `ROADMAP_FULL_M14_PLUS.md`:

1. Verify the `3d` feature gate builds clean with and without the feature.
2. Decide the glTF import approach (ADR needed for any new crate).
3. Scope renderer pieces (depth buffer, perspective camera, frustum culling) behind `3d`.
4. Plan the 3D editor scene view with gizmos (builds on ADR-0032).
5. Record the `3d`-off 2D web WASM size baseline (`just size`).
6. Pick a reference Low/Medium GPU for old-hardware testing.

## See Also

- `docs/STATE.md` — current session handoff
- `docs/TODO_LIST.md` — 2D gate checklists and remaining items
- `ROADMAP_FULL_M14_PLUS.md` — M29 plan and "Prepare before M29" checklist
