# Gameplay camera follow and dead zones

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
Follow history resets with the controller; this checkpoint does not add persisted
camera motion history, shake, transitions, parallax or multiple views.

Invalid settings, targets, overflow or unrepresentable projections return an error
without changing the controller's stored position/initialization. Fixing the data
allows the same controller to update again. Screen/world conversions use finite
intermediates and return None for unrepresentable outputs. Their existing outside-
viewport extrapolation remains available for tools; callers must still choose and
clip the correct viewport for gameplay input. Offset/scale and Y orientation are
covered by round-trip tests; split-view routing remains separate work.

Runtime tools now require kiln-authoring-v8. Rebuild matching editor/CLI/player and
browser bundles; older runtimes lack dead zones or shake calls and are rejected by the new
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
