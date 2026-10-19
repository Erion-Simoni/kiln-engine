//! GPU implementation of the post-processing pipeline (bloom, outline, fog).
//!
//! Pass structure:
//! 1. Bright extract: scene -> bloom mip 0.
//! 2. Separable blur: bloom mip 0 -> mip 1 (horizontal), mip 1 -> mip 0 (vertical).
//! 3. Composite: scene + bloom -> output, with outline and fog applied.
//!
//! Every pass uses a fullscreen triangle (3 vertices, no vertex buffer) and
//! samples with a linear filter.

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue, RenderPipeline, Sampler,
    TextureView,
};

use crate::post::{PostConfig, PostFramebuffer};
use crate::tier::RenderTier;

/// Uniform data for post passes (matches `PostUniforms` in `post.wgsl`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PostUniforms {
    /// Luminance above which a pixel contributes to bloom.
    pub bloom_threshold: f32,
    /// Strength of the bloom added during composite.
    pub bloom_intensity: f32,
    /// Fog blend amount (0 = clear).
    pub fog_density: f32,
    /// Luminance delta that counts as an edge.
    pub outline_threshold: f32,
    /// Blur direction in texel space.
    pub blur_dir: [f32; 2],
    /// Padding so the uniform stays 16-byte aligned.
    pub _pad: [f32; 2],
    /// Fog colour (RGB) plus unused alpha.
    pub fog_color: [f32; 4],
    /// Outline colour (RGB) plus strength in alpha.
    pub outline_color: [f32; 4],
}

impl PostUniforms {
    /// Build the uniform block for `config`.
    pub fn from_config(config: &PostConfig) -> Self {
        Self {
            bloom_threshold: config.bloom_threshold,
            bloom_intensity: config.bloom_intensity,
            fog_density: config.fog_density,
            outline_threshold: config.outline_threshold,
            blur_dir: [1.0, 0.0],
            _pad: [0.0; 2],
            fog_color: [
                config.fog_color[0],
                config.fog_color[1],
                config.fog_color[2],
                0.0,
            ],
            outline_color: [
                config.outline_color[0],
                config.outline_color[1],
                config.outline_color[2],
                config.outline_color[3] * config.outline_thickness,
            ],
        }
    }
}

/// Bind groups bound to the textures of one framebuffer.
struct PostBindGroups {
    /// Bright extract: reads the scene, writes bloom mip 0.
    bright: BindGroup,
    /// Horizontal blur: reads bloom mip 0, writes bloom mip 1.
    blur_h: BindGroup,
    /// Vertical blur: reads bloom mip 1, writes bloom mip 0.
    blur_v: BindGroup,
    /// Composite: reads the scene and the blurred bloom.
    composite: BindGroup,
}

/// GPU post-processing pipeline.
pub struct PostGpuPipeline {
    config: PostConfig,
    tier: RenderTier,
    layout: BindGroupLayout,
    sampler: Sampler,
    bright_pipeline: RenderPipeline,
    blur_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,
    bright_uniform: Buffer,
    blur_h_uniform: Buffer,
    blur_v_uniform: Buffer,
    composite_uniform: Buffer,
    bind_groups: Option<PostBindGroups>,
    /// Framebuffer generation the cached bind groups were built for.
    bind_generation: Option<u64>,
}

