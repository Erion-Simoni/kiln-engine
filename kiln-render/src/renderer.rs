use wgpu::{Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration};

use crate::pass_config::PassEnableConfig;
use crate::post::{PostFramebuffer, PostPipeline};
use crate::profiler::{requested_profiler_features, GpuProfiler, PassTiming};
use crate::tier::RenderTier;

/// Renderer with tier support, post-processing, and optional GPU profiling.
pub struct Renderer {
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    surface_config: SurfaceConfiguration,
    tier: RenderTier,
    post_pipeline: Option<PostPipeline>,
    post_framebuffer: Option<PostFramebuffer>,
    profiler: GpuProfiler,
    pass_config: PassEnableConfig,
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
        let required_features = requested_profiler_features(&adapter);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Kiln Device"),
                    required_features,
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

        let profiler = GpuProfiler::new(&device, &queue, 32);

        Self {
            _adapter: adapter,
            device,
            queue,
            surface_config,
            tier,
            post_pipeline,
            post_framebuffer,
            profiler,
            pass_config: PassEnableConfig::default_2d(),
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

        if let Some(ref mut framebuffer) = self.post_framebuffer {
            framebuffer.resize(&self.device, width, height, self.tier);
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
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

    pub fn post_pipeline(&self) -> Option<&PostPipeline> {
        self.post_pipeline.as_ref()
    }

    pub fn post_framebuffer(&self) -> Option<&PostFramebuffer> {
        self.post_framebuffer.as_ref()
    }

    pub fn profiler(&self) -> &GpuProfiler {
        &self.profiler
    }

    pub fn profiler_mut(&mut self) -> &mut GpuProfiler {
        &mut self.profiler
    }

    /// Last resolved per-pass GPU timings (may be one frame behind).
    pub fn pass_timings(&self) -> &[PassTiming] {
        self.profiler.timings()
    }

    pub fn pass_config(&self) -> &PassEnableConfig {
        &self.pass_config
    }

    pub fn set_pass_config(&mut self, config: PassEnableConfig) {
        self.pass_config = config;
    }

    /// Resolve + poll GPU timings after `queue.submit`.
    pub fn finish_profiler_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.profiler.resolve(encoder);
    }

    pub fn poll_profiler(&mut self) {
        self.profiler.poll_timings(&self.device);
    }
}
