//! Post-processing pipeline for 2D rendering.
//!
//! Supports:
//! - Bloom (downsample, blur, threshold, upsample, blend)
//! - LUT color grading
//! - Screen-space fog
//! - Outline effect

use wgpu::{Device, TextureView};

use crate::tier::RenderTier;

/// Monotonic counter used to invalidate cached GPU state on reallocation.
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Next unique framebuffer generation.
fn next_generation() -> u64 {
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

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
    /// Outline color (RGB + alpha as strength).
    pub outline_color: [f32; 4],
    /// Outline thickness (edge detection strength in pixels).
    pub outline_thickness: f32,
    /// Luminance delta required before an edge is outlined.
    pub outline_threshold: f32,
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
            outline_threshold: 0.1,
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
                outline_threshold: 0.1,
            },
            RenderTier::Medium => Self {
                bloom_threshold: 0.8,
                bloom_intensity: 0.4,
                fog_density: 0.1,
                fog_color: [0.5; 3],
                outline_color: [1.0; 4],
                outline_thickness: 0.0,
                outline_threshold: 0.1,
            },
            RenderTier::High => Self {
                bloom_threshold: 0.8,
                bloom_intensity: 0.6,
                fog_density: 0.15,
                fog_color: [0.5; 3],
                outline_color: [1.0, 1.0, 1.0, 1.0],
                outline_thickness: 1.0,
                outline_threshold: 0.1,
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
    /// Bloom chain target (buffer 0) and blur ping-pong buffer (buffer 1).
    ///
    /// Empty on tiers without bloom.
    bloom_views: Vec<TextureView>,
    /// Bumped whenever the framebuffer is reallocated, so cached GPU state
    /// (e.g. bind groups) can be invalidated.
    generation: u64,
    /// Framebuffer width in pixels.
    width: u32,
    /// Framebuffer height in pixels.
    height: u32,
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
        let mut bloom_views = Vec::new();

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
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                bloom_views.push(view);
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
            bloom_views,
            generation: next_generation(),
            width,
            height,
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

    /// Views into the bloom chain (empty on tiers without bloom).
    pub fn bloom_views(&self) -> &[TextureView] {
        &self.bloom_views
    }

    /// Unique id of this framebuffer allocation.
    ///
    /// Changes whenever the framebuffer is reallocated (e.g. on resize), so
    /// consumers can invalidate cached GPU state such as bind groups.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Framebuffer width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Framebuffer height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Post-processing pipeline (GPU implementation).
///
/// The pipeline owns the bloom / outline / fog passes described in
/// [`crate::post_gpu`]. Use [`PostPipeline::apply`] after rendering the scene
/// into the [`PostFramebuffer`] colour target.
pub use crate::post_gpu::PostGpuPipeline as PostPipeline;

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
