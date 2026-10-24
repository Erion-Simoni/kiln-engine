use wgpu::{Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration};

use crate::pass_config::PassEnableConfig;
use crate::post::{PostFramebuffer, PostPipeline};
use crate::profiler::{requested_profiler_features, GpuProfiler, PassTiming};
use crate::tier::RenderTier;

/// Renderer with tier support, post-processing, and optional GPU profiling.
pub struct Renderer {
    /// GPU adapter (kept alive for the device's lifetime; `None` on web where
    /// the adapter is not needed after device creation).
    _adapter: Option<Adapter>,
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
            Some(PostPipeline::new(&device, tier, surface_config.format))
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
            _adapter: Some(adapter),
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
        if self.surface_config.width == width && self.surface_config.height == height {
            return;
        }

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

    /// The format the scene must be rendered into.
    ///
    /// With post-processing enabled this is the intermediate `Rgba16Float`
    /// target of the post framebuffer; otherwise it matches the swap-chain
    /// format. Pipelines that render the scene must use this format.
    pub fn scene_format(&self) -> wgpu::TextureFormat {
        match self.post_framebuffer.as_ref() {
            Some(_) => wgpu::TextureFormat::Rgba16Float,
            None => self.surface_config.format,
        }
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
            self.post_pipeline = Some(PostPipeline::new(
                &self.device,
                tier,
                self.surface_config.format,
            ));
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

    pub fn post_pipeline_mut(&mut self) -> Option<&mut PostPipeline> {
        self.post_pipeline.as_mut()
    }

    pub fn post_framebuffer(&self) -> Option<&PostFramebuffer> {
        self.post_framebuffer.as_ref()
    }

    pub fn post_framebuffer_mut(&mut self) -> Option<&mut PostFramebuffer> {
        self.post_framebuffer.as_mut()
    }

    /// Whether post-processing is active for the current tier.
    pub fn has_post(&self) -> bool {
        self.post_pipeline.is_some() && self.post_framebuffer.is_some()
    }

    /// The texture view the scene must be rendered into.
    ///
    /// With post-processing enabled this is the intermediate `Rgba16Float`
    /// target of the post framebuffer; without it, `None` is returned and the
    /// caller should render straight to the swap-chain view.
    pub fn scene_target(&self) -> Option<&wgpu::TextureView> {
        self.post_framebuffer.as_ref().map(|fb| fb.color_view())
    }

    /// Run the post-processing chain: reads the scene from the post framebuffer
    /// and writes the final image to `output_view`.
    ///
    /// Does nothing when the current tier has post-processing disabled.
    pub fn apply_post(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        let Some(pipeline) = self.post_pipeline.as_mut() else {
            return;
        };
        let Some(framebuffer) = self.post_framebuffer.as_ref() else {
            return;
        };
        pipeline.apply(&self.device, &self.queue, encoder, framebuffer, output_view);
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

    /// Return the current surface configuration (useful for web where the
    /// surface is created externally).
    pub fn surface_config(&self) -> SurfaceConfiguration {
        self.surface_config.clone()
    }

    /// Construct a `Renderer` from an existing device, queue, and surface
    /// format. Used by the web frontend where the `wgpu::Surface` is created
    /// from an HTML canvas element outside this crate.
    pub async fn new_from_parts(
        device: Device,
        queue: Queue,
        surface_format: wgpu::TextureFormat,
        tier: RenderTier,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let post_pipeline = if tier != RenderTier::Low {
            Some(PostPipeline::new(&device, tier, surface_format))
        } else {
            None
        };

        let post_framebuffer = if tier != RenderTier::Low {
            Some(PostFramebuffer::new(&device, width, height, tier))
        } else {
            None
        };

        let profiler = GpuProfiler::new(&device, &queue, 32);

        Ok(Self {
            _adapter: None,
            device,
            queue,
            surface_config,
            tier,
            post_pipeline,
            post_framebuffer,
            profiler,
            pass_config: PassEnableConfig::default_2d(),
        })
    }
}
