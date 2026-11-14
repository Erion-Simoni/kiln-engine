# Kiln: Roadmap to GameMaker / Unity 2D Level

> **Historical plan / technical reference.** The [unified engine completion plan](ENGINE_COMPLETION_PLAN.md) is authoritative for current scope, order, status and release gates. Older completion claims and package-size goals below are historical; preserve their evidence, but do not use them to close current milestones.


**Status:** Recommended plan to follow — the 2D production-readiness roadmap for Kiln.
**Goal:** Kiln 2D v1.0: a complete, standalone 2D game development environment, reached and verified before serious 3D development begins.

## How to follow this plan

Work one stage at a time. Each stage is independently testable and must be completed — with its acceptance criteria met — before moving to the next. Do not implement this entire roadmap in one operation; take one milestone at a time, with a testable acceptance criterion.

This plan runs alongside `docs/ROADMAP.md`, `ROADMAP_FULL_M14_PLUS.md`, `docs/EDITOR_ROADMAP.md`, and `docs/STATE.md`. For 2D v1.0 work, this plan takes precedence where it differs from existing milestone docs; 3D work (M29+) proceeds only after the 2D v1.0 release gate in Section 5 is passed.

---

## My recommendation

Make Kiln 2D Production Ready (v1.0) a separate achievement before beginning serious 3D development.

The objective should not be to reproduce every feature in Unity. That would make Kiln unnecessarily large and undermine the original goal: a lightweight, optimized engine that can run on less powerful computers.

Instead, Kiln should reach a point where a developer can independently create, edit, test, debug, optimize, export and distribute a complete 2D game without needing another engine.

There are three different things to achieve:

1. **Feature capability:** enough tools to create different genres of 2D games.
2. **Developer experience:** the editor, scripting, asset management and workflow must feel like a real engine.
3. **Production reliability:** exported games must work, perform consistently and preserve their data.

All three matter. Having 50 features that aren't properly connected is not parity with GameMaker or Unity.

---

## The target

**Kiln 2D v1.0 — target before 3D**

A complete, standalone 2D game development environment.

| Area | Scope |
|------|-------|
| 2D | Rendering, physics, animation, UI, tilemaps |
| Editor | Scene editor, inspector, assets, debugger |
| Runtime | Scripting, input, audio, save/load, AI |
| Export | Windows, Linux, macOS, web, Android, iOS |

**Definition of completion:** A developer can build a complete platformer, top-down game and UI-heavy game using only Kiln.

---

## 1. What GameMaker and Unity 2D actually provide

Use them as two different references.

| Capability | GameMaker | Unity |
|------------|-----------|-------|
| Scene/room editor | Core | Core |
| Sprite editor | Built-in image tools | Sprite importer/editor |
| Tilemaps | Brushes and autotiling | Tilemap and extras |
| Animation | Sprites and sequences | Frame and skeletal animation |
| Physics | 2D physics | Advanced 2D physics |
| Scripting | GML and visual tools | C# |
| Visual effects | Sequences and effects | Particles and graphics |
| Lighting | 2D visual effects | Dedicated 2D lights |
| Debugging | Integrated debugger | Integrated debugger and profiler |
| Export | Multiple platforms | Multiple platforms |

GameMaker's room editor, object editor, sequences, tile tools and debugger are important workflow references.

Unity provides additional depth in sprite importers, skeletal animation, spline-based worlds, 2D lighting, tilemaps and physics.

**Recommendation:** Aim for GameMaker-level convenience and core feature coverage, while selectively adopting Unity's more advanced 2D technology.

Do not try to recreate Unity's entire editor or package ecosystem.

---

## 2. The actual implementation roadmap

Divide the work into 17 development stages. Each stage should be independently testable and completed before moving to the next.

### Stage 0 — Stabilise the existing engine

**First priority — Foundation**

Before adding anything, fix the existing project.

**What to fix:**

1. Resolve the current hecs and ECS compilation errors.
2. Repair `SliceSession` and `SliceHost` ownership.
3. Fix the physics/runtime interfaces.
4. Repair the web runtime syntax errors.
5. Fix malformed integration tests.
6. Fix CI, mdBook and cargo-dist configuration.
7. Run formatting, Clippy, checks and all tests successfully.

