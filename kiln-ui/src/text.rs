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
