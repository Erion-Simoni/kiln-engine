//! SDF (Signed Distance Field) font atlas builder.
//!
//! Takes a TTF font file, rasterizes glyphs using `fontdue`, generates
//! signed-distance-field data from the bitmaps, and packs everything into
//! a texture atlas with [`SdfFont`] metadata.
//!
//! This module is only compiled when the `fonts` Cargo feature is enabled
//! in `kiln-ui` (which pulls in `fontdue`). Without the feature, SDF fonts
//! can still be used with manually-authored atlas data.

use crate::SdfTextRenderer;
use std::collections::HashMap;
use image::{GrayImage, Luma, RgbaImage};
use kiln_core::project::{GlyphMetrics, SdfFont};

/// Configuration for building an SDF font atlas.
#[derive(Debug, Clone)]
pub struct SdfBuilderConfig {
    /// Font size in pixels used for rasterization.
    pub font_size: f32,
    /// Atlas texture width in pixels.
    pub atlas_width: u32,
    /// Atlas texture height in pixels.
    pub atlas_height: u32,
    /// Padding between glyphs in the atlas (in pixels).
    pub padding: u32,
    /// Characters to include in the atlas.
    pub chars: Vec<char>,
    /// SDF spread: the distance range (in pixels) over which the distance
    /// field transitions from 1.0 (inside) to 0.0 (outside).
    pub spread: u32,
}

impl Default for SdfBuilderConfig {
    fn default() -> Self {
        Self {
            font_size: 32.0,
            atlas_width: 512,
            atlas_height: 512,
            padding: 2,
            chars: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?:;'\"-()[]{}"
                .chars()
                .collect(),
            spread: 8,
        }
    }
}

/// Build an SDF atlas from TTF bytes.
///
/// # Errors
/// Returns a `String` if the TTF data is invalid, a glyph cannot be
/// rasterized, or the atlas is too small to fit all glyphs.
pub fn build_sdf_atlas(
    ttf_data: &[u8],
    config: &SdfBuilderConfig,
) -> Result<SdfAtlasResult, String> {
    // Parse the TTF font with fontdue.
    let font = fontdue::Font::from_bytes(ttf_data, fontdue::FontSettings::default())
        .map_err(|e| format!("Failed to parse TTF font: {e}"))?;

    let mut atlas = RgbaImage::new(config.atlas_width, config.atlas_height);
    let mut glyphs = HashMap::new();
    let mut x: u32 = config.padding;
    let mut y: u32 = config.padding;
    let mut row_height: u32 = 0;

    for ch in &config.chars {
        // Rasterize the glyph to a bitmap.
        let (metrics, bitmap) = font.rasterize(*ch, config.font_size as f32);
        if bitmap.is_empty() {
            // Skip glyphs that have no visible pixels (e.g. space).
            continue;
        }

        let glyph_w = metrics.width as u32;
        let glyph_h = metrics.height as u32;

        // Check if the glyph fits on the current row.
        if x + glyph_w + config.padding > config.atlas_width {
            x = config.padding;
            y += row_height + config.padding;
            row_height = 0;
        }

        if y + glyph_h + config.padding > config.atlas_height {
            return Err(format!(
                "Atlas too small: glyph '{ch}' ({glyph_w}x{glyph_h}) does not fit"
            ));
        }

        // Generate SDF from the bitmap.
        let sdf = bitmap_to_sdf(&bitmap, config.spread);

        // Copy the SDF into the atlas (all channels = SDF value).
        for (dy, row) in sdf.rows().enumerate() {
            for (dx, &val) in row.iter().enumerate() {
                let ax = x + dx as u32;
                let ay = y + dy as u32;
                if ax < config.atlas_width && ay < config.atlas_height {
                    let pixel = atlas.get_pixel_mut(ax, ay);
                    pixel[0] = val;
                    pixel[1] = val;
                    pixel[2] = val;
                    pixel[3] = 255;
                }
            }
        }

        let advance = metrics.advance as f32;
        let bearing = [metrics.bearing[0] as f32, metrics.bearing[1] as f32];

        glyphs.insert(
            *ch,
            GlyphMetrics {
                codepoint: *ch,
                offset: [x as f32, y as f32],
                size: [glyph_w as f32, glyph_h as f32],
                bearing,
                advance,
            },
        );

        x += glyph_w + config.padding;
        row_height = row_height.max(glyph_h);
    }

    Ok(SdfAtlasResult {
        atlas,
        font: SdfFont {
            id: "imported".to_string(),
            name: "imported".to_string(),
            texture_path: "".to_string(),
            atlas_width: config.atlas_width,
            atlas_height: config.atlas_height,
            glyphs,
            line_height: config.font_size,
            space_advance: config.font_size * 0.5,
            tab_width: 4,
        },
    })
}

/// Convert a grayscale bitmap to a signed-distance-field.
///
/// For each pixel, computes the distance to the nearest edge pixel.
/// Inside the glyph (opaque): positive values fading from spread to 0.
/// Outside the glyph (transparent): negative values fading from 0 to -spread.
fn bitmap_to_sdf(bitmap: &GrayImage, spread: u32) -> GrayImage {
    let w = bitmap.width() as i32;
    let h = bitmap.height() as i32;
    let mut dist: Vec<Vec<i32>> = vec![vec![i32::MAX; w as usize]; h as usize];

    // Pass 1: forward (top-left to bottom-right)
    for y in 0..h {
        for x in 0..w {
            if is_edge_pixel(bitmap, x, y) {
                dist[y as usize][x as usize] = 0;
            } else {
                let mut d = i32::MAX;
                if x > 0 {
                    d = d.min(dist[y as usize][(x - 1) as usize] + 1);
                }
                if y > 0 {
                    d = d.min(dist[(y - 1) as usize][x as usize] + 1);
                }
                dist[y as usize][x as usize] = d;
            }
        }
    }

    // Pass 2: backward (bottom-right to top-left)
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut d = dist[y as usize][x as usize];
            if x < w - 1 {
                d = d.min(dist[y as usize][(x + 1) as usize] + 1);
            }
            if y < h - 1 {
                d = d.min(dist[(y + 1) as usize][x as usize] + 1);
            }
            dist[y as usize][x as usize] = d;
        }
    }

    let spread = spread as f32;
    let mut sdf = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let inside = bitmap.get_pixel(x as u32, y as u32)[0] > 127;
            let d = dist[y as usize][x as usize] as f32;
            let val = if inside {
                (1.0 - (d / spread).clamp(0.0, 1.0)) * 255.0
            } else {
                (d / spread).clamp(0.0, 1.0) * 255.0
            };
            sdf.put_pixel(x as u32, y as u32, Luma([val as u8]));
        }
    }

    sdf
}

/// Check if a pixel is on the edge of the glyph (different from a neighbor).
fn is_edge_pixel(bitmap: &GrayImage, x: i32, y: i32) -> bool {
    let w = bitmap.width() as i32;
    let h = bitmap.height() as i32;
    let center = bitmap.get_pixel(x as u32, y as u32)[0] > 127;

    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                let neighbor = bitmap.get_pixel(nx as u32, ny as u32)[0] > 127;
                if neighbor != center {
                    return true;
                }
            } else if center {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = SdfBuilderConfig::default();
        assert!(config.font_size > 0.0);
        assert!(config.atlas_width > 0);
        assert!(!config.chars.is_empty());
    }
}