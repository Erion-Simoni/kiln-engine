# 3d — M29 Editor 3D View Plan (Pre-M29)

**Goal:** Scope the 3D editor scene view with gizmos, building on ADR-0032
(the editor draws the scene with the engine). Design doc only — implementation
happens in M29.

**Why:** "3D scene view in the editor with gizmos" is one of M29's four
acceptance items. The viewport-switching (2D ortho ↔ 3D perspective) and
gizmo drawing approach need to be decided before implementation.

---

## Current editor scene view state (verified from source)

| Component | Current capability |
|-----------|-------------------|
| `kiln-editor/src/scene_view.rs` | `letterbox()`, `world_camera()`, `overlay_camera()`, grid lines, scene textures |
| `kiln-editor/src/app.rs` | `EditorApp` with `GpuState`, `SpriteRenderer`, scene view panel, play session |
| ADR-0032 | Editor draws scene with engine's sprite renderer; viewport letterboxed; world camera matches player |
| Gizmos | None — no gizmo drawing code exists |

---

## Design

### 3D viewport switching

The editor scene view panel currently uses an orthographic camera
(`Camera::new(world_w, world_h)`). For 3D, add a perspective camera option:

```rust
// In kiln-editor::scene_view, behind feature gate or runtime flag
pub enum ViewMode {
    Ortho2D,     // current: world Y up, orthographic
    Perspective3D, // NEW: perspective camera with yaw/pitch orbit
}

pub struct SceneViewState {
    pub view_mode: ViewMode,
    pub orbit_yaw: f32,   // for 3D orbit camera
    pub orbit_pitch: f32,
    pub orbit_distance: f32,
    pub show_gizmos: bool,
}
```

- The panel toggles between ortho and perspective via a menu/toolbar button.
- In Perspective3D mode, the camera orbits the world origin (yaw/pitch/distance).
- The world grid adapts: ortho shows a 2D grid; perspective shows a floor grid + axis indicators.

### Gizmos

Gizmos are drawn as screen-aligned or world-aligned geometry using the
sprite renderer (consistent with ADR-0032 — engine draws the scene):

```rust
// Gizmo types
pub enum Gizmo {
    Translate { axis: Axis, color: [f32; 4] },  // colored arrow
    Rotate { axis: Axis, color: [f32; 4] },     // colored ring
    Scale { axis: Axis, color: [f32; 4] },      // colored cube/line
}

pub enum Axis { X, Y, Z }
```

- **Translation gizmo:** three colored arrows (red X, green Y, blue Z).
- **Rotation gizmo:** three colored rings around each axis.
- **Scale gizmo:** three colored cubes/handles.
- Drawn with `SpriteRenderer::push_textured` or colored quads, depth-tested
  against the scene so they occlude correctly.
- Click/drag on a gizmo axis transforms the selected entity (translate,
  rotate, or scale).

### Rendering path

In `kiln-editor/src/app.rs`, the scene view render pass:
1. Clear with background color.
2. Draw the scene (sprites/tilemaps) with the active camera (ortho or perspective).
3. If `show_gizmos` and an entity is selected, draw the gizmo for the active transform mode.
4. Draw the grid overlay.

The same `SpriteRenderer` instance is used (batched with scene sprites where possible).

### Feature gating

3D view mode and gizmos are only available when the `3d` feature is enabled
in `kiln-core` (mesh types, perspective math). The `3d`-off editor build
shows only the ortho 2D view.

---

## Acceptance criteria (testable, M29)

| Item | Test |
|------|------|
| 3D editor scene view | Editor switches to perspective camera; scene renders with perspective projection |
| Gizmos | Translation/rotation/scale gizmos draw for selected entity in 3D mode |
| 2D↔3D toggle | Toggling view mode does not crash; ortho view unchanged when `3d` off |

---

## See Also

- `kiln-editor/src/scene_view.rs` — `letterbox`, `world_camera`, `overlay_camera` to extend
- `kiln-editor/src/app.rs` — `EditorApp`, scene view panel
- `docs/DECISIONS.md` — ADR-0032 (editor draws scene with engine)
- `kiln-core/src/mesh.rs` — 3D types behind `3d` feature (ADR-0034)
