//! Post-processing pipeline for 2D rendering.
//!
//! Supports:
//! - Bloom (downsample, blur, threshold, upsample, blend)
//! - LUT color grading
//! - Screen-space fog
//! - Outline effect

use wgpu::{Device, TextureView, CommandEncoder};

use crate::tier::RenderTier;

/// Post-processing configuration.
#[derive(Debug, Clone, Copy)]
pub struct PostConfig {
    /// Bloom threshold (pixels brighter than this emit light).
    pub bloom_threshold: f32,
    /// Bloom intensity.
    pub bloom_intensity: f32,
    /// Fog density (0 = no fog, higher = thicker).
    pub fog_density: f32,
    /// Fog color.
    pub fog_color: [f32; 3],
    /// Outline color.
    pub outline_color: [f32; 4],
    /// Outline thickness.
    pub outline_thickness: f32,
}

impl Default for PostConfig {
    fn default() -> Self {
        Self {
            bloom_threshold: 0.8,
            bloom_intensity: 0.5,
            fog_density: 0.0,
            fog_color: [0.5, 0.5, 0.5],
            outline_color: [1.0, 1.0, 1.0, 1.0],
            outline_thickness: 1.0,
        }
    }
}

impl PostConfig {
    /// Get config for a given render tier.
    pub fn for_tier(tier: RenderTier) -> Self {
        match tier {
            RenderTier::Low => Self {
                bloom_threshold: 0.0,
                bloom_intensity: 0.0,
                fog_density: 0.0,
                fog_color: [0.5; 3],
                outline_color: [1.0; 4],
                outline_thickness: 0.0,
            },
            RenderTier::Medium => Self {
                bloom_threshold: 0.8,
                bloom_intensity: 0.4,
                fog_density: 0.1,
                fog_color: [0.5; 3],
                outline_color: [1.0; 4],
                outline_thickness: 0.0,
            },
            RenderTier::High => Self {
                bloom_threshold: 0.8,
                bloom_intensity: 0.6,
                fog_density: 0.15,
                fog_color: [0.5; 3],
                outline_color: [1.0, 1.0, 1.0, 1.0],
                outline_thickness: 1.0,
            },
        }
    }
}

/// Framebuffer for post-processing pipeline.
pub struct PostFramebuffer {
    /// Color texture (intermediate pass).
    _color_texture: wgpu::Texture,
    color_view: TextureView,
    /// Bloom mip chain (for multi-pass blur).
    _bloom_textures: Vec<wgpu::Texture>,
    /// Depth texture (for outline edge detection).
    _depth_texture: wgpu::Texture,
    depth_view: TextureView,
}

impl PostFramebuffer {
    /// Create framebuffer for given size and tier.
    pub fn new(device: &Device, width: u32, height: u32, tier: RenderTier) -> Self {
        let format = wgpu::TextureFormat::Rgba16Float;

        // Main color texture
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post_color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Bloom mip chain (only if bloom enabled)
        let bloom_passes = tier.max_bloom_passes();
        let mut bloom_textures = Vec::new();

        if bloom_passes > 0 {
            for i in 0..bloom_passes {
                let mip_width = width >> i;
                let mip_height = height >> i;
                let mip_width = mip_width.max(1);
                let mip_height = mip_height.max(1);

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(&format!("bloom_mip_{}", i)),
                    size: wgpu::Extent3d {
                        width: mip_width,
                        height: mip_height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let _view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                bloom_textures.push(texture);
            }
        }

        // Depth texture (for outline)
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post_depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _color_texture: color_texture,
            color_view,
            _bloom_textures: bloom_textures,
            _depth_texture: depth_texture,
            depth_view,
        }
    }

    /// Resize framebuffer.
    pub fn resize(&mut self, device: &Device, width: u32, height: u32, tier: RenderTier) {
        *self = Self::new(device, width, height, tier);
    }

    /// Get color view for main scene render.
    pub fn color_view(&self) -> &TextureView {
        &self.color_view
    }

    /// Get depth view for outline.
    pub fn depth_view(&self) -> &TextureView {
        &self.depth_view
    }
}

/// Post-processing pipeline.
pub struct PostPipeline {
    config: PostConfig,
    tier: RenderTier,
    // Pipelines for each effect
    // (To be filled in with actual shader modules)
}

impl PostPipeline {
    /// Create new post-processing pipeline.
    pub fn new(_device: &Device, tier: RenderTier) -> Self {
        let config = PostConfig::for_tier(tier);

        Self { config, tier }
    }

    /// Apply post-processing to the framebuffer.
    ///
    /// Renders to the final output texture view.
    pub fn apply(
        &self,
        _encoder: &mut CommandEncoder,
        _framebuffer: &PostFramebuffer,
        _output_view: &TextureView,
    ) {
        // TODO: Implement actual post-processing passes
        // For now, this is a stub - we'll add the shader modules
        // and actual rendering in subsequent steps

        // Pass structure:
        // 1. Bloom downsample (if enabled)
        // 2. Bloom blur (if enabled)
        // 3. Bloom upsample + blend (if enabled)
        // 4. Fog pass (if enabled)
        // 5. Outline pass (if enabled)
        // 6. LUT color grading (if enabled)
        // 7. Final composite to output
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: PostConfig) {
        self.config = config;
    }

    /// Get current configuration.
    pub fn config(&self) -> &PostConfig {
        &self.config
    }

    /// Change render tier (rebuilds internal state).
    pub fn set_tier(&mut self, _device: &Device, tier: RenderTier) {
        self.tier = tier;
        self.config = PostConfig::for_tier(tier);
        // TODO: Rebuild pipelines for new tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = PostConfig::default();
        assert_eq!(config.bloom_threshold, 0.8);
        assert_eq!(config.bloom_intensity, 0.5);
        assert_eq!(config.fog_density, 0.0);
    }

    #[test]
    fn low_tier_config() {
        let config = PostConfig::for_tier(RenderTier::Low);
        assert_eq!(config.bloom_intensity, 0.0);
        assert_eq!(config.fog_density, 0.0);
        assert_eq!(config.outline_thickness, 0.0);
    }

    #[test]
    fn medium_tier_config() {
        let config = PostConfig::for_tier(RenderTier::Medium);
        assert!(config.bloom_intensity > 0.0);
        assert!(config.fog_density > 0.0);
        assert_eq!(config.outline_thickness, 0.0);
    }

    #[test]
    fn high_tier_config() {
        let config = PostConfig::for_tier(RenderTier::High);
        assert!(config.bloom_intensity > 0.0);
        assert!(config.fog_density > 0.0);
        assert!(config.outline_thickness > 0.0);
    }
}
