//! Alpha-atlas fonts for saved screens. One textured quad per glyph.
use kiln_core::ui::UiDocument;
use kiln_render::{SpriteInstance, TextureId};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct FontGlyph {
    pub uv: [f32; 4],
    pub size: [f32; 2],
    pub bearing: [f32; 2],
    pub advance: f32,
}
#[derive(Debug, Clone)]
pub struct AtlasFont {
    pub texture: TextureId,
    pub glyphs: BTreeMap<char, FontGlyph>,
    pub line_height: f32,
}
#[derive(Default, Debug, Clone)]
pub struct UiFonts {
    pub fonts: BTreeMap<String, AtlasFont>,
}

/// A CPU atlas can be inspected in tests or uploaded into either host's UI renderer.
pub struct FontAtlas {
    pub rgba: Vec<u8>,
    pub size: [u32; 2],
    pub font: AtlasFont,
}

#[cfg(feature = "fonts")]
pub fn build_atlas(
    bytes: &[u8],
    chars: impl IntoIterator<Item = char>,
) -> Result<FontAtlas, String> {
    use std::collections::BTreeSet;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("UI font file exceeds 16 MiB".into());
    }
    let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
        .map_err(|error| error.to_string())?;
    let chars: BTreeSet<_> = (32_u8..=126)
        .map(char::from)
        .chain(chars)
        .filter(|ch| !ch.is_control())
        .collect();
    if chars.len() > 2048 {
        return Err("UI font exceeds 2048 baked characters".into());
    }
    let metrics = font
        .horizontal_line_metrics(48.0)
        .ok_or("Font has no horizontal metrics")?;
    let mut out = FontAtlas {
        rgba: vec![0; 1024 * 1024 * 4],
        size: [1024, 1024],
        font: AtlasFont {
            texture: TextureId::WHITE,
            glyphs: BTreeMap::new(),
            line_height: metrics.new_line_size,
        },
    };
    let (mut x, mut y, mut row) = (2, 2, 0);
    for ch in chars {
        let (m, pixels) = font.rasterize(ch, 48.0);
        if m.width + 4 > 1024 || m.height + 4 > 1024 {
            return Err("UI glyph is too large".into());
        }
        if x + m.width + 2 > 1024 {
            x = 2;
            y += row + 2;
            row = 0;
        }
        if y + m.height + 2 > 1024 {
            return Err("UI font atlas is full; reduce extra characters".into());
        }
        for gy in 0..m.height {
            for gx in 0..m.width {
                let index = ((y + gy) * 1024 + x + gx) * 4;
                out.rgba[index..index + 4].copy_from_slice(&[
                    255,
                    255,
                    255,
                    pixels[gy * m.width + gx],
                ]);
            }
        }
        out.font.glyphs.insert(
            ch,
            FontGlyph {
                uv: [
                    x as f32 / 1024.0,
                    y as f32 / 1024.0,
                    (x + m.width) as f32 / 1024.0,
                    (y + m.height) as f32 / 1024.0,
                ],
                size: [m.width as f32, m.height as f32],
                bearing: [
                    m.xmin as f32,
                    metrics.ascent - m.ymin as f32 - m.height as f32,
                ],
                advance: m.advance_width,
            },
        );
        x += m.width + 2;
        row = row.max(m.height);
    }
    Ok(out)
}

#[cfg(not(feature = "fonts"))]
pub fn build_atlas(
    _bytes: &[u8],
    _chars: impl IntoIterator<Item = char>,
) -> Result<FontAtlas, String> {
    Err("Custom UI fonts require the fonts feature".into())
}

impl UiFonts {
    /// Resource reads and uploads are supplied by the host; archive files need no extraction.
    pub fn load(
        document: &UiDocument,
        mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
        mut upload: impl FnMut(&str, [u32; 2], &[u8]) -> TextureId,
    ) -> (Self, Vec<String>) {
        let mut out = Self::default();
        let mut errors = Vec::new();
        if let Err(error) = document.validate() {
            return (out, vec![error]);
        }
        for (id, asset) in &document.fonts {
            let chars = asset
                .extra_chars
                .chars()
                .chain(document.screens.values().flat_map(|s| {
                    s.widgets
                        .iter()
                        .filter(|w| w.font == *id)
                        .flat_map(|w| w.text.chars())
                }));
            match read(&asset.path).and_then(|bytes| build_atlas(&bytes, chars)) {
                Ok(mut atlas) => {
                    atlas.font.texture = upload(id, atlas.size, &atlas.rgba);
                    out.fonts.insert(id.clone(), atlas.font);
                }
                Err(error) => errors.push(format!("UI font '{id}' ({}): {error}", asset.path)),
            }
        }
        (out, errors)
    }
}

impl AtlasFont {
    pub fn measure(&self, text: &str, size: f32) -> [f32; 2] {
        if text.is_empty() {
            return [0.0; 2];
        }
        let scale = size / 48.0;
        let mut width: f32 = 0.0;
        let mut cursor: f32 = 0.0;
        let mut lines = 1;
        for ch in text.chars() {
            if ch == '\n' {
                width = width.max(cursor);
                cursor = 0.0;
                lines += 1;
            } else if let Some(glyph) = self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?')) {
                cursor += glyph.advance * scale;
            }
        }
        [width.max(cursor), self.line_height * lines as f32 * scale]
    }
    pub fn render(
        &self,
        text: &str,
        origin: [f32; 2],
        size: f32,
        color: [f32; 4],
    ) -> Vec<crate::UiQuad> {
        let scale = size / 48.0;
        let mut cursor = origin;
        let mut out = Vec::new();
        for ch in text.chars() {
            if ch == '\n' {
                cursor[0] = origin[0];
                cursor[1] += self.line_height * scale;
                continue;
            }
            if let Some(g) = self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?')) {
                if g.size[0] > 0.0 && g.size[1] > 0.0 {
                    let mut instance = SpriteInstance::colored(
                        [
                            cursor[0] + (g.bearing[0] + g.size[0] * 0.5) * scale,
                            cursor[1] + (g.bearing[1] + g.size[1] * 0.5) * scale,
                        ],
                        g.size.map(|v| v * scale),
                        color,
                    );
                    instance.uv_rect = g.uv;
                    out.push(crate::UiQuad {
                        texture: self.texture,
                        instance,
                    });
                }
                cursor[0] += g.advance * scale;
            }
        }
        out
    }
}