impl PostGpuPipeline {
    /// Create the pipeline for `tier`, targeting `output_format` for the composite pass.
    pub fn new(device: &Device, tier: RenderTier, output_format: wgpu::TextureFormat) -> Self {
        let config = PostConfig::for_tier(tier);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post_pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let bright_pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            "post_bright",
            "fs_bright",
            wgpu::TextureFormat::Rgba16Float,
        );
        let blur_pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            "post_blur",
            "fs_blur",
            wgpu::TextureFormat::Rgba16Float,
        );
        let composite_pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            "post_composite",
            "fs_composite",
            output_format,
        );

        let mk_uniform = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<PostUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        Self {
            config,
            tier,
            layout,
            sampler,
            bright_pipeline,
            blur_pipeline,
            composite_pipeline,
            bright_uniform: mk_uniform("post_bright_uniform"),
            blur_h_uniform: mk_uniform("post_blur_h_uniform"),
            blur_v_uniform: mk_uniform("post_blur_v_uniform"),
            composite_uniform: mk_uniform("post_composite_uniform"),
            bind_groups: None,
            bind_generation: None,
        }
    }

    /// Current configuration.
    pub fn config(&self) -> &PostConfig {
        &self.config
    }

    /// Replace the configuration.
    pub fn set_config(&mut self, config: PostConfig) {
        self.config = config;
    }

    /// Tier this pipeline was built for.
    pub fn tier(&self) -> RenderTier {
        self.tier
    }

    /// Whether any post effect is active for this pipeline.
    ///
    /// The composite pass always runs (it copies the scene to the output);
    /// this reports whether any *effect* changes the image.
    pub fn is_active(&self) -> bool {
        if self.tier == RenderTier::Low {
            return false;
        }
        self.bloom_enabled() || self.fog_enabled() || self.outline_enabled()
    }

    /// Whether the bloom chain should run.
    fn bloom_enabled(&self) -> bool {
        self.config.bloom_intensity > 0.0
    }

    /// Whether fog blending should run.
    fn fog_enabled(&self) -> bool {
        self.config.fog_density > 0.0
    }

    /// Whether outline detection should run.
    fn outline_enabled(&self) -> bool {
        self.config.outline_thickness > 0.0 && self.config.outline_color[3] > 0.0
    }

    /// Apply post-processing: reads the scene from `framebuffer` and writes to `output_view`.
    ///
    /// The scene must already be rendered into [`PostFramebuffer::color_view`].
    pub fn apply(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        framebuffer: &PostFramebuffer,
        output_view: &TextureView,
    ) {
        let bloom_views = framebuffer.bloom_views();
        // Fall back to the scene view so the composite bind group stays valid
        // even when the tier has no bloom buffers.
        let bloom_tex: &TextureView = bloom_views
            .first()
            .unwrap_or_else(|| framebuffer.color_view());

        self.ensure_bind_groups(device, framebuffer, bloom_tex);
        self.write_uniforms(queue);

        // Disjoint field borrows so the bind groups and pipelines can be used
        // at the same time.
        let groups = self.bind_groups.as_ref().expect("post bind groups");
        let bright_pipeline = &self.bright_pipeline;
        let blur_pipeline = &self.blur_pipeline;
        let composite_pipeline = &self.composite_pipeline;

        // 1. Bright extract: scene -> bloom mip 0.
        if self.bloom_enabled() {
            if let Some(mip0) = bloom_views.first() {
                fullscreen_pass(
                    encoder,
                    bright_pipeline,
                    &groups.bright,
                    mip0,
                    "post_bright",
                );

                // 2. Separable blur, ping-ponging between mip 0 and mip 1.
                if let Some(mip1) = bloom_views.get(1) {
                    fullscreen_pass(encoder, blur_pipeline, &groups.blur_h, mip1, "post_blur_h");
                    fullscreen_pass(encoder, blur_pipeline, &groups.blur_v, mip0, "post_blur_v");
                }
            }
        }

        // 3. Composite: scene + bloom -> output.
        fullscreen_pass(
            encoder,
            composite_pipeline,
            &groups.composite,
            output_view,
            "post_composite",
        );
    }

    fn write_uniforms(&self, queue: &Queue) {
        let base = PostUniforms::from_config(&self.config);
        queue.write_buffer(&self.bright_uniform, 0, bytemuck::bytes_of(&base));

        let mut blur_h = base;
        blur_h.blur_dir = [1.0, 0.0];
        queue.write_buffer(&self.blur_h_uniform, 0, bytemuck::bytes_of(&blur_h));

        let mut blur_v = base;
        blur_v.blur_dir = [0.0, 1.0];
        queue.write_buffer(&self.blur_v_uniform, 0, bytemuck::bytes_of(&blur_v));

        queue.write_buffer(&self.composite_uniform, 0, bytemuck::bytes_of(&base));
    }

    /// Rebuild the bind groups for the current framebuffer.
    ///
    /// The `bloom` slot of each bind group is filled with a texture that is
    /// *not* the target of the corresponding pass, because wgpu forbids using a
    /// texture as both a render attachment and a bound texture in one pass.
    ///
    /// Bind groups are cached and only rebuilt when the framebuffer is
    /// reallocated (tracked by [`PostFramebuffer::generation`]).
    fn ensure_bind_groups(
        &mut self,
        device: &Device,
        framebuffer: &PostFramebuffer,
        bloom: &TextureView,
    ) {
        if self.bind_generation == Some(framebuffer.generation()) && self.bind_groups.is_some() {
            return;
        }

        let scene = framebuffer.color_view();
        let bloom1 = framebuffer.bloom_views().get(1).unwrap_or(bloom);

        self.bind_groups = Some(PostBindGroups {
            // Bright pass writes bloom mip 0: bind the scene for both slots.
            bright: self.make_bind_group(device, scene, scene, &self.bright_uniform),
            // Horizontal blur writes bloom mip 1: bind bloom mip 0 as source.
            blur_h: self.make_bind_group(device, bloom, bloom, &self.blur_h_uniform),
            // Vertical blur writes bloom mip 0: bind bloom mip 1 as source.
            blur_v: self.make_bind_group(device, bloom1, bloom1, &self.blur_v_uniform),
            // Composite reads the scene and bloom mip 0, writes the output.
            composite: self.make_bind_group(device, scene, bloom, &self.composite_uniform),
        });
        self.bind_generation = Some(framebuffer.generation());
    }

    fn make_bind_group(
        &self,
        device: &Device,
        src: &TextureView,
        bloom: &TextureView,
        uniform: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post_bind_group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(bloom),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(uniform.as_entire_buffer_binding()),
                },
            ],
        })
    }
}

