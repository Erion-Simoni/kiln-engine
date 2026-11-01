//! Import settings per asset.
//!
//! Defines how individual asset files are processed during import,
//! such as texture filtering, mipmap generation, and compression.

use serde::{Deserialize, Serialize};

/// Import settings for an individual asset.
///
/// These settings control how an asset is processed during import
/// and can be configured per-asset in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSettings {
    /// Texture filtering mode (nearest, linear).
    pub filtering: TextureFilter,
    /// Whether to generate mipmaps for textures.
    pub generate_mipmaps: bool,
    /// Whether to compress textures for the target platform.
    pub compress: bool,
    /// Maximum texture size in pixels (0 = no limit).
    pub max_size: u32,
    /// Whether the texture should be treated as a sprite (pixel-perfect).
    pub is_sprite: bool,
}

/// Filtering mode for texture sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureFilter {
    /// Nearest-neighbor filtering (pixel-perfect, no blurring).
    Nearest,
    /// Linear interpolation filtering (smooth, blurred).
    Linear,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            filtering: TextureFilter::Linear,
            generate_mipmaps: true,
            compress: true,
            max_size: 0,
            is_sprite: false,
        }
    }
}

impl ImportSettings {
    /// Create import settings optimized for sprite/pixel-art textures.
    pub fn sprite() -> Self {
        Self {
            filtering: TextureFilter::Nearest,
            generate_mipmaps: false,
            compress: false,
            max_size: 0,
            is_sprite: true,
        }
    }

    /// Create import settings optimized for photographic/texture assets.
    pub fn texture() -> Self {
        Self {
            filtering: TextureFilter::Linear,
            generate_mipmaps: true,
            compress: true,
            max_size: 0,
            is_sprite: false,
        }
    }
}