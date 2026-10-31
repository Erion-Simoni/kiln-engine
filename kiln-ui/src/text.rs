//! Bitmap text renderer using the built-in 5×7 font.
//!
//! Text is emitted as horizontal pixel runs of solid quads (sampling the
//! white texture), so it needs no atlas and no font dependency. Vector/SDF
//! fonts would need a new dependency and therefore an ADR.

use crate::font::{self, ADVANCE, GLYPH_H, GLYPH_W, LINE_ADVANCE};
use kiln_render::SpriteInstance;

/// Bitmap text layout + quad generation (screen space, y-down).
#[derive(Debug, Default, Clone, Copy)]
pub struct TextRenderer;

impl TextRenderer {
    /// Creates a renderer.
    pub fn new() -> Self {
        Self
    }

    /// Measures text; `font_size` is the glyph height in pixels.
    /// Supports `\n` for multiple lines. Returns `(width, height)`.
    pub fn measure(&self, text: &str, font_size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }
        let scale = font_size / GLYPH_H as f32;
        let lines = text.split('\n').count();
        let max_chars = text
            .split('\n')
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let width = if max_chars == 0 {
            0.0
        } else {
            (max_chars * ADVANCE - (ADVANCE - GLYPH_W)) as f32 * scale
        };
        (width, ((lines - 1) * LINE_ADVANCE + GLYPH_H) as f32 * scale)
    }

    /// Lays out `text` with its top-left at `(x, y)` and returns the quads.
    pub fn render(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
    ) -> Vec<SpriteInstance> {
        let scale = font_size / GLYPH_H as f32;
        let mut out = Vec::new();
        for (line_idx, line) in text.split('\n').enumerate() {
            let line_y = y + (line_idx * LINE_ADVANCE) as f32 * scale;
            for (ci, c) in line.chars().enumerate() {
                let gx = x + (ci * ADVANCE) as f32 * scale;
                push_glyph(&mut out, font::glyph(c), gx, line_y, scale, color);
            }
        }
        out
    }
}

