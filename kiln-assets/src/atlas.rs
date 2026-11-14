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
    sprites: Vec<(String, Image, Rect)>,
    invalid_configuration: bool,
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
            invalid_configuration: false,
            padding: 2,
        }
    }

    /// Set padding before adding sprites. Changing it after packing is an error.
    pub fn with_padding(mut self, padding: u32) -> Self {
        self.invalid_configuration |= !self.sprites.is_empty() && self.padding != padding;
        self.padding = padding;
        self
    }

    /// Add a sprite to the atlas.
    pub fn add_sprite(&mut self, name: &str, image: Image) -> Result<(), AtlasError> {
        self.validate_configuration()?;
        let w = image.width;
        let h = image.height;
        if w == 0
            || h == 0
            || w > self.max_width
            || h > self.max_width
            || image.pixels.len() != w as usize * h as usize * 4
        {
            return Err(AtlasError::InvalidImage);
        }
        if name.is_empty() || self.sprites.iter().any(|(existing, _, _)| existing == name) {
            return Err(AtlasError::DuplicateName);
        }
        // Stage the next shelf: rejected sprites must not change accepted layout.
        let mut row_y = self.current_y;
        let mut row_h = self.current_max_h;
        let mut x = self
            .current_x
            .checked_add(self.padding)
            .ok_or(AtlasError::AtlasFull)?;
        if x.checked_add(w)
            .and_then(|v| v.checked_add(self.padding))
            .is_none_or(|v| v > self.max_width)
        {
            row_y = row_y
                .checked_add(row_h)
                .and_then(|v| v.checked_add(self.padding))
                .ok_or(AtlasError::AtlasFull)?;
            row_h = 0;
            x = self.padding;
        }
        let y = row_y
            .checked_add(self.padding)
            .ok_or(AtlasError::AtlasFull)?;
        if x.checked_add(w)
            .and_then(|v| v.checked_add(self.padding))
            .is_none_or(|v| v > self.max_width)
            || y.checked_add(h)
                .and_then(|v| v.checked_add(self.padding))
                .is_none_or(|v| v > self.max_width)
        {
            return Err(AtlasError::AtlasFull);
        }
        self.sprites
            .push((name.to_owned(), image, Rect::new(x, y, w, h)));
        self.current_x = x + w;
        self.current_y = row_y;
        self.current_max_h = row_h.max(h);
        Ok(())
    }

    fn validate_configuration(&self) -> Result<(), AtlasError> {
        if self.invalid_configuration
            || self.max_width == 0
            || self.max_width > crate::image::MAX_DIMENSION
        {
            return Err(AtlasError::InvalidConfiguration);
        }
        Ok(())
    }

    /// Build an atlas, returning errors for invalid dimensions or allocation failure.
    pub fn try_build(self) -> Result<TextureAtlas, AtlasError> {
        self.validate_configuration()?;
        let bytes = self.max_width as usize * self.max_width as usize * 4;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes)
            .map_err(|_| AtlasError::AllocationFailed)?;
        pixels.resize(bytes, 0);
        let mut texture = Image::from_rgba(self.max_width, self.max_width, pixels)
            .ok_or(AtlasError::InvalidConfiguration)?;
        let mut sprites = HashMap::new();
        for (name, image, rect) in self.sprites {
            for y in 0..rect.h {
                let src = (y * rect.w * 4) as usize;
                let dst = ((rect.y + y) * self.max_width * 4 + rect.x * 4) as usize;
                let len = rect.w as usize * 4;
                texture.pixels[dst..dst + len].copy_from_slice(&image.pixels[src..src + len]);
            }
            sprites.insert(
                name,
                UvRect {
                    u0: rect.x as f32 / self.max_width as f32,
                    v0: rect.y as f32 / self.max_width as f32,
                    u1: (rect.x + rect.w) as f32 / self.max_width as f32,
                    v1: (rect.y + rect.h) as f32 / self.max_width as f32,
                },
            );
        }
        Ok(TextureAtlas {
            name: String::new(),
            texture,
            sprites,
        })
    }

    /// Build a previously validated atlas. Use `try_build` for external input.
    pub fn build(self) -> TextureAtlas {
        self.try_build().expect("invalid atlas configuration")
    }
}

