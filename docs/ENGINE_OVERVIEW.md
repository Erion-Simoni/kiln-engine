# Kiln Engine Overview

How all crates fit together, the frame lifecycle, and how a KilnScript command travels from source code to pixels on screen.

## Crate Map

| Crate | Responsibility |
|-------|---------------|
| `kiln-core` | Math, time, shared types, ECS glue, feature-gated 3D mesh/material types |
| `kiln-render` | wgpu renderer, tiers, post, 2D batching → 3D, frame graph, GPU timestamps |
| `kiln-assets` | PNG decoding, content-hash cache, packed `.kar` archives |
| `kiln-ir` | IR bytecode, VM, host call interface |
| `kiln-script` | KilnScript lexer/parser/compiler → IR |
| `kiln-graph` | Node graph model → IR compilation |
| `kiln-runtime` | Fixed-timestep loop, ECS world, scene spawning, schedule/jobs, slice runtime |
| `kiln-editor` | egui shell, scene view, panels, PIE, undo, asset browser, tilemap editor |
| `kiln-cli` | `init`, `import`, `pack`, `export`, `publish` |
| `kiln-player` | Thin standalone game launcher (loads `.kar` or project dir) |
| `kiln-web` | WASM export shell (stub — no game loop yet) |
| `kiln-ui` | In-game UI widgets, text renderer, nine-slice |
| `kiln-input` | Action maps, key/gamepad bindings |
| `kiln-save` | Versioned save slots with migration |
| `kiln-anim` | Spritesheet clips, tween easing curves |
| `kiln-vfx` | CPU + GPU particle systems |
| `kiln-gameplay` | Data tables, dialogue, quests |
| `kiln-ai` | Navmesh, grid pathfinding |
| `kiln-platform` | Platform services (achievements, leaderboards, cloud saves, IAP) |
| `kiln-updater` | Delta patching, in-game updates (feature-gated stubs) |
| `kiln-plugin-api` | Sandboxed WASM plugin ABI |
| `samples/` | Tutorial + sample games |

## Data Flow: Authoring → Running

```
kiln.toml + PNG art + .ks scripts
        │
        ▼
  kiln-cli init (scaffolds project)
        │
        ▼
  kiln-player loads Project + assets
        │
        ▼
  Scene spawning: entities → GameWorld (hecs ECS)
        │
        ▼
  KilnScript runs once per frame (SliceSession)
        │
        ▼
  Script emits commands (move_player, collect_coin, add_score)
        │
        ▼
  Runtime applies commands to world
        │
        ▼
  SpriteRenderer batches sprites by texture
        │
        ▼
  Renderer draws to swap chain (or post framebuffer)
        │
        ▼
  Post-processing (bloom/fog/outline/LUT) if tier != Low
        │
        ▼
  Pixels on screen
```

## Frame Lifecycle

Every frame (60 Hz fixed timestep):

1. **Input** — winit events collected into `InputState`
2. **KilnScript step** — `SliceSession::step(world, input_axis, dt)`:
   - Script reads facts (`get_input_x`, `get_score`, `get_coin_count`, etc.)
   - Script emits commands (`move_player`, `collect_coin`, `add_score`, `set_status`)
   - Runtime applies commands to `GameWorld` (hecs ECS)
3. **ECS systems** — Run in parallel stages via `Schedule` (non-conflicting access sets)
4. **Render** — `SpriteRenderer` collects sprites/tilemaps, batches by `TextureId`, draws
5. **Post-processing** — If tier != Low, scene rendered to intermediate target, then bloom/fog/outline/LUT applied
6. **HUD** — Screen-space text drawn with `UiRenderer` + `TextRenderer`
7. **Present** — Swap chain presented, GPU profiler timestamps resolved

## How a KilnScript Command Reaches the Screen

Example: `move_player(3.0, 4.0);`

1. **Lexer/Parser** — `scripts/main.ks` → tokens → AST (`kiln-script`)
2. **Compiler** — AST → IR bytecode (`kiln-ir`) with `HostCall { id: 8, argc: 2 }`
3. **VM** — `SliceSession` executes the program once per frame
4. **Host call** — VM calls host function id 8 (`move_player`) with args `[3.0, 4.0]`
5. **Runtime** — `SliceSession` records a `SliceCommand::MovePlayer { dx: 3.0, dy: 4.0 }`
6. **World update** — Command applied: player `Transform` position updated in `GameWorld`
7. **Render** — `SpriteRenderer` reads `Transform` + `Sprite` from ECS, pushes instance
8. **GPU** — Instanced draw call batches all sprites sharing the same `TextureId`

## The 2D-First Gate (G-1 … G-6) — Complete

| Gate | Item | Status |
|------|------|--------|
| G-1 | Asset → GPU (PNG decode, texture upload, sprite batching) | Done |
| G-2 | Real tilemap (chunked, culling, solid collision, editor panel) | Done |
| G-3 | VM maturity (I64/F64/Bool/Str, strings, locals, tests) | Done |
| G-4 | Vertical slice sample (`samples/vertical_slice/`) | Done |
| G-5 | Roadmap reconciliation (verdicts match audit) | Done |
| G-6 | Editor truth (engine-drawn scene view, save/load, undo, PIE) | Done |

M29 (3D foundation) is in progress. See `ROADMAP_FULL_M14_PLUS.md`.

## Render Tiers

| Tier | Post-processing | Use case |
|------|----------------|----------|
| Low | None | Integrated GPUs, web, old hardware |
| Medium | Bloom, fog | Mid-range hardware |
| High | Bloom, fog, outline, LUT | Discrete GPUs |

Tier is auto-selected from the GPU adapter (`RenderTier::auto_select`) and can be overridden at runtime.

## Save System

Slice state (score, elapsed, player position, collected coins) serializes to JSON via `kiln-save` (`SaveSlotManager`). Written to platform save folders (AppData/Application Support/XDG). F5 saves, F9 loads.

## See Also

- `docs/API_REFERENCE.md` — Complete API reference
- `docs/PROJECT_FORMAT.md` — `kiln.toml` schema
- `docs/KILNSCRIPT.md` — KilnScript language reference
- `docs/TILEMAPS.md` — Chunked tilemaps with collision
- `docs/ARCHITECTURE.md` — Crate map, data flow, IR design (detailed)