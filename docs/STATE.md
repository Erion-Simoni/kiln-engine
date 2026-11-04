# Kiln Session Handoff

**Active focus:** Task 6 implementation complete; native workflow acceptance pending.
**Branch:** `feat/2d-production-workflow`. 3D is paused. Task 7 has not started.

## Delivered

- Image drag/drop creates a validated, undoable scene sprite using decoded image dimensions and camera-aware pointer placement. Saved `SpriteLayout` holds pivot, flips and pixel density; Inspector and Sprite Editor edit it. Runtime drawing/framing honors those properties and transform rotation/scale.
- Shared scene draw collection interleaves sprites, tile layers and particles. Equal sorting layers preserve authored hierarchy order across ECS archetypes. Renderer sorting preserves transparent submission order across texture/material changes, batching adjacent runs only.
- Per-asset nearest/linear filtering and mip generation reach native/WebGL uploads. Box-filtered mips include odd edge texels. Resource edits refresh the scene; texture rebinding removes obsolete project bindings. Legacy unconfigured assets retain nearest/no-mip sampling.
- Sprite grid slicing creates validated clips and attaches them. Animation Editor supports decoded texture preview, frame add/delete/reorder, rect/duration/pivot editing, named events/text payloads, clip undo/redo, scrubbing and non-looping next-clip transitions. Shared runtime binds real sheet dimensions/all project clips, switches textures, preserves time for repeated current-state requests and delivers crossed events to attached scripts.
- Tilemap Editor uses the selected layer's textured palette, bounded drag/erase/fill, multilayer Z/offsets, per-tile solids/collision previews, paged palettes, paint undo/redo and saved four-neighbor terrain rules. Map attachment preserves local parents/nested offsets; culling, bounds and collision queries account for layer offsets and owner translation. Solid maps automatically create merged static Rapier geometry when their owner has no explicit rigid body; grounding, contacts, ray identity, translation, cell edits and removal are tested.
- Saved `Particles` components use shared core emitter data. VFX Editor edits validated scene data through undoable commands and previews the same fixed simulation/quad geometry. Emitters have reproducible random streams. Particles retain their own gravity/fade/original alpha even after emitter removal. Start/stop toggles emission without destroying configuration.
- `samples/visual2d_editor` is generated through authoring APIs and saved normally. It supplies a blue animated character, idle transition/events, two tile layers, a solid wall and a particle trail. Authoring contracts and native acceptance steps are in `docs/VISUAL_AUTHORING_2D.md`; ADR-0054 records architecture choices.

## Validation

- `just check`: passed (formatting, strict all-target workspace Clippy, workspace and standalone audio sample checks).
- `just test`: **592 passed, zero failed/skipped**. Task 5 physics regressions remain passing.
- `cargo build -p kiln-editor -p kiln-player`: passed.
- `cargo clippy -p kiln-runtime --no-default-features --all-targets -- -D warnings`: passed.
- `cargo build --target wasm32-unknown-unknown -p kiln-web` and matching wasm-bindgen generation: passed.
- Native player acceptance authors/saves/reopens, packs a real KAR archive, loads source/archive with the actual player loader, and compares movement, animation events/transitions, particle geometry and sorting at 30 vs 144 render FPS. Authored data is unchanged by gameplay.
- Headless egui acceptance inspects the decoded preview texture mesh and actual frame UVs/events. Additional tests cover clip history, terrain adjacency/erase/fill limits, component history/disk projection, pivots, transparent painter order, nested map offsets, particle preview/runtime parity and mip texels.
- Chromium WebGL: new visual sample passed visible-pixel/input/release/zero-error checks. Original vertical slice passed. A temporary copy with linear filtering and generated mips also passed. The 120-frame debug-WASM cadence observation was 16.67 ms mean / 16.80 ms p95 with SwiftShader at 640×480; see BUDGETS.md for scope and debug artifact sizes.
- Actual CLI packing passed: `cargo run -p kiln-cli -- pack --source samples/visual2d_editor --output target/visual2d_editor.kar` packed six files successfully.
- Built editor and native player startup were attempted. Both stop at window creation because DISPLAY/Wayland variables are absent. Native drag/drop/panel interaction and graphical packed-player acceptance remain unverified. Browser rendering is separate evidence.

## Supported limits / next

- Task 6 implementation boxes are checked; native workflow acceptance remains pending. Follow the graphical procedure in VISUAL_AUTHORING_2D.md. Do not start Task 7 without the user's next instruction.
- Real archive/runtime export is exercised; the editor's automated Build/Export panel and distribution bundling remain Task 8.
- Clip grids are tight/spaced uniform rectangles, capped at 4096 frames. Per-frame pivots override sprite layout pivots while animated. Transitions run after non-looping clips finish. Clip and paint histories are resource-specific; component/sprite edits use scene history.
- Tile fill uses explicit finite bounds (the UI fills the visible 16×16 area). Terrain variants use cardinal neighbor masks on one selected layer. Tile collision is axis-aligned; author grid scale through map world tile size and offsets.
- Particle capacity and the existing 16384-quad renderer frame budget are distinct. Native visual/device/release-size gates remain pending; debug build observations do not certify release budgets.
- Task 5 physics/camera implementation remains in place; its native visual acceptance is also pending. Browser/mobile physics host integration is separate from this visual workflow.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Git push/local HTTP browser tests use the supported `require_escalated` permission in this runner. Prior remote tip was `985cbd1`; this session's commit records Task 6.