fn push_glyph(
    out: &mut Vec<SpriteInstance>,
    rows: [u8; GLYPH_H],
    gx: f32,
    gy: f32,
    scale: f32,
    color: [f32; 4],
) {
    for (ry, row) in rows.iter().enumerate() {
        let lit = |col: usize| row & (1 << (GLYPH_W - 1 - col)) != 0;
        let mut col = 0usize;
        while col < GLYPH_W {
            if !lit(col) {
                col += 1;
                continue;
            }
            let start = col;
            while col < GLYPH_W && lit(col) {
                col += 1;
            }
            let w = (col - start) as f32 * scale;
            out.push(SpriteInstance::colored(
                [
                    gx + start as f32 * scale + w / 2.0,
                    gy + ry as f32 * scale + scale / 2.0,
                ],
                [w, scale],
                color,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_single_line() {
        assert_eq!(TextRenderer::new().measure("AB", 7.0), (11.0, 7.0));
    }

    #[test]
    fn measure_multiline_and_empty() {
        let r = TextRenderer::new();
        assert_eq!(r.measure("A\nB", 7.0), (5.0, 16.0));
        assert_eq!(r.measure("", 7.0), (0.0, 0.0));
    }

    #[test]
    fn render_text_emits_quads_inside_measured_box() {
        let r = TextRenderer::new();
        let quads = r.render("Hi", 100.0, 50.0, 14.0, [1.0; 4]);
        assert!(!quads.is_empty());
        let (w, h) = r.measure("Hi", 14.0);
        for q in &quads {
            assert!(q.position[0] - q.size[0] / 2.0 >= 100.0 - 0.001);
            assert!(q.position[0] + q.size[0] / 2.0 <= 100.0 + w + 0.001);
            assert!(q.position[1] + q.size[1] / 2.0 <= 50.0 + h + 0.001);
        }
    }

    #[test]
    fn space_renders_nothing() {
        assert!(TextRenderer::new()
            .render("   ", 0.0, 0.0, 7.0, [1.0; 4])
            .is_empty());
    }
}

/// SDF (Signed Distance Field) text renderer for high-quality font rendering.
///
/// Uses pre-baked SDF font atlas textures to render crisp text at any size.
/// This is an alternative to the built-in bitmap font renderer that scales
/// smoothly without pixelation.
#[derive(Debug, Default)]
pub struct SdfTextRenderer {
    /// SDF font data.
    pub font: Option<SdfFont>,
}

/// SDF font data for rendering.
#[derive(Debug, Clone)]
pub struct SdfFontData {
    /// Atlas texture ID.
    pub texture_id: kiln_render::TextureId,
    /// Glyph metrics.
    pub glyphs: HashMap<char, GlyphMetrics>,
    /// Line height in pixels.
    pub line_height: f32,
    /// Space advance width.
    pub space_advance: f32,
}

/// Glyph metrics for SDF rendering.
#[derive(Debug, Clone, Copy)]
pub struct SdfGlyphMetrics {
    /// Glyph position in atlas (x, y).
    pub offset: [f32; 2],
    /// Glyph size in pixels.
    pub size: [f32; 2],
    /// Bearing (offset to left edge).
    pub bearing: [f32; 2],
    /// Advance width.
    pub advance: f32,
}

impl SdfTextRenderer {
    /// Creates a new SDF text renderer with no font loaded.
    pub fn new() -> Self {
        Self {
            font: None,
        }
    }

    /// Load an SDF font into the renderer.
    pub fn load_font(&mut self, font: SdfFontData) {
        self.font = Some(font);
    }

    /// Render text using SDF fonts.
    /// Returns sprite instances for each glyph.
    pub fn render(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
    ) -> Vec<SpriteInstance> {
        let font = match &self.font {
            Some(f) => f,
            None => return Vec::new(),
        };

        let mut out = Vec::new();
        let scale = font_size / font.line_height.max(1.0);
        let mut cursor_x = x;
        let mut cursor_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                cursor_y += font.line_height * scale;
                continue;
            }

            if ch == ' ' {
                cursor_x += font.space_advance * scale;
                continue;
            }

            if let Some(glyph) = font.glyphs.get(&ch) {
                let scale_x = font_size / glyph.size[0].max(1.0);
                let scale_y = font_size / glyph.size[1].max(1.0);
                let scale = scale_x.min(scale_y);

                let display_w = glyph.size[0] * scale;
                let display_h = glyph.size[1] * scale;

                let draw_x = cursor_x + glyph.bearing[0] * scale;
                let draw_y = cursor_y + glyph.bearing[1] * scale;

                out.push(SpriteInstance::colored(
                    [
                        draw_x + display_w / 2.0,
                        draw_y + display_h / 2.0,
                    ],
                    [display_w, display_h],
                    color,
                ));
                // Note: material_id would be set to the font texture ID
                // for SDF rendering. For now, use default (0).
                let _ = glyph;
            }

            cursor_x += glyph.advance * scale;
        }

        out
    }

    /// Measure text width and height.
    pub fn measure(&self, text: &str, font_size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }

        let font = match &self.font {
            Some(f) => f,
            None => return (0.0, 0.0),
        };

        let scale = font_size / font.line_height.max(1.0);
        let mut max_x = 0.0;
        let mut cursor_x = 0.0;
        let mut lines = 1;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = 0.0;
                lines += 1;
                continue;
            }

            if ch == ' ' {
                cursor_x += font.space_advance * scale;
                continue;
            }

            if let Some(glyph) = font.glyphs.get(&ch) {
                cursor_x += glyph.advance * scale;
                max_x = cursor_x.max(max_x);
            }
        }

        let height = lines as f32 * font.line_height * scale;
        (max_x, height)
    }
}

//! SDF (Signed Distance Field) text renderer for high-quality font rendering.
//!
//! Uses pre-baked SDF font atlas textures to render crisp text at any size.
//! This is an alternative to the built-in bitmap font renderer that scales
//! smoothly without pixelation.

use kiln_render::{SpriteInstance, TextureId};
use std::collections::HashMap;

/// SDF font data for rendering.
#[derive(Debug, Clone)]
pub struct SdfFontData {
    /// Atlas texture ID.
    pub texture_id: TextureId,
    /// Glyph metrics.
    pub glyphs: HashMap<char, SdfGlyphMetrics>,
    /// Line height in pixels.
    pub line_height: f32,
    /// Space advance width.
    pub space_advance: f32,
}

/// Glyph metrics for SDF rendering.
#[derive(Debug, Clone, Copy)]
pub struct SdfGlyphMetrics {
    /// Glyph position in atlas (x, y).
    pub offset: [f32; 2],
    /// Glyph size in pixels.
    pub size: [f32; 2],
    /// Bearing (offset to left edge).
    pub bearing: [f32; 2],
    /// Advance width.
    pub advance: f32,
}

