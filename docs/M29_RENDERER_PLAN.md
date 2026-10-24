# 3c — M29 Renderer Plan (Pre-M29)

**Goal:** Scope the depth buffer, perspective camera, and frustum culling
behind the `3d` feature in `kiln-render`. Design doc only — implementation
happens in M29.

**Why:** These are three of M29's four acceptance items. Scoping them now
prevents scope creep and ensures the `3d`-off 2D build is not affected.

---

## Current render state (verified from source)

| Component | Current capability |
|-----------|-------------------|
| `kiln-render::Camera` | Orthographic only (`Camera::new`, top-left origin option) |
| `kiln-render::Renderer` | wgpu device/swapchain, surface config, tier, post pipeline |
| `kiln-render::SpriteRenderer` | 2D instanced batching, ortho projection via camera |
| `kiln-render::Frame2dGraph` / `graph.rs` | Declared passes, resource lifetimes, transient aliasing |
| `kiln-render::post` | Bloom, fog, outline, LUT (2D post-processing) |
| Depth buffer | None — no depth texture, no depth-stencil state |
| Perspective camera | None — orthographic only |
| Frustum culling | None — tilemap has chunk culling, but no general frustum cull |

---

## Design

### Depth buffer

Add a `DepthTexture` to `Renderer` (or `PostFramebuffer`) when `3d` is on:

```rust
// In kiln-render, behind cfg(feature = "3d") or always-present but only used with 3d
pub struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}
```

- Format: `Depth24Plus` (already in `TextureFormatKey`).
- Created with `RENDER_ATTACHMENT | TEXTURE_BINDING` usage.
- Bound as `depth_stencil_attachment` in render passes that draw 3D.
- Cleared to 1.0 each frame.

### Perspective camera

Extend `kiln-render::Camera` with a perspective projection mode:

```rust
pub struct Camera {
    pub position: [f32; 2],   // or Vec3 for 3D
    pub zoom: f32,
    pub viewport_size: [f32; 2],
    pub top_left_origin: bool,
    // NEW (3d):
    pub projection: Projection,  // Orthographic | Perspective { fov, aspect, near, far }
}

pub enum Projection {
    Orthographic,
    Perspective { fov_y: f32, near: f32, far: f32 },
}
```

- `projection_matrix()` returns orthographic (current) or perspective matrix.
- `view_bounds()` for perspective returns the frustum corners in world space.

### Frustum culling

Add a `frustum_cull` helper that takes a perspective camera's view-projection
matrix and an axis-aligned bounding box, returning visible/invisible:

```rust
pub struct Frustum {
    planes: [[f32; 4]; 6],  // left, right, bottom, top, near, far
}
impl Frustum {
    pub fn from_view_proj(vp: &glam::Mat4) -> Self;
    pub fn contains_aabb(&self, min: [f32; 3], max: [f32; 3]) -> bool;
}
```

- Used by the renderer to skip drawing meshes outside the camera frustum.
- Tilemap already does chunk culling; frustum culling is for 3D meshes.

### Feature gating

All 3D renderer additions live behind `#[cfg(feature = "3d")]` or are
guarded by `renderer.tier()` / a `has_3d` flag, so the `3d`-off build is
unchanged.

---

## Acceptance criteria (testable, M29)

| Item | Test |
|------|------|
| Depth buffer | `cargo test -p kiln-render --features 3d` — depth texture clears, depth writes work |
| Perspective camera | `Camera` with `Projection::Perspective` produces a non-orthographic projection matrix |
| Frustum culling | `Frustum::contains_aabb` correctly culls boxes outside the view |

---

## See Also

- `kiln-render/src/camera.rs` — `Camera` to extend
- `kiln-render/src/renderer.rs` — `Renderer` to add depth texture
- `kiln-render/src/graph.rs` — `TextureFormatKey::Depth24Plus` already exists
- `docs/BUDGETS.md` — `3d`-off size check baseline