**Completion criteria:**

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo test --workspace
```

Also verify the web target and optional features independently.

Don't accept a green build from only one crate. The whole workspace must build.

### Stage 1 — Build a general-purpose runtime

**Major architectural work**

This is arguably the most important engine-level improvement.

Currently, `kiln-runtime/src/slice.rs` contains functionality specifically designed around the sample game.

Replace this with a general-purpose runtime architecture.

**Add:**

- Generic entities and components.
- Generic component registration.
- Entity creation and destruction at runtime.
- Entity lookup by stable ID.
- Component access from scripts.
- Event and message systems.
- Runtime scene management.
- A shared host-function registry.
- Script lifecycle events.
- Generic state management.

For example, a game script should be able to:

```text
let player = find_entity("player");
let position = get_position(player);

set_position(
    player,
    position.x + 10.0,
    position.y
);
```

The actual API syntax can be decided later. The important part is that the runtime should work for any game, not only the vertical slice.

**Completion criteria:** Create two completely different sample games without changing the engine's core runtime code.

---

### Stage 2 — Complete project and asset management

**Major editor work**

Unity and GameMaker both make managing game resources central to the workflow.

Kiln needs a genuinely useful asset browser.

**Add:**

- Asset search and filtering.
- Folder creation and organisation.
- Import and reimport.
- File-change detection.
- Asset dependency tracking.
- Stable asset IDs.
- Missing asset detection.
- Duplicate asset detection.
- Asset previews.
- Drag-and-drop into scenes.
- Import settings per asset.
- Automatic texture atlas generation.

**Supported formats:**

| Format | Priority |
|--------|----------|
| PNG | Essential |
| JPG/JPEG | Essential |
| WAV | Essential |
| OGG | Essential |
| MP3 | Useful |
| WebP | Useful |
| Aseprite | High |
| PSD | Later |
| SVG | Later |

**Important:** don't create a replacement for Photoshop or Aseprite. Build importers and useful sprite editing tools that integrate with those applications.

**Completion criteria:** Import an asset, use it in a scene, change its source file, and have Kiln recognise and update the asset without manually rebuilding the project.

### Stage 3 — Complete the scene editor

**Core editor**

The scene editor needs to become one of Kiln's strongest components.

**Add:**

- Scene hierarchy.
- Drag-and-drop entities.
- Parent-child relationships.
- Multi-object selection.
- Group movement and scaling.
- Copy, paste and duplicate.
- Grid snapping.
- Alignment tools.
- Locked and hidden objects.
- Multi-scene tabs.
- Scene transition settings.
- Unsaved-change indicators.
- Automatic saving and recovery.

Extend the existing inspector with editable properties for every supported component.

**Also add — Camera system:**

- Multiple cameras.
- Camera follow.
- Camera bounds.
- Zoom.
- Camera shake.
- Smooth transitions.
- Dead zones.
- Split-screen support.

**Completion criteria:** Create a scene with 100 entities, organise them into groups, save it, close Kiln, reopen it and recover the exact same scene.

### Stage 4 — Professional 2D sprite and rendering system

**High priority**

Kiln already has a good renderer. Don't rewrite it. Improve the tools and capabilities surrounding it.

**Add — Sprite functionality:**

- Sprite sheets.
- Automatic sprite slicing.
- Manual slicing.
- Sprite pivots.
- Sprite flipping.
- Sprite masks.
- Sprite outlines.
- Sprite sorting layers.
- Per-sprite material settings.

**Rendering improvements:**

- Texture atlas packing.
- Automatic batching.
- Transparent sorting.
- Pixel-perfect rendering.
- Parallax backgrounds.
- Camera viewport scaling.
- Texture filtering.
- Mipmaps where appropriate.
- Texture compression per platform.

Your existing sprite editor already has pivot, slicing and 9-slice foundations. Build upon those rather than duplicating them.

**Completion criteria:** Import a large sprite sheet, slice it, create an atlas, render hundreds or thousands of sprites correctly, and verify sorting and texture behaviour.

---

### Stage 5 — Make the tilemap editor competitive

**Existing foundation**

Kiln has chunked tilemap storage and rendering. Now improve the level-authoring experience.

**Add:**

1. Tile palette.
2. Brush painting.
3. Eraser.
4. Rectangle fill.
5. Flood fill.
6. Grid controls.
7. Multiple tile layers.
8. Layer visibility and locking.
9. Automatic tiles / Rule Tiles.
10. Animated tiles.
11. Per-tile collision.
12. Tilemap collision preview.
13. Brush presets.
14. Large-world editing improvements.

**Then add an optional advanced feature: Spline-based 2D terrain.**

This lets developers create curved platforms, roads, rivers and decorative outlines without manually placing every tile.

**Completion criteria:** Build a complete platformer level with multiple layers, automatic terrain, collision and a large map without performance degradation.

### Stage 6 — Make KilnScript a proper game scripting language

**Very high priority**

Keep the custom Rust-based scripting VM. It is an important part of Kiln's identity.

However, the current language is too restrictive for developers to make larger games.

**Add these features in order:**

| Feature | Priority |
|---------|----------|
| `while` and `for` loops | Essential |
| User-defined functions | Essential |
| Arrays and dictionaries | Essential |
| Return values and parameters | Essential |
| Global variables | Essential |
| Enums and structured values | High |
| Modules and imports | High |
| Coroutines | High |
| Timers and delayed execution | High |
| Events and signals | High |
| Exception/error reporting | High |
| Script hot reload | High |
| Async operations | Later |

**Also improve:**

- Syntax highlighting.
- Autocomplete.
- Function documentation.
- Error squiggles.
- Call stacks.
- Variable inspection.
- Source-level debugging.
- Script performance budgets.

**Important requirement:** Scripts and visual graphs should compile against the same host-function registry and runtime API. Don't maintain two independent sets of gameplay functionality.

**Completion criteria:** Implement a platformer, inventory, enemy AI and dialogue using KilnScript without modifying the engine's Rust source.

### Stage 7 — Complete animation and sequencing

**High priority**

This is an area where GameMaker and Unity provide much more than simple sprite animation.

Kiln needs three separate animation capabilities.

**A. Sprite animation**

- Frame-by-frame animation.
- Animation playback controls.
- Looping and ping-pong.
- Frame events.
- Playback speed.
- Animation blending.
- Animation state machines.

**B. 2D skeletal animation**

- Bone creation.
- Sprite attachment to bones.
- Skinning.
- Inverse kinematics.
- Animation blending.
- Character rig editing.

**C. Timeline / sequencer**

- Keyframe editor.
- Position, rotation and scale animation.
- Color and opacity animation.
- Camera animation.
- Audio tracks.
- Event tracks.
- Cutscenes.
- Animation curves.

Implement sprite animation first, the timeline second and skeletal animation third.

**One existing issue to correct:** `update_animations()` currently advances animations using a hardcoded 1/60 timestep instead of the actual frame delta.

**Completion criteria:** Create an animated character, an animation state machine and a cutscene entirely in the editor.

### Stage 8 — Finish 2D physics

**Currently incomplete**

The Rapier integration is a good foundation, but it needs to become a reliable engine-level system.

**Implement and test:**

| Feature | Requirement |
|---------|-------------|
| Rigidbody | Static, dynamic, kinematic |
| Colliders | Box, circle, capsule, polygon |
| Sensors | Trigger detection |
| Collision layers | Configurable filtering |
| Collision events | Begin, stay, end |
| Raycasting | Real physics queries |
| Shape casting | Predictive collision queries |
| Joints | Distance, hinge, slider |
| Motors | Configurable motors |
| Continuous collision | Fast-moving bodies |
| Physics materials | Friction and restitution |
| Debug drawing | Collider and body visualisation |

**Also add a dedicated 2D character controller.** It should support:

- Ground detection.
- Slopes.
- Jumping.
- Moving platforms.
- Coyote time.
- Jump buffering.
- Wall detection.
- One-way platforms.

The character controller should be reusable rather than implemented separately for each sample.

**Completion criteria:** A physics sample with a platformer character, moving platforms, sensors, joints and collision events must run correctly in the editor and player.

---

### Stage 9 — Complete input management

**Input system**

The action map and rebinding work are good foundations.

**Add:**

- Keyboard input.
- Mouse buttons and movement.
- Mouse wheel.
- Multiple gamepads.
- Gamepad hot-plugging.
- Controller-specific glyphs.
- Customisable dead zones.
- Analog sticks.
- Triggers.
- Touch input.
- Multi-touch gestures.
- Mobile virtual controls.
- Input profiles per player.
- Saved input settings.
- Controller vibration where supported.

Every input should be accessible through one unified action system. For example:

```
Move
Jump
Attack
Interact
Pause
Inventory
```

Developers should not have to implement different gameplay logic for keyboards, controllers and mobile touchscreens.

**Completion criteria:** The same sample game operates using keyboard, Xbox controller, PlayStation controller and touchscreen without changing gameplay scripts.

### Stage 10 — Complete the in-game UI system

**High priority**

A game engine without a reliable UI system becomes difficult to use for anything beyond simple arcade games.

**Add — Layout:**

- Anchors.
- Responsive scaling.
- Horizontal and vertical containers.
- Flex-style layouts.
- Padding and margins.
- Layout constraints.
- Screen resolution handling.

**Widgets:**

- Buttons.
- Labels.
- Images.
- Panels.
- Lists.
- Scroll views.
- Sliders.
- Checkboxes.
- Dropdowns.
- Text input.
- Progress bars.
- Tooltips.

**Advanced functionality:**

- Keyboard navigation.
- Gamepad focus navigation.
- Input capture.
- UI animation.
- Drag and drop.
- Localization.
- Font importing.
- SDF/MSDF text.
- Multiple UI canvases.

Also fix the existing list renderer so that text is actually included in the rendered output, and ensure buttons respond to hover and pressed states.

**Completion criteria:** Build a main menu, settings screen, inventory, pause menu and dialogue interface using only Kiln's UI editor.

### Stage 11 — Proper audio engine

**High priority**

The engine needs a complete audio pipeline, not just sound playback commands.

**Add:**

- WAV, OGG and MP3 importing.
- Audio asset preview.
- Sound effects.
- Music playback.
- Multiple audio channels.
- Master volume.
- Music and SFX buses.
- Volume automation.
- Looping.
- Pause and resume.
- Audio streaming for large music files.
- Playback position.
- Fade-in and fade-out.
- Audio events from animation.
- 2D positional audio.
- Distance attenuation.
- Basic audio effects.

Create an audio editor panel with a sound preview and playback settings.

For older hardware, the system should provide sensible limits on concurrent voices and audio memory.

**Completion criteria:** Create a game with background music, multiple simultaneous sound effects, volume controls and spatial audio. Verify the same assets work in native and web builds.

### Stage 12 — 2D lighting and advanced graphics

**Important differentiator for Kiln**

This is particularly important for the original goal of producing modern-looking games without demanding expensive hardware.

**Implement — 2D lighting:**

- Point lights.
- Freeform lights.
- Directional-style lighting where appropriate.
- Normal maps.
- Light colour and intensity.
- Light layers.
- Occlusion and shadow casters.
- Soft shadows.

**Rendering:**

- Blend modes.
- Color grading.
- Bloom.
- Distortion.
- Outline effects.
- Normal-map materials.
- Custom shaders.
- Pixel-art-friendly rendering.

**Performance:**

- Light culling.
- Light limits per object or region.
- Low-tier lighting fallback.
- Baked lighting options.
- Resolution scaling for expensive effects.

Prioritise normal mapping and 2D shadows. They give developers the ability to make relatively simple sprites look much more dimensional.

**Completion criteria:** Create a scene with multiple moving lights, normal-mapped sprites, occlusion and shadows. Demonstrate that Low and Medium rendering tiers remain usable.

---

### Stage 13 — Complete particles and VFX

The CPU and GPU particle foundations should become a proper editor-driven system.

**Add:**

- Particle emission.
- Emission shapes.
- Velocity curves.
- Size curves.
- Colour gradients.
- Lifetime controls.
- Rotation.
- Gravity.
- Burst emission.
- Sub-emitters.
- Collision.
- Texture selection.
- Sprite animation.
- Particle trails.
- GPU simulation.
- CPU fallback.

**Create a VFX editor with:**

- Live preview.
- Pause and restart.
- Parameter controls.
- Curve editor.
- Colour gradient editor.
- Particle count display.
- Performance information.

**Completion criteria:** Make fire, smoke, explosions, rain and magic effects in the editor without writing custom Rust code.

### Stage 14 — Prefabs, scenes and persistent game data

**Core engine workflow**

Developers need to reuse objects and reliably preserve game progress.

**Add — Prefab system:**

- Create prefab from entity.
- Instantiate prefab.
- Edit prefab.
- Nested prefabs.
- Per-instance overrides.
- Revert overrides.
- Prefab variants.

**Scene management:**

- Load scene.
- Unload scene.
- Switch scene.
- Additive scene loading.
- Scene transitions.
- Persistent entities.
- Scene lifecycle events.

**Saving:**

- Multiple save slots.
- Autosave.
- Save versioning.
- Save migrations.
- Persistent player settings.
- Graphics and audio settings.
- Key bindings.
- Save previews and timestamps.

The existing `kiln-save` crate and nested-scene structure are useful foundations. The important part is connecting them to the general-purpose runtime rather than saving only the vertical-slice state.

**Completion criteria:** Create two scenes with reusable prefabs, switch between them, save player progress, close the game and restore everything correctly.

### Stage 15 — Gameplay tools, debugging and performance

This stage combines two areas that make a small engine much more useful to developers.

**A. Built-in gameplay tools**

The `kiln-ai` and `kiln-gameplay` crates already contain useful beginnings. Complete their integration:

- Grid A* navigation.
- 2D navigation meshes.
- Pathfinding visualisation.
- Behavior trees.
- State machines.
- Dialogue system.
- Quest system.
- Inventory framework.
- Data tables.
- Event-driven interactions.

Make these optional built-in modules, not mandatory engine dependencies. For example, a developer making an arcade shooter shouldn't have to compile the dialogue and quest systems into their game.

**B. Professional profiler**

**Add:**

- CPU profiler.
- GPU profiler.
- Per-system execution time.
- Per-render-pass timing.
- Memory profiler.
- Texture memory monitoring.
- Audio memory monitoring.
- Entity/component statistics.
- Draw-call statistics.
- Script execution time.
- Allocation tracking.
- Performance warnings.

Also add an integrated runtime debugger with breakpoints, watches, stack traces and source mapping.

The GPU profiling foundation from M28 already exists. The next step is making it useful for everyday game development.

**Completion criteria:** A developer can identify the most expensive system, script, render pass or asset in a running game without using an external profiler.

### Stage 16 — Complete export, distribution and the 2D release gate

**Release blocker**

This is where Kiln becomes an actual engine that other people can use.

**Export pipeline**

Build a complete project-specific export system.

| Platform | Required functionality |
|----------|------------------------|
| Windows | Standalone game and installer/portable package |
| Linux | Standalone game and distributable package |
| macOS | Proper `.app` and distribution package |
| Web | Complete browser build |
| Android | APK and signed AAB |
| iOS | Proper Xcode project, build and signing |

The pipeline must automatically:

1. Read the project manifest.
2. Compile the game runtime.
3. Compile the game's scripts.
4. Import and package required assets.
5. Apply platform-specific settings.
6. Exclude unused optional features.
7. Generate the executable or web application.
8. Generate the necessary configuration files.
9. Report build errors clearly.

**Also support:**

- Development builds.
- Release builds.
- Debug symbols.
- Asset compression.
- Build profiles.
- Icons and splash screens.
- Version numbers.
- Application name and publisher.
- Crash logs.

**Three mandatory example games**

Before calling Kiln 2D complete, require three actual games:

1. **Platformer** — demonstrates tilemaps, physics, character controllers, animation, camera, sound, collectibles, menus and checkpoints.
2. **Top-down adventure** — demonstrates scenes, NPCs, dialogue, inventory, navigation, quests, save/load and gamepad controls.
3. **Arcade shooter** — demonstrates high sprite counts, particles, effects, projectiles, audio, input and performance.

All three must be created entirely through Kiln's normal developer workflow, not special Rust code written just for the samples. They must run from the editor and from their exported builds.

---

## 3. What you already have versus what you need to finish

This is important so development time is not spent rewriting things that already exist.

| Existing Kiln system | Recommended action |
|----------------------|--------------------|
| Sprite renderer | Improve, don't rewrite |
| Texture loading | Extend importer and atlas pipeline |
| Sprite editor | Extend existing editor |
| Chunked tilemaps | Complete editing tools |
| Scene view | Extend scene workflows |
| Inspector | Add more component types |
| Undo/redo | Generalise to editor commands |
| KilnScript VM | Expand language and runtime API |
| Graph compiler | Connect to actual gameplay runtime |
| Animation | Complete state machines and timeline |
| Rapier physics | Repair and complete integration |
| UI widgets | Finish layout and interaction |
| Input maps | Finish controller and mobile integration |
| CPU/GPU particles | Connect to VFX editor |
| Audio | Complete actual asset loading and playback |
| AI libraries | Integrate as optional modules |
| Save system | Connect to general game state |
| Render graph | Extend profiling and effects |

**A few existing implementation issues deserve particular attention:**

- Animation currently uses a hardcoded timestep.
- Physics collision event collection and raycasting are incomplete.
- Native audio registration does not actually load the provided sound file.
- The list renderer has a text-rendering defect.
- The web archive loader is a stub.
- The native and web export pipelines need project and asset packaging.

The code should be the authority for marking features complete, not the checkbox in `TODO_LIST.md`.

## 4. Features I would deliberately postpone

Scope must be controlled. Otherwise Kiln could spend years trying to reproduce every feature of Unity.

These are not prerequisites for 2D v1.0:

| Feature | Decision |
|---------|----------|
| Multiplayer networking | Postpone |
| Console SDK integration | Postpone |
| Advanced modding | Postpone |
| Plugin marketplace | Postpone |
| Full Photoshop-like editor | Use external asset tools |
| Advanced 2D skeletal IK | Advanced feature |
| Complex procedural world generation | Optional |
| Huge RPG framework | Optional modules |
| Advanced 2D navigation | Expand later |

The distinction: developers should be able to implement these things through Kiln's APIs, even when Kiln doesn't provide a specialised editor for every one.

---

## 5. The final 2D release gate

Before writing serious 3D code, require the following:

**Kiln 2D v1.0 checklist — 0/17 verified**

Check an item only after its acceptance tests have passed. This is a planning checklist, not a live repository audit.

- [ ] **Build and CI** — The complete Rust workspace compiles, formats, passes Clippy and tests.
- [ ] **Generic runtime** — The engine can run games that have no connection to the original vertical slice.
- [ ] **Project workflow** — Create, import, edit, save, reopen and organise projects without data loss.
- [ ] **Sprites and rendering** — Sprite sheets, atlases, cameras, sorting and materials work.
- [ ] **Level editing** — Tilemaps, automatic tiles, multiple layers and collision editing work.
- [ ] **Scripting** — Loops, functions, collections, debugging and runtime entity APIs work.
- [ ] **Animation** — Frame animation, state machines and timeline sequencing work.
- [ ] **Physics** — Collisions, triggers, raycasts, joints and character controllers work.
- [ ] **Input** — Keyboard, mouse, gamepads and mobile touch are supported.
- [ ] **UI** — Responsive menus, buttons, lists, text and gamepad navigation work.
- [ ] **Audio** — Real audio assets load, stream and play correctly on target platforms.
- [ ] **Graphics and VFX** — 2D lighting, shadows, particles and rendering tiers work.
- [ ] **Persistence** — Scenes, prefabs, save slots, settings and migrations work.
- [ ] **Profiling** — CPU, GPU, memory and asset budgets are measurable.
- [ ] **Export** — Project-specific native and web builds run independently of Rust development tools.
- [ ] **Samples** — Three complete sample games run in editor and exported builds.
- [ ] **Hardware testing** — Low-tier hardware and required mobile devices are physically tested.

**Performance acceptance:** quantify it. Keep the existing 16.6 ms Low-tier frame budget, select an actual reference integrated GPU, and benchmark a repeatable scene containing sprites, tilemaps, physics and particles. Record CPU and GPU time separately.

For desktop and mobile ambitions, establish desktop and web reliability first, then require real Android and iOS device tests before claiming mobile production readiness.

## 6. When should you finally start 3D?

Once the 2D gate is completed, the existing M29 roadmap becomes the right next step.

**Kiln 3D development entry gate:**

1. glTF 2.0 model importing.
2. GPU mesh rendering and vertex/index buffers.
3. Depth buffer and perspective camera.
4. Frustum culling.
5. 3D editor viewport and transform gizmos.
6. Verify that disabling `3d` excludes optional 3D code/dependencies. Package size is informational (ADR-0062).
7. Run a basic 3D sample on Low and Medium hardware.

M29 acceptance required.

**One architectural rule to preserve throughout all 17 stages:** keep the 2D runtime, renderer and optional systems modular. Don't let the 3D work force unnecessary dependencies into 2D-only games.

## 7. My suggested immediate development order

Do not implement this entire roadmap in one operation. Follow this order, one milestone at a time:

| Order | Work | Importance |
|-------|------|------------|
| 1 | Stabilise current build and CI | Critical |
| 2 | Generalise runtime and scripting architecture | Critical |
| 3 | Complete project and asset management | Critical |
| 4 | Complete scene editor | Critical |
| 5 | Sprite and rendering improvements | High |
| 6 | Tilemap editor improvements | High |
| 7 | KilnScript maturity and graph integration | Critical |
| 8 | Animation and sequencing | High |
| 9 | Physics completion | Critical |
| 10 | Input completion | High |
| 11 | UI completion | High |
| 12 | Audio completion | High |
| 13 | 2D lighting | High |
| 14 | Particles and VFX | High |
| 15 | Prefabs, persistence and gameplay tools | High |
| 16 | Profiler, export and release validation | Critical |

**Important distinction:** Stages 0–5 should be completed before expanding the feature set. They establish the architecture and workflow upon which the other systems depend.