/// Render one fullscreen-triangle pass into `target`.
fn fullscreen_pass(
    encoder: &mut CommandEncoder,
    pipeline: &RenderPipeline,
    bind_group: &BindGroup,
    target: &TextureView,
    label: &str,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}

/// Build a fullscreen-triangle render pipeline for one fragment entry point.
fn make_pipeline(
    device: &Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &str,
    entry_point: &str,
    format: wgpu::TextureFormat,
) -> RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point,
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_are_16_byte_aligned() {
        // The WGSL struct has two vec4-sized members plus 8 scalars, so the
        // total size must be a multiple of 16 bytes for the uniform buffer.
        let size = std::mem::size_of::<PostUniforms>();
        assert_eq!(size % 16, 0, "PostUniforms must be 16-byte aligned");
    }

    #[test]
    fn uniforms_match_config() {
        let config = PostConfig {
            bloom_threshold: 0.25,
            bloom_intensity: 0.75,
            fog_density: 0.5,
            fog_color: [0.1, 0.2, 0.3],
            outline_color: [1.0, 0.0, 0.0, 0.5],
            outline_thickness: 2.0,
            outline_threshold: 0.4,
        };
        let u = PostUniforms::from_config(&config);
        assert_eq!(u.bloom_threshold, 0.25);
        assert_eq!(u.bloom_intensity, 0.75);
        assert_eq!(u.fog_density, 0.5);
        assert_eq!(u.outline_threshold, 0.4);
        assert_eq!(u.fog_color, [0.1, 0.2, 0.3, 0.0]);
        // Outline alpha carries the strength (thickness).
        assert_eq!(u.outline_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn low_tier_config_disables_effects() {
        let config = PostConfig::for_tier(RenderTier::Low);
        assert_eq!(config.bloom_intensity, 0.0);
        assert_eq!(config.fog_density, 0.0);
        assert_eq!(config.outline_thickness, 0.0);
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;

    /// Render the full post chain into an offscreen target and read it back.
    ///
    /// This validates that the WGSL shader compiles and that the pass chain
    /// produces a lit output pixel on real hardware.
    #[test]
    fn post_chain_renders_on_gpu() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let Some(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
        else {
            // No adapter available in this environment: nothing to validate.
            eprintln!("skipping post GPU test: no adapter");
            return;
        };

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("post_test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("device");

        let width = 32u32;
        let height = 32u32;

        let framebuffer = PostFramebuffer::new(&device, width, height, RenderTier::High);

        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post_test_output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());

        let mut pipeline =
            PostGpuPipeline::new(&device, RenderTier::High, wgpu::TextureFormat::Rgba8Unorm);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("post_test_enc"),
        });

        // Bright-white scene so the bloom threshold passes.
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post_test_scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: framebuffer.color_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            drop(pass);
        }

        pipeline.apply(&device, &queue, &mut encoder, &framebuffer, &output_view);

        // Read the output back.
        let bytes_per_row = width * 4;
        let padded = bytes_per_row.div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post_test_readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &output,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().expect("readback").expect("map ok");

        let data = readback.slice(..).get_mapped_range();
        // Middle pixel of the first row.
        let px = &data[16..20];
        let lit = px[0] > 0 || px[1] > 0 || px[2] > 0;
        assert!(lit, "expected a lit output pixel, got {px:?}");
        drop(data);
        readback.unmap();
    }
}
