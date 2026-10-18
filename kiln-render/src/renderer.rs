use wgpu::{Instance, Adapter, Device, Queue, Surface, SurfaceConfiguration};

use crate::tier::RenderTier;
use crate::post::{PostPipeline, PostFramebuffer};

/// Renderer with tier support and post-processing.
pub struct Renderer {
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    surface_config: SurfaceConfiguration,
    tier: RenderTier,
    post_pipeline: Option<PostPipeline>,
    post_framebuffer: Option<PostFramebuffer>,
}

impl Renderer {
    pub async fn new(instance: &Instance, surface: &Surface<'_>, width: u32, height: u32) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find GPU adapter");

        let tier = RenderTier::auto_select(&adapter);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Kiln Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .first()
            .expect("No surface formats available");

        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // Initialize post-processing if not Low tier
        let post_pipeline = if tier != RenderTier::Low {
            Some(PostPipeline::new(&device, tier))
        } else {
            None
        };

        let post_framebuffer = if tier != RenderTier::Low {
            Some(PostFramebuffer::new(&device, width, height, tier))
        } else {
            None
        };

        Self {
            _adapter: adapter,
            device,
            queue,
            surface_config,
            tier,
            post_pipeline,
            post_framebuffer,
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn resize(&mut self, surface: &Surface<'_>, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        surface.configure(&self.device, &self.surface_config);

        // Resize post framebuffer if enabled
        if let Some(ref mut framebuffer) = self.post_framebuffer {
            framebuffer.resize(&self.device, width, height, self.tier);
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// Get current render tier.
    pub fn tier(&self) -> RenderTier {
        self.tier
    }

    /// Force a specific render tier (rebuilds post-processing if needed).
    pub fn set_tier(&mut self, tier: RenderTier) {
        if self.tier == tier {
            return;
        }

        self.tier = tier;

        if tier == RenderTier::Low {
            self.post_pipeline = None;
            self.post_framebuffer = None;
        } else {
            self.post_pipeline = Some(PostPipeline::new(&self.device, tier));
            self.post_framebuffer = Some(PostFramebuffer::new(
                &self.device,
                self.surface_config.width,
                self.surface_config.height,
                tier,
            ));
        }
    }

    /// Get post-processing pipeline (if enabled).
    pub fn post_pipeline(&self) -> Option<&PostPipeline> {
        self.post_pipeline.as_ref()
    }

    /// Get post-processing framebuffer (if enabled).
    pub fn post_framebuffer(&self) -> Option<&PostFramebuffer> {
        self.post_framebuffer.as_ref()
    }
}