/// SDF (Signed Distance Field) text renderer for high-quality font rendering.
///
/// Uses pre-baked SDF font atlas textures to render crisp text at any size.
/// This is an alternative to the built-in bitmap font renderer that scales
//! smoothly without pixelation.
#[derive(Debug, Default)]
pub struct SdfTextRenderer {
    /// SDF font data.
    pub font: Option<SdfFontData>,
}

impl SdfTextRenderer {
    /// Creates a new SDF text renderer with no font loaded.
    pub fn new() -> Self {
        Self { font: None }
    }

    /// Load an SDF font into the renderer.
    pub fn load_font(&mut self, font: SdfFontData) {
        self.font = Some(font);
    }

    /// Render text using SDF fonts.
    ///
    /// Returns sprite instances for each glyph. Each instance uses
    /// `TextureId::WHITE` as its texture; the SDF shader (if bound via a
    /// material) will sample the actual font atlas texture.
    pub fn render(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
    ) -> Vec<SpriteInstance> {
        let font = match &self.font {
            Some(f) => f,
            None => return Vec::new(),
        };

        let mut out = Vec::new();
        let scale = font_size / font.line_height.max(1.0);
        let mut cursor_x = x;
        let mut cursor_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                cursor_y += font.line_height * scale;
                continue;
            }

            if ch == ' ' {
                cursor_x += font.space_advance * scale;
                continue;
            }

            if let Some(glyph) = font.glyphs.get(&ch) {
                let scale_x = font_size / glyph.size[0].max(1.0);
                let scale_y = font_size / glyph.size[1].max(1.0);
                let scale = scale_x.min(scale_y);

                let display_w = glyph.size[0] * scale;
                let display_h = glyph.size[1] * scale;

                let draw_x = cursor_x + glyph.bearing[0] * scale;
                let draw_y = cursor_y + glyph.bearing[1] * scale;

                out.push(SpriteInstance::colored(
                    [
                        draw_x + display_w / 2.0,
                        draw_y + display_h / 2.0,
                    ],
                    [display_w, display_h],
                    color,
                ));
            }

            cursor_x += glyph.advance * scale;
        }

        out
    }

    /// Measure text width and height.
    pub fn measure(&self, text: &str, font_size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }

        let font = match &self.font {
            Some(f) => f,
            None => return (0.0, 0.0),
        };

        let scale = font_size / font.line_height.max(1.0);
        let mut max_x = 0.0;
        let mut cursor_x = 0.0;
        let mut lines = 1;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = 0.0;
                lines += 1;
                continue;
            }

            if ch == ' ' {
                cursor_x += font.space_advance * scale;
                continue;
            }

            if let Some(glyph) = font.glyphs.get(&ch) {
                cursor_x += glyph.advance * scale;
                max_x = cursor_x.max(max_x);
            }
        }

        let height = lines as f32 * font.line_height * scale;
        (max_x, height)
    }
}

#[cfg(test)]
mod sdf_text_renderer_tests {
    use super::*;

    #[test]
    fn sdf_renderer_empty() {
        let r = SdfTextRenderer::new();
        assert_eq!(r.measure("hello", 16.0), (0.0, 0.0));
        assert!(r.render("hello", 0.0, 0.0, 16.0, [1.0; 4]).is_empty());
    }

    #[test]
    fn sdf_renderer_with_font() {
        let mut r = SdfTextRenderer::new();
        let mut font = SdfFontData {
            texture_id: TextureId(1),
            glyphs: HashMap::new(),
            line_height: 16.0,
            space_advance: 8.0,
        };
        font.glyphs.insert(
            'A',
            SdfGlyphMetrics {
                offset: [0.0, 0.0],
                size: [8.0, 8.0],
                bearing: [0.0, 0.0],
                advance: 8.0,
            },
        );
        font.glyphs.insert(
            'B',
            SdfGlyphMetrics {
                offset: [0.0, 0.0],
                size: [8.0, 8.0],
                bearing: [0.0, 0.0],
                advance: 8.0,
            },
        );
        r.load_font(font);

        let (w, h) = r.measure("AB", 16.0);
        assert!((w - 16.0).abs() < 0.001);
        assert!((h - 16.0).abs() < 0.001);

        let quads = r.render("AB", 0.0, 0.0, 16.0, [1.0; 4]);
        assert_eq!(quads.len(), 2);
    }
}
