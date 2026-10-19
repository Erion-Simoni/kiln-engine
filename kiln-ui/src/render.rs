//! Rendering support for UI elements (9-slice sprites and text).

use crate::layout::Rect;
use serde::{Deserialize, Serialize};

/// 9-slice sprite configuration for scalable UI elements.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NineSlice {
    /// Sprite texture ID.
    pub texture_id: u64,
    /// Left border width (in pixels).
    pub border_left: f32,
    /// Right border width (in pixels).
    pub border_right: f32,
    /// Top border height (in pixels).
    pub border_top: f32,
    /// Bottom border height (in pixels).
    pub border_bottom: f32,
}

impl NineSlice {
    pub fn new(texture_id: u64, border: f32) -> Self {
        Self {
            texture_id,
            border_left: border,
            border_right: border,
            border_top: border,
            border_bottom: border,
        }
    }

    pub fn with_borders(
        mut self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    ) -> Self {
        self.border_left = left;
        self.border_right = right;
        self.border_top = top;
        self.border_bottom = bottom;
        self
    }
}

/// Renderer for 9-slice sprites.
///
/// Generates 9 sprite instances (corners, edges, center) from a single texture
/// with border regions that don't stretch when the sprite is scaled.
pub struct NineSliceRenderer {
    // Stub: full integration with kiln-render sprite batch deferred
}

impl NineSliceRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Renders a 9-slice sprite to the given bounds.
    ///
    /// Full implementation will:
    /// 1. Calculate 9 regions based on border sizes and target bounds
    /// 2. Generate 9 SpriteInstance structs with appropriate UV coordinates
    /// 3. Add them to the kiln-render sprite batch
    pub fn render(&self, nine_slice: &NineSlice, bounds: Rect) {
        let _ = (nine_slice, bounds);
        // TODO: Generate 9 sprite instances and add to sprite batch
    }
}

impl Default for NineSliceRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Text renderer with SDF/MSDF support.
///
/// NOTE: Full SDF/MSDF text rendering requires:
/// - Font loading (e.g., fontdue, ab_glyph)
/// - SDF/MSDF generation (msdfgen or similar)
/// - Font atlas generation and management
/// - Font subsetting for size optimization
/// - Text layout engine (line breaking, alignment)
/// - SDF/MSDF shaders for distance-field rendering
///
/// This is deferred to a future milestone; stub provided for API design.
pub struct TextRenderer {
    // Stub: font atlas, SDF/MSDF shaders, text layout deferred
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Measures text dimensions.
    ///
    /// Full implementation will calculate text width and height based on font metrics.
    pub fn measure(&self, text: &str, font_size: f32) -> (f32, f32) {
        let _ = (text, font_size);
        (0.0, 0.0)
    }

    /// Renders text at the given position.
    ///
    /// Full implementation will:
    /// 1. Layout text (glyph positions, line breaks)
    /// 2. Generate quads for each glyph with UV coordinates from font atlas
    /// 3. Add to sprite batch with SDF/MSDF shader
    pub fn render(&self, text: &str, x: f32, y: f32, font_size: f32) {
        let _ = (text, x, y, font_size);
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nine_slice_new() {
        let texture_id = 123;
        let nine_slice = NineSlice::new(texture_id, 8.0);

        assert_eq!(nine_slice.texture_id, texture_id);
        assert_eq!(nine_slice.border_left, 8.0);
        assert_eq!(nine_slice.border_right, 8.0);
        assert_eq!(nine_slice.border_top, 8.0);
        assert_eq!(nine_slice.border_bottom, 8.0);
    }

    #[test]
    fn test_nine_slice_with_borders() {
        let texture_id = 123;
        let nine_slice = NineSlice::new(texture_id, 8.0)
            .with_borders(10.0, 12.0, 8.0, 14.0);

        assert_eq!(nine_slice.border_left, 10.0);
        assert_eq!(nine_slice.border_right, 12.0);
        assert_eq!(nine_slice.border_top, 8.0);
        assert_eq!(nine_slice.border_bottom, 14.0);
    }
}
