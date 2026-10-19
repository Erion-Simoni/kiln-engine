//! 9-slice rendering for UI elements.
//!
//! All output is screen-space, **y-down**, in the same units as the widget
//! bounds. Each [`SpriteInstance`] position is the quad centre. Pair with an
//! ortho camera whose top-left is the origin.

use crate::layout::Rect;
use kiln_render::SpriteInstance;
use serde::{Deserialize, Serialize};

pub use crate::text::TextRenderer;

/// 9-slice sprite configuration for scalable UI elements.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NineSlice {
    /// Sprite texture ID (a [`kiln_render::TextureId`] index).
    pub texture_id: u64,
    /// Left border width (in pixels).
    pub border_left: f32,
    /// Right border width (in pixels).
    pub border_right: f32,
    /// Top border height (in pixels).
    pub border_top: f32,
    /// Bottom border height (in pixels).
    pub border_bottom: f32,
    /// Source texture width in pixels (used to compute UVs).
    pub tex_width: f32,
    /// Source texture height in pixels (used to compute UVs).
    pub tex_height: f32,
}

impl NineSlice {
    /// Uniform border; source texture assumed `2*border + 1` pixels square
    /// until [`NineSlice::with_texture_size`] is called.
    pub fn new(texture_id: u64, border: f32) -> Self {
        Self {
            texture_id,
            border_left: border,
            border_right: border,
            border_top: border,
            border_bottom: border,
            tex_width: border * 2.0 + 1.0,
            tex_height: border * 2.0 + 1.0,
        }
    }

    /// Sets per-edge borders (left, right, top, bottom).
    pub fn with_borders(mut self, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        self.border_left = left;
        self.border_right = right;
        self.border_top = top;
        self.border_bottom = bottom;
        self
    }

    /// Sets the source texture size in pixels.
    pub fn with_texture_size(mut self, width: f32, height: f32) -> Self {
        self.tex_width = width;
        self.tex_height = height;
        self
    }
}

/// Renderer for 9-slice sprites.
///
/// Generates up to 9 sprite instances (corners, edges, centre) from a single
/// texture; border regions keep their pixel size when the target is resized.
#[derive(Debug, Default, Clone, Copy)]
pub struct NineSliceRenderer;

impl NineSliceRenderer {
    /// Creates a renderer.
    pub fn new() -> Self {
        Self
    }

    /// Builds the instances for `nine_slice` stretched over `bounds`, tinted
    /// by `color`. If the borders exceed the bounds they are scaled down.
    pub fn render(
        &self,
        nine_slice: &NineSlice,
        bounds: Rect,
        color: [f32; 4],
    ) -> Vec<SpriteInstance> {
        let tw = nine_slice.tex_width.max(1.0);
        let th = nine_slice.tex_height.max(1.0);
        let (l, r) = fit_pair(
            nine_slice.border_left,
            nine_slice.border_right,
            bounds.width,
        );
        let (t, b) = fit_pair(
            nine_slice.border_top,
            nine_slice.border_bottom,
            bounds.height,
        );

        let xs = [bounds.x, bounds.x + l, bounds.right() - r, bounds.right()];
        let ys = [bounds.y, bounds.y + t, bounds.bottom() - b, bounds.bottom()];
        let us = [
            0.0,
            nine_slice.border_left / tw,
            1.0 - nine_slice.border_right / tw,
            1.0,
        ];
        let vs = [
            0.0,
            nine_slice.border_top / th,
            1.0 - nine_slice.border_bottom / th,
            1.0,
        ];

        let mut out = Vec::with_capacity(9);
        for j in 0..3 {
            for i in 0..3 {
                let w = xs[i + 1] - xs[i];
                let h = ys[j + 1] - ys[j];
                if w <= 0.0 || h <= 0.0 {
                    continue;
                }
                out.push(SpriteInstance {
                    position: [xs[i] + w / 2.0, ys[j] + h / 2.0],
                    size: [w, h],
                    color,
                    uv_rect: [us[i], vs[j], us[i + 1], vs[j + 1]],
                });
            }
        }
        out
    }
}

/// Scales two borders down proportionally if they exceed `total`.
fn fit_pair(a: f32, b: f32, total: f32) -> (f32, f32) {
    let sum = a + b;
    if sum > total && sum > 0.0 {
        let s = (total / sum).max(0.0);
        (a * s, b * s)
    } else {
        (a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nine_slice_new() {
        let nine_slice = NineSlice::new(123, 8.0);
        assert_eq!(nine_slice.texture_id, 123);
        assert_eq!(nine_slice.border_left, 8.0);
        assert_eq!(nine_slice.border_right, 8.0);
        assert_eq!(nine_slice.border_top, 8.0);
        assert_eq!(nine_slice.border_bottom, 8.0);
    }

    #[test]
    fn test_nine_slice_with_borders() {
        let nine_slice = NineSlice::new(123, 8.0).with_borders(10.0, 12.0, 8.0, 14.0);
        assert_eq!(nine_slice.border_left, 10.0);
        assert_eq!(nine_slice.border_right, 12.0);
        assert_eq!(nine_slice.border_top, 8.0);
        assert_eq!(nine_slice.border_bottom, 14.0);
    }

    #[test]
    fn nine_slice_makes_nine_quads_covering_bounds() {
        let ns = NineSlice::new(1, 4.0).with_texture_size(16.0, 16.0);
        let quads =
            NineSliceRenderer::new().render(&ns, Rect::new(10.0, 20.0, 100.0, 50.0), [1.0; 4]);
        assert_eq!(quads.len(), 9);
        let area: f32 = quads.iter().map(|q| q.size[0] * q.size[1]).sum();
        assert!((area - 5000.0).abs() < 0.01);
        assert_eq!(quads[0].size, [4.0, 4.0]);
        assert_eq!(quads[0].position, [12.0, 22.0]);
        assert_eq!(quads[0].uv_rect, [0.0, 0.0, 0.25, 0.25]);
    }

    #[test]
    fn nine_slice_shrinks_borders_in_small_bounds() {
        let ns = NineSlice::new(1, 10.0);
        let quads = NineSliceRenderer::new().render(&ns, Rect::new(0.0, 0.0, 10.0, 10.0), [1.0; 4]);
        let area: f32 = quads.iter().map(|q| q.size[0] * q.size[1]).sum();
        assert!((area - 100.0).abs() < 0.01);
    }
}
