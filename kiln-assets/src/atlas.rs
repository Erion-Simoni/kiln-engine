//! Texture atlas generation.
//!
//! Automatically packs multiple sprite textures into a single texture
//! atlas to reduce draw calls and improve rendering performance.

use crate::image::Image;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A packed texture atlas containing multiple sprites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureAtlas {
    /// Name of the atlas.
    pub name: String,
    /// The packed texture image (RGBA8).
    pub texture: Image,
    /// Mapping from sprite name to its UV rect in the atlas.
    pub sprites: HashMap<String, UvRect>,
}

/// A UV rectangle in normalized [0, 1] coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// A rectangular region in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
}

/// Texture atlas builder using a simple shelf packing algorithm.
pub struct AtlasBuilder {
    /// Current y offset for the current shelf.
    current_y: u32,
    /// Current max height in the current shelf.
    current_max_h: u32,
    /// Current x offset within the current shelf.
    current_x: u32,
    /// Maximum texture width (for limiting atlas size).
    max_width: u32,
    /// Sprites packed so far.
    sprites: Vec<(String, Image)>,
    /// Padding between sprites in pixels.
    padding: u32,
}

impl AtlasBuilder {
    /// Create a new atlas builder.
    pub fn new(max_width: u32) -> Self {
        Self {
            current_y: 0,
            current_max_h: 0,
            current_x: 0,
            max_width,
            sprites: Vec::new(),
            padding: 2,
        }
    }

    /// Set padding between sprites in pixels.
    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    /// Add a sprite to the atlas.
    pub fn add_sprite(&mut self, name: &str, image: Image) -> Result<(), AtlasError> {
        let w = image.width;
        let h = image.height;

        // Check if the sprite fits in the current shelf
        if self.current_x + self.padding + w > self.max_width {
            // Move to next shelf
            self.current_y += self.current_max_h + self.padding;
            self.current_x = self.padding;
            self.current_max_h = 0;
        }

        // Check if it fits vertically
        if self.current_y + self.padding + h > self.max_width {
            return Err(AtlasError::AtlasFull);
        }

        let _x = self.current_x + self.padding;
        let _y = self.current_y + self.padding;

        self.sprites.push((name.to_string(), image));
        self.current_x += self.padding + w;
        self.current_max_h = self.current_max_h.max(h);

        Ok(())
    }

    /// Build the atlas from all added sprites.
    pub fn build(self) -> TextureAtlas {
        let mut texture = Image::from_rgba(
            self.max_width,
            self.max_width,
            vec![0; self.max_width as usize * self.max_width as usize * 4],
        )
        .expect("failed to create blank texture");

        let mut sprites = HashMap::new();

        let mut current_x = self.padding;
        let mut current_y = self.padding;
        let mut current_max_h = 0;

        for (name, image) in self.sprites {
            let w = image.width;
            let h = image.height;

            if current_x + self.padding + w > self.max_width {
                current_y += current_max_h + self.padding;
                current_x = self.padding;
                current_max_h = 0;
            }

            let uv_rect = UvRect {
                u0: current_x as f32 / self.max_width as f32,
                v0: current_y as f32 / self.max_width as f32,
                u1: (current_x + w) as f32 / self.max_width as f32,
                v1: (current_y + h) as f32 / self.max_width as f32,
            };

            // Copy image pixels into texture
            for y in 0..h {
                for x in 0..w {
                    let src_idx = ((y * image.width + x) * 4) as usize;
                    let dst_x = current_x + x;
                    let dst_y = current_y + y;
                    let dst_idx = ((dst_y * self.max_width + dst_x) * 4) as usize;
                    texture.pixels[dst_idx..dst_idx + 4]
                        .copy_from_slice(&image.pixels[src_idx..src_idx + 4]);
                }
            }

            sprites.insert(name, uv_rect);

            current_x += self.padding + w;
            current_max_h = current_max_h.max(h);
        }

        TextureAtlas {
            name: String::new(),
            texture,
            sprites,
        }
    }
}

/// Errors that can occur during atlas building.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AtlasError {
    #[error("Atlas is full: cannot fit more sprites")]
    AtlasFull,
}

/// Automatically pack sprites into a texture atlas.
pub fn pack_atlas(
    name: &str,
    sprites: &[(String, Image)],
    max_width: u32,
) -> Result<TextureAtlas, AtlasError> {
    let mut builder = AtlasBuilder::new(max_width);
    for (name, image) in sprites {
        builder.add_sprite(name, image.clone())?;
    }
    let mut atlas = builder.build();
    atlas.name = name.to_string();
    Ok(atlas)
}