/// Errors that can occur during atlas building.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AtlasError {
    #[error("Atlas dimensions or padding configuration are invalid")]
    InvalidConfiguration,
    #[error("Atlas image dimensions or RGBA data are invalid")]
    InvalidImage,
    #[error("Atlas sprite names must be nonempty and unique")]
    DuplicateName,
    #[error("Cannot allocate the atlas texture")]
    AllocationFailed,
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
    let mut atlas = builder.try_build()?;
    atlas.name = name.to_string();
    Ok(atlas)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(w: u32, h: u32, color: [u8; 4]) -> Image {
        Image::from_rgba(w, h, color.repeat((w * h) as usize)).unwrap()
    }
    #[test]
    fn accepted_shelves_keep_identical_uvs_and_pixels_at_the_edges() {
        let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
        let mut b = AtlasBuilder::new(12).with_padding(1);
        b.add_sprite("red", image(5, 3, colors[0])).unwrap();
        b.add_sprite("green", image(4, 3, colors[1])).unwrap();
        b.add_sprite("blue", image(10, 5, colors[2])).unwrap();
        let a = b.try_build().unwrap();
        for (name, color) in ["red", "green", "blue"].iter().zip(colors) {
            let uv = a.sprites[*name];
            let (x0, y0, x1, y1) = (
                (uv.u0 * 12.0).round() as u32,
                (uv.v0 * 12.0).round() as u32,
                (uv.u1 * 12.0).round() as u32,
                (uv.v1 * 12.0).round() as u32,
            );
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * 12 + x) * 4) as usize;
                    assert_eq!(&a.texture.pixels[i..i + 4], &color);
                }
            }
        }
        assert_eq!(&a.texture.pixels[..4], &[0, 0, 0, 0]);
        assert_eq!(a.sprites["blue"].v0, 5.0 / 12.0);
    }
    #[test]
    fn rejected_images_do_not_poison_the_remaining_shelf() {
        let mut b = AtlasBuilder::new(10).with_padding(1);
        b.add_sprite("first", image(3, 3, [1; 4])).unwrap();
        assert_eq!(
            b.add_sprite("too_large", image(9, 9, [2; 4])),
            Err(AtlasError::AtlasFull)
        );
        assert_eq!(
            b.add_sprite("first", image(1, 1, [2; 4])),
            Err(AtlasError::DuplicateName)
        );
        assert_eq!(
            b.add_sprite(
                "broken",
                Image {
                    width: 1,
                    height: 1,
                    pixels: vec![]
                }
            ),
            Err(AtlasError::InvalidImage)
        );
        b.add_sprite("last", image(4, 3, [3; 4])).unwrap();
        let a = b.try_build().unwrap();
        assert_eq!(a.sprites.len(), 2);
        assert_eq!(a.sprites["last"].u0, 0.5);
        assert_eq!(a.sprites["last"].v0, 0.1);
    }
    #[test]
    fn dimensions_padding_and_empty_names_are_validated_without_allocation() {
        for size in [0, u32::MAX, crate::image::MAX_DIMENSION + 1] {
            assert!(matches!(
                AtlasBuilder::new(size).try_build(),
                Err(AtlasError::InvalidConfiguration)
            ));
        }
        let mut b = AtlasBuilder::new(10).with_padding(u32::MAX);
        assert_eq!(
            b.add_sprite("x", image(1, 1, [0; 4])),
            Err(AtlasError::AtlasFull)
        );
        let mut b = AtlasBuilder::new(10);
        assert_eq!(
            b.add_sprite("", image(1, 1, [0; 4])),
            Err(AtlasError::DuplicateName)
        );
        b.add_sprite("ok", image(1, 1, [0; 4])).unwrap();
        assert!(matches!(
            b.with_padding(0).try_build(),
            Err(AtlasError::InvalidConfiguration)
        ));
    }
}
