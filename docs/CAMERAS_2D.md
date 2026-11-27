# Gameplay cameras

Open View → Scene Editor → Gameplay Camera. Choose a follow entity and edit
Dead zone (world width / height). Settings are stored per scene in the project:

```toml
[cameras.main]
target = "hero"
position = [0.0, 0.0]
zoom = 1.0
smoothing = 4.0
dead_zone = [64.0, 32.0]
bounds = [0.0, 0.0, 960.0, 480.0]
```

The zone uses full width/height in world units. Zero preserves centered follow;
old projects without the field load with zero. Initial camera creation/reset
centers on the target plus follow offset. Later updates hold the origin while
that point remains inside the centered box, then move the goal only far enough
to put it back on the edge. Each effective dimension is capped to the visible
world extent after zoom. A stationary camera ignores dead_zone.

Follow response applies exponential smoothing to that goal; zero snaps. World
bounds apply before and after pixel snapping and take priority if a bound lies
off the pixel grid. A view larger than its level centers on the bounded level.
Follow history resets with the controller. Camera motion history is transient;
parallax and multiple views remain separate work.

Invalid settings, targets, overflow or unrepresentable projections return an error
without changing the controller's stored position/initialization. Fixing the data
allows the same controller to update again. Screen/world conversions use finite
intermediates and return None for unrepresentable outputs. Their existing outside-
viewport extrapolation remains available for tools; callers must still choose and
clip the correct viewport for gameplay input. Offset/scale and Y orientation are
covered by round-trip tests; split-view routing remains separate work.

Runtime tools now require kiln-authoring-v12. Rebuild matching editor/CLI/player and
browser bundles; older runtimes lack current camera effects, parallax or pointer validity and are rejected by the new
exporter. VM-memory/save-slot layout is unchanged. See samples/camera_dead_zone and
[evidence](evidence/A05_CAMERA_DEAD_ZONES.md).

## Scripted shake

`camera_shake(x, y, duration, frequency, seed)` queues a deterministic world-space
shake. Amplitudes are 0..4096, duration is (0,60] seconds, frequency is (0,120] Hz,
and seed is an integer 0..4294967295. `camera_shake_stop()` clears active shakes.
Both return true if queued, false when the shared 32-command queue is full.
Invalid arguments fail the callback. Failed or suspended callbacks do not deliver
uncommitted effects. All scene and attached scripts share the queue.

Up to 32 active shakes mix with quadratic decay; newer overflow replaces the
oldest. Offsets never feed back into follow/dead-zone history. Pixel snapping and
final bounds apply to the shaken view, so bounds can suppress motion. UI pause
holds effect time. Controller resets clear transient effects; logical saves do
not preserve shake history. Matching seeds and elapsed time reproduce the same
oscillator, without promising identical native/browser floating-point pixels.
See `samples/camera_shake` and [evidence](evidence/A05_CAMERA_SHAKE.md).

## Scripted transitions

`camera_transition(x, y, zoom, seconds)` moves to a world-space camera **origin**
and zoom using smoothstep interpolation. Coordinates must be finite; zoom must be
1/64..64 and duration 0..60 seconds. Zero seconds cuts immediately. Nonzero requests
start at time zero on delivery and hold the exact endpoint after completion.
`camera_transition_stop()` returns immediately to the current authored follow view.
Both calls return false if the shared 32-request effect queue is full. Invalid
arguments fail the callback; failed/suspended callbacks do not deliver requests.

New transitions replace the active one, starting from the last successfully
sampled view after bounds/pixel snapping, excluding shake. Authored follow keeps
updating underneath. UI pause holds transition time; invalid projections commit
neither transition nor shake time. Transitioned zoom determines final world bounds.
Pixel-perfect mode rounds zoom to an integer, so use it only when stepped zoom is
wanted. Shake offsets layer over transitions. Stop-transition leaves shake active;
stop-shake leaves the transition active. Scene/controller reset clears both.
Logical saves do not preserve transition history. The shared editor/player/browser
adapter implements these semantics without extra render passes or dependencies.

See `samples/camera_transition` and [transition evidence](evidence/A05_CAMERA_TRANSITIONS.md).

## Sprite parallax

Open **Inspector → Sprite Pivot / Pixel Scale / Parallax**, or **View → Sprite
Editor**, to edit per-axis scroll factors and camera-origin anchor. The default
factor `[1,1]` keeps ordinary world scrolling. `[0,0]` keeps the sprite at a fixed
camera-relative origin; fractional factors scroll slowly, factors above one scroll
faster, and negative factors reverse motion. Factors must be finite in -8..8;
anchors must be finite. For each axis the renderer adds
`(camera_origin - anchor) * (1 - factor)` to the transformed sprite quad, then culls
and sorts it. The anchor is the camera origin where the offset is zero.

```toml
{ type = "SpriteLayout", settings = { parallax = [0.5, 0.25], parallax_origin = [20.0, 8.0] } }
```

Factors apply to world X/Y independently, after hierarchy/pivot/rotation/scale.
They do not alter transforms, collisions or descendants. Zoom still scales these
sprites, and camera shake/transitions affect their camera-relative offset. Use UI
widgets for HUDs. Authoring view retains logical world coordinates for selection
and gizmos; Play shows the actual parallax. Edits use existing component undo/redo
and persist through project restart and logical gameplay saves. Old layouts/saves
without these fields default to ordinary scrolling. Invalid authored layouts
reject export before publication. Tile/particle parallax and repeated backgrounds
remain separate work. See `samples/parallax` and
[parallax evidence](evidence/A05_SPRITE_PARALLAX.md).

## Gameplay pointer validity and prepared views

Gameplay now uses `Camera::pointer_to_world`: viewport edges are half-open and
letterbox bars are excluded. Missing/non-finite pointers clear stale world
coordinates. Use `mouse_world_valid()` before world interactions; coordinate
getters return zero when invalid. Native cursor leave/focus loss and browser
pointer absence/touch completion clear presence. Editor Play clears absent input.
UI event routing stays separate. Tools can use `screen_to_world` to extrapolate.

`kiln_render::views::route_pointer` accepts up to four fitted view rectangles,
rejects invalid rectangles and maps through the last (frontmost) containing camera.
Shared edges belong to one adjacent view, gaps to none. This helper and the
renderer API below are prerequisites; authored split-screen configuration and
multi-camera host scheduling are still open.

Multiple world views before one GPU submission must use
`SpriteRenderer::prepare_view(device,camera)` and `draw_prepared(pass,view)`.
Prepared views retain independent camera/instance buffers while sharing textures
and materials. Set viewport/scissor before each draw. Empty queues allocate
nothing, sprite count remains capped at 16384, and each view exposes its own
queued/draw-call counts. Prepared uploads survive clearing the CPU queue and can
be retained while renderer resources stay unchanged. Replaced/foreign resources
reject drawing. Clear project textures also discards pending invalid texture IDs.
The existing single-view `flush` remains supported and reuses mutable buffers;
default hosts keep that path. No extra allocation is added to their frame loop.

See [prepared-view and pointer evidence](evidence/A05_VIEW_UPLOADS_AND_POINTERS.md)
and `samples/world_pointer`.
