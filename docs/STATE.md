# Kiln Session Handoff

**Active focus:** Task 5 implementation complete; native graphical acceptance pending.
**Branch:** `feat/2d-production-workflow`. 3D remains paused. Task 6 has not started.

## Delivered

- Saved `Component::Physics` and `Component::Joint` data and per-scene project camera settings. Optional simulation builds retain their serialized authoring data.
- Rapier/ECS fixed 60 Hz synchronization tracks generational handles, body/collider/component removal, collider/material/scale rebuilds, teleports, velocities, hierarchy/local/world-offset conversion, and non-physical intermediate parents. Kinematic platforms integrate velocity; physics writes simulated poses/velocities back to the ECS.
- Actual solid contact and sensor enter/exit transitions, mutual membership/mask bitsets, normalized geometry rays with stable IDs/distance/normals, circle shape overlaps, and distance/hinge/slider joints with scaled anchors, supported limits/motors, removal and rebuild reconnection. Attached script callbacks share the parent's live query world without stepping it again.
- Inspector body/collider/joint authoring uses validated scene commands, undo/redo, projection and disk persistence. Scene View draws transformed collider outlines during editing and Play. Joint duplication redirects the owner's endpoint. The existing standalone Physics panel is explicitly a sandbox preview.
- Editor/player share gameplay camera follow, exponential smoothing, zoom, bounds, pixel snapping and aspect-fit physical viewports. Pixel-perfect upscales use integer viewport scaling; smaller panels downscale. Both hosts map pointer positions through that viewport and expose mouse world coordinates to attached scripts. Camera project settings participate in dirty/save tracking, separate from scene undo history.
- `samples/physics2d_editor` is an editable project with a jumping hero, slope, moving platform, trigger, excluded collision layer and bounded follow camera. `docs/PHYSICS_2D.md` describes authoring, contracts and native acceptance. KILNSCRIPT.md documents the updated stable-ID ray result layout and velocity/input APIs.

## Validation

- `just check`: passed (formatting, strict workspace all-target Clippy, workspace and standalone audio sample checks).
- `just test`: **577 passed, zero failed/skipped**. Includes 11 physics workflow tests and new camera/editor acceptance coverage.
- `cargo build -p kiln-editor -p kiln-player`: passed.
- `cargo clippy -p kiln-runtime --no-default-features --all-targets -- -D warnings`: passed.
- `cargo build --target wasm32-unknown-unknown -p kiln-web`: passed; this verifies compilation, not browser physics gameplay.
- The checked-in sample executes source resources at 30 render FPS and deserialized player project/byte assets at 144 FPS, with identical fixed-tick poses, trigger state and camera limits. Jumping, filtering and input propagation are asserted; authoring documents remain unchanged.
- Tests cover grounding, slope normals, moving-platform carrying, shape-vs-AABB queries, ray filtering, single contact transitions, despawn/handle reuse, all three constraints, rebuilds, scaled anchors, intermediate-parent conversion, smoothing/pixel-grid/bounds/viewport mapping, undo/redo and save/reopen.
- Built editor startup was attempted. Window creation failed because neither WAYLAND_DISPLAY, WAYLAND_SOCKET nor DISPLAY is set. Native visual acceptance is **unrun**, with a concrete procedure in PHYSICS_2D.md. Headless tests are not visual evidence.

## Supported limits / next

- Task 5 implementation checkboxes are checked; its native graphical acceptance remains pending. Task 6 requires the user's next instruction.
- Native editor/player enable physics. Browser/mobile gameplay host integration and device acceptance require separate work/evidence; optional runtime builds compile without simulation.
- Collider shapes are circles and rectangles. Circle nonuniform scaling uses the largest absolute axis. Joint endpoints must identify existing physics bodies to activate. Bounds override pixel snapping at non-grid-aligned edges; small viewports downscale to fit.
- Physics ray results now use `[source_id, distance, normal_x, normal_y]`; migrate provisional scripts that expected internal arena indices. World physics queries and local authored position getters use different coordinate spaces for parented entities.
- Prior Task 4 entity scripting/Play implementation remains in place; live debugging is frame-granular and arbitrary game-save script-state serialization is later work.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Git pushes use the supported `require_escalated` network permission. The prior remote tip was `7115d8f`; this session's commit records Task 5.
