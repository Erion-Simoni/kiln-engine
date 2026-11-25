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

Runtime tools now require kiln-authoring-v7. Rebuild matching editor/CLI/player and
browser bundles; a v6 runtime could ignore dead_zone and is rejected by the new
exporter. VM-memory/save-slot layout is unchanged. See samples/camera_dead_zone and
[evidence](evidence/A05_CAMERA_DEAD_ZONES.md).
