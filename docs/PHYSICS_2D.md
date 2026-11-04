# Physics and gameplay cameras

Open `samples/physics2d_editor/kiln.toml` through **File → Open Project** in the
native editor. Play the current scene. Left/right moves the hero; up jumps when
grounded. The sample has a slope, moving platform, amber trigger, and purple
body whose collision layer excludes the hero. The camera follows within world
bounds. This project uses colored sprites and needs no imported assets.

## Authoring

Select an entity, then use **+ Physics Body / Collider** in the Inspector. Choose
Static for level geometry, Dynamic for simulated bodies, or Kinematic for moving
platforms. Set rectangle half-extents or circle radius, friction, restitution,
density, gravity scale, and initial velocities. Mark **Sensor / trigger** to
receive events without a collision response. Edits are validated, undoable,
and saved with the scene. Circle scaling uses the largest absolute axis;
rectangle dimensions scale per axis. Rotation and hierarchy use world geometry.

Membership and mask are 16-bit bitsets. A pair interacts only when each body's
membership intersects the other's mask. For example, hero membership 2/mask 5
interacts with floor membership 1/mask 2 and sensor membership 4/mask 2.
**Colliders** in the Scene View toolbar draws real scene collider outlines in
edit and Play, green for solids and yellow for sensors. The Physics panel's
separate sandbox preview does not edit these saved components.

Use **+ Joint to Entity** to add a Distance, Revolute (hinge), or Prismatic
(slider) constraint. Both stable endpoint IDs must have physics bodies for the
constraint to activate. Anchors use each entity's local coordinates and scale;
distance uses world units. Sliders support axis angle, translation limits, and
velocity motors; hinges support velocity motors. Maximum motor force/torque
must be positive for a motor to drive motion. Removing an endpoint removes
its live constraint; recreating a collider reconnects the constraint. Duplicating
the joint owner redirects its own endpoint to the copy.

In **Scene Editor → Gameplay Camera**, choose a follow entity or Stationary,
origin/follow offset, zoom, follow response, and optional world bounds. Follow
response zero snaps; positive values use exponential smoothing independent of
render rate. A missing target retains the last camera position. When the view
is larger than the bounds, the level is centered. Settings are saved per scene
and included in project dirty tracking. They are project settings, separate
from the scene's undo history.

Pixel-perfect mode rounds zoom to an integer, snaps origin to the configured
pixels-per-world-unit grid, and uses integer physical viewport upscales.
Bounds take precedence at edges that do not align with the pixel grid. Panels
smaller than the reference resolution downscale to keep the whole view visible.
Editor and player preserve the project's reference aspect on resize and use
that same physical viewport for pointer mapping. `get_mouse_world_x/y` exposes
the mapped point to scripts. Outside points extrapolate beyond the viewport;
screen-space UI retains its own coordinate system.

## Simulation contract

Editor/player schedule one physics step per fixed 1/60-second gameplay tick.
Rapier owns simulated world poses; the bridge converts them back into local
transforms, including non-physical intermediate parents and nested offsets.
Authored teleports, body velocity edits, collider/material/scale edits, component
removal, despawn, and generational ECS handle reuse are synchronized. Enter/exit
events report transitions once per stable source-ID pair. Kinematic velocity
advances platforms without applying gravity. Script callbacks share the same
query world and do not advance simulation a second time.

Physics is optional in `kiln-runtime`; native editor/player enable it. This
checkpoint covers native hosts. Browser/mobile host integration and device
validation require their own evidence. See `KILNSCRIPT.md` for query return
values and the velocity API.

## Acceptance evidence and remaining native check

`kiln-runtime/tests/physics_workflow.rs` executes the checked-in sample from
source resources and deserialized player project/byte assets, with identical
results at 30 and 144 render FPS. It verifies jumping, grounding, trigger
callbacks, excluded collision layers, input mapping propagation, and camera
limits. Additional cases exercise slope normals, moving-platform carrying,
shape-vs-AABB accuracy, masks/sensors/self exclusion, removal transitions,
distance/hinge/slider constraints, rebuilds, scaled anchors, and hierarchy
conversion. Editor tests verify undo/redo, invalid-edit rejection, runtime
projection and disk persistence. Camera tests verify smoothing, pixel snapping,
small bounds, missing targets, physical integer scaling, and mapping against the
render projection in both Y directions.

Run `just check`, `just test`, and `cargo build -p kiln-editor -p kiln-player`.
The no-physics configuration is checked with
`cargo clippy -p kiln-runtime --no-default-features --all-targets -- -D warnings`.

Native graphical acceptance remains **unrun** on this cloud runner: starting
the built editor fails before window creation because no DISPLAY/Wayland server
is available. On a graphical machine:

1. Open the sample, toggle Colliders, and verify outlines align with transformed
   solids/sensors. Edit a collider and joint; undo/redo, save, restart, and reopen.
2. Play, move/jump onto the slope/platform, pass the amber trigger and purple
   filtered body, and inspect actual collision state in the Gameplay Debugger.
3. Change camera follow/zoom/bounds/pixel settings outside Play. Resize the Scene
   View and exported native player; verify matching framing, crisp integer
   upscales, bounds, and pointer world coordinates in the hero's debugger state.
4. Record OS/GPU, editor/player build revision, results, and screenshots before
   marking Task 5's native visual acceptance complete.
