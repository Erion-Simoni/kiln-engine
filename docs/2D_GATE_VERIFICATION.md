# 2D Gate Verification (G-1 … G-6) — Complete

**Goal:** Confirm every 2D-first gate item is met and testable, per `docs/TODO_LIST.md`
Phase 0.5. Verified against actual source code.

---

## Gate summary

| Gate | Item | State | Evidence |
|------|------|-------|----------|
| G-1 | Asset → GPU | Done | PNG decode, texture upload, sprite batching |
| G-2 | Real tilemap | Done | Chunked maps, camera culling, solid collision, editor panel |
| G-3 | VM maturity | Done | I64/F64/Bool/Str, strings, locals, tests |
| G-4 | Vertical slice | Done | `samples/vertical_slice/` — complete script-driven game |
| G-5 | Roadmap reconciled | Done | Verdicts match honesty audit |
| G-6 | Editor truth | Done | Engine-drawn scene view, save/load, undo, PIE |

---

## G-1: Asset → GPU

### Acceptance (from TODO_LIST.md)
- Decode PNG bytes to RGBA8 on the CPU ✅
- Register decoded images as GPU textures (`TextureId`) in the sprite renderer ✅
- Wire `Component::Sprite` in project data to a texture asset path ✅
- Player loads textures from `.kar` assets and binds them ✅
- Editor asset panel lists and previews imported images ✅

### Code evidence
| File | What it provides |
|------|-----------------|
| `kiln-assets/src/image.rs` | `decode_png()`, `encode_png_rgba()`, `Image` — all PNG colour types → straight RGBA8 |
| `kiln-render/src/texture.rs` | `TextureId`, `create_rgba8_texture()`, `FULL_UV`, `build_grid_atlas_rgba()` |
| `kiln-render/src/sprite_renderer.rs` | `SpriteRenderer::create_texture_rgba()`, batch-by-texture, `MAX_SPRITES = 16_384` |
| `kiln-core/src/project.rs` | `Component::Sprite { color, texture: Option<String> }` |
| `kiln-player/src/textures.rs` | `TextureLibrary::upload_for_project()` — decodes + uploads all project textures |
| `kiln-editor/src/assets.rs` | `AssetBrowser` — scans project dir, decodes PNG previews with egui |

### Tests
- `kiln-assets` image tests: PNG decode, palette/grayscale/tRNS expansion, round-trip
- `kiln-render` texture tests: UV rect, grid atlas
- `kiln-player` textures tests: texture path collection, asset lookup by exact path + file-name fallback

---

## G-2: Real tilemap

### Acceptance
- `kiln-tilemap` module with tileset definitions ✅
- Multiple layers per map, each with transform/z-order ✅
- Chunked storage (no entity per tile) ✅
- Camera culling of chunks ✅
- Tile collision data (solid flags) queryable ✅
- Editor panel to author tilemaps ✅

### Code evidence
| File | What it provides |
|------|-----------------|
| `kiln-core/src/tilemap.rs` | `TileSet`, `TileMap`, `TileMapLayer`, `TileChunk`, `CHUNK_SIZE=16`, `EMPTY_TILE`, solid flags, UV rects |
| `kiln-runtime/src/tilemap.rs` | `Tilemap` runtime component, `collect_visible()` (chunk culling), `first_solid_hit()` |
| `kiln-runtime/src/tilemap_collision.rs` | `entity_box()`, `first_solid_hit()`, `move_with_collision()` (axis-split, slide along walls) |
| `kiln-runtime/src/tilemap_scene.rs` | `spawn_tilemaps()` — binds project tilesets + GPU textures |
| `kiln-editor/src/tilemap_editor.rs` | `TilemapEditorState` — paint tiles, toggle solid, scroll by chunk |

### Tests
- `kiln-core` tilemap tests: tile UV, out-of-range, solid flag, chunk storage, layer z-order, solid hit, map size
- `kiln-runtime` tilemap tests: visible tile collection, chunk culling, empty view, solid query, world bounds

---

## G-3: VM maturity

### Acceptance
- `Value::Str` variant with equality and comparison ✅
- String constants in program encoding ✅
- String host-call arguments and results ✅
- Script frontend parses string literals into `Value::Str` ✅
- Graph nodes expose string constants ✅
- VM tests covering strings end to end ✅

