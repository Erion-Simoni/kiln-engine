//! Sprite component for 2D rendering

use kiln_render::{TextureId, FULL_UV};

/// CPU-side sprite drawn via [`kiln_render::SpriteRenderer`].
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub color: [f32; 4],
    pub size: [f32; 2],
    /// Normalised UV rect into [`Self::texture`].
    pub uv_rect: [f32; 4],
    /// GPU texture handle (`TextureId::WHITE` = solid colour tint).
    pub texture: TextureId,
}

impl Sprite {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            size: [32.0, 32.0],
            uv_rect: FULL_UV,
            texture: TextureId::WHITE,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }

    pub fn with_texture(mut self, texture: TextureId) -> Self {
        self.texture = texture;
        self
    }

    pub fn with_uv(mut self, uv_rect: [f32; 4]) -> Self {
        self.uv_rect = uv_rect;
        self
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}
