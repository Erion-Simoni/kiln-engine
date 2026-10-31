# SDF (Signed Distance Field) Text Rendering

Kiln supports SDF (Signed Distance Field) font rendering for high-quality text at any size. SDF fonts use pre-baked texture atlases with distance field data, allowing crisp text rendering at any resolution without pixelation.

## Overview

SDF fonts store distance information for each glyph in a texture atlas. The shader samples the SDF texture and uses the distance value to determine whether a pixel is inside or outside the glyph shape, allowing for crisp rendering at any size.

## SDF Font Format

SDF font assets are stored in `kiln_core::project::SdfFont`:

```rust
pub struct SdfFont {
    pub id: String,
    pub name: String,
    pub texture_path: String,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub glyphs: HashMap<char, GlyphMetrics>,
    pub line_height: f32,
    pub space_advance: f32,
    pub tab_width: u32,
}

pub struct GlyphMetrics {
    pub codepoint: char,
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub bearing: [f32; 2],
    pub advance: f32,
}
```

## Using SDF Fonts

### 1. Create an SDF Font Asset

Add an SDF font to your project's `kiln.toml`:

```toml
[sdf_fonts.my_font]
id = "my_font"
name = "My Font"
texture_path = "assets/fonts/my_font.png"
atlas_width = 512
atlas_height = 512
line_height = 16.0
space_advance = 8.0

[sdf_fonts.my_font.glyphs.'A']
codepoint = 65
offset = [0.0, 0.0]
size = [8.0, 8.0]
bearing = [0.0, 0.0]
advance = 8.0
```

### 2. Use SDF Font in Game

```rust
use kiln_ui::SdfTextRenderer;
use kiln_ui::text::SdfFontData;
use kiln_render::TextureId;
use std::collections::HashMap;

let mut text_renderer = SdfTextRenderer::new();

// Load font data
let font_data = SdfFontData {
    texture_id: TextureId(1),
    glyphs: HashMap::new(),
    line_height: 16.0,
    space_advance: 8.0,
};
text_renderer.load_font(font_data);

// Render text
let instances = text_renderer.render("Hello, World!", 100.0, 100.0, 32.0, [1.0, 1.0, 1.0, 1.0]);
```

## Benefits

- **Crisp rendering at any size** - SDF fonts scale without pixelation
- **Small texture size** - SDF textures are compact
- **Fast rendering** - Simple shader sampling
- **Good for UI** - Ideal for UI text, titles, and headings

## Limitations

- **Not suitable for complex scripts** - Works best for Latin-based scripts
- **Requires pre-baked atlas** - Font must be pre-baked into an SDF atlas
- **Limited font effects** - Limited to simple color tints