### Code evidence
| File | What it provides |
|------|-----------------|
| `kiln-ir/src/value.rs` | `Value { I64, F64, Bool, Str }`, `values_equal()` (numeric-aware), `to_f64()` |
| `kiln-ir/src/vm.rs` | `Vm`, `HostContext`, `HostCall` instruction, string round-trip test |
| `kiln-ir/src/program.rs` | `Program`, `Instruction`, `Label`, `validate_jump_targets()` |
| `kiln-script/src/lexer.rs` | String literal tokenization with escapes |
| `kiln-script/src/parser.rs` | `Expr::String`, `Token::String` |
| `kiln-script/src/compiler.rs` | String constant compilation, `HostCall` emission |
| `kiln-graph/src/node.rs` | `ConstantValue::String` node type |
| `kiln-graph/src/compiler.rs` | Constant string compilation |

### Tests
- `kiln-ir` VM tests: string equality, string less-than, string host-call round-trip, host call keeps stack
- `kiln-script` tests: string literal lexing, string return, string equality

---

## G-4: Vertical slice

### Acceptance
- Script-driven project in `samples/vertical_slice/` ✅

### Code evidence
| File | What it provides |
|------|-----------------|
| `samples/vertical_slice/kiln.toml` | Project manifest: scene, entities (ground, player, 4 coins), roles, textures |
| `samples/vertical_slice/scripts/main.ks` | KilnScript: move player, collect coins, score, status |
| `samples/vertical_slice/README.md` | How to play, controls, entity reference |
| `kiln-player/tests/vertical_slice.rs` | Integration test for the player |
| `kiln-editor/tests/vertical_slice.rs` | Integration test for the editor |

---

## G-5: Roadmap reconciled

### Acceptance
- Checkboxes and verdicts reconciled with the audit ✅

### Code evidence
- `docs/ROADMAP.md` — M0–M17+ verdicts (Done / Partial / In progress)
- `ROADMAP_FULL_M14_PLUS.md` — M14–M61+ with per-milestone verdicts
- `docs/fixes m1-29.md` — M1–M29 findings with status legend (`[ ]` / `[x]` / `[~]` / `[-]`)
- `docs/STATE.md` — honesty audit summary

---

## G-6: Editor truth

### Acceptance
- GPU scene view, inspector save/load, undo, PIE viewport ✅

### Code evidence
| File | What it provides |
|------|-----------------|
| `kiln-editor/src/scene_view.rs` | Engine-drawn scene viewport (not egui), letterboxing, world camera = player camera |
| `kiln-editor/src/history.rs` | `SceneHistory` — undo/redo stack of scene snapshots |
| `kiln-editor/src/editing.rs` | `add_entity()`, `remove_entity()`, `set_role()`, `next_entity_id()` |
| `kiln-editor/src/app.rs` | `EditorApp` — PIE, scene view, inspector, all panels working |
| `kiln-editor/src/assets.rs` | `AssetBrowser` — PNG import + preview |
| `kiln-editor/src/prefab_editor.rs` | Prefab create/instantiate |
| `kiln-runtime/src/scene.rs` | `spawn_scene()` — shared by editor and player |

### Tests
- `kiln-editor` tests: scene history undo/redo, entity add/remove, role set, tilemap editor paint, asset browser select/preview

---

## Remaining known gaps (don't block M29)

| Gap | Why it doesn't block M29 |
|-----|-------------------------|
| `kiln-web` is a shell, not a full game loop | Documented in `docs/M29_BASELINE_AND_HARDWARE.md`; web game loop is Part 2a |
| Frame graph post passes are stubs in editor | Player runs real post chain; editor scene view lacks post effects |
| Player has no menu/loader UI | Not part of 2D gate |
| iOS/Android capability paths return fixed values | Platform-specific; deferred |
| Audio sample needs system audio backend | Checked separately via `samples/audio/Cargo.toml` |

---

## Sign-off

All six gate items (G-1 … G-6) are `[x]` with code evidence and tests.
M29 (3D foundation) is unblocked. See `ROADMAP_FULL_M14_PLUS.md` for the M29 plan
and the "Prepare before M29" checklist.

