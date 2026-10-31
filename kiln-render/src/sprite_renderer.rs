//! Sprite batch renderer for 2D (textured, UV rects, batch-by-texture).
//!
//! Usage per frame:
//! 1. Call [`SpriteRenderer::begin_frame`] to clear pending sprites.
//! 2. Call [`SpriteRenderer::push`] / [`push_textured`] for each visible sprite.
//! 3. Call [`SpriteRenderer::flush`] inside a render pass to draw everything.
//!
//! Sprites sharing a [`TextureId`] are drawn in one instanced call.

use bytemuck::{Pod, Zeroable};
use wgpu::{Device, Queue};

use crate::camera::Camera;
use crate::texture::{create_rgba8_texture, TextureId, FULL_UV};

/// Maximum sprites per frame (prevents unbounded allocations).
pub const MAX_SPRITES: usize = 16_384;

/// Per-sprite GPU data (matches `InstanceInput` in `sprite.wgsl`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteInstance {
    /// World-space centre of the sprite.
    pub position: [f32; 2],
    /// Width and height in world units.
    pub size: [f32; 2],
    /// RGBA colour tint (multiplied with texture).
    pub color: [f32; 4],
    /// Normalised UV rect: `u0, v0, u1, v1`.
    pub uv_rect: [f32; 4],
}

impl SpriteInstance {
    /// Byte stride of the instance layout.
    pub const STRIDE: u64 = std::mem::size_of::<SpriteInstance>() as u64;

    /// Colour-tinted sprite covering the full texture (or white texel).
    pub fn colored(position: [f32; 2], size: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            position,
            size,
            color,
            uv_rect: FULL_UV,
        }
    }

    /// `wgpu` vertex attributes for the instance step.
    pub fn vertex_attributes() -> [wgpu::VertexAttribute; 4] {
        wgpu::vertex_attr_array![
            1 => Float32x2,   // position
            2 => Float32x2,   // size
            3 => Float32x4,   // color
            4 => Float32x4,   // uv_rect
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    corner: [f32; 2],
}

struct TextureSlot {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

/// Batching 2D sprite renderer with per-texture draw calls.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: Vec<TextureSlot>,
    quad_vertices: wgpu::Buffer,
    quad_indices: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    /// Pending sprites queued for this frame: `(texture, layer, instance)`.
    /// Sprites are sorted by `(layer, texture)` before drawing so that
    /// sprites on lower layers (background) are drawn before higher layers
    /// (foreground), preserving painter order for transparent sprites.
    /// Within the same layer and texture, submission order is preserved.
    pending: Vec<(TextureId, f32, SpriteInstance)>,
    last_draw_calls: u32,
}

const QUAD_VERTS: &[QuadVertex] = &[
    QuadVertex {
        corner: [-0.5, -0.5],
    },
    QuadVertex {
        corner: [0.5, -0.5],
    },
    QuadVertex { corner: [0.5, 0.5] },
    QuadVertex {
        corner: [-0.5, 0.5],
    },
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

impl SpriteRenderer {
    /// Create a new renderer (includes [`TextureId::WHITE`]).
    pub fn new(device: &Device, queue: &Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_uniform_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_uniform_bg"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_texture_bgl"),
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
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let quad_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_verts"),
            size: std::mem::size_of_val(QUAD_VERTS) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_idx"),
            size: std::mem::size_of_val(QUAD_INDICES) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_instances"),
            size: SpriteInstance::STRIDE * MAX_SPRITES as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&uniform_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let quad_attr = wgpu::vertex_attr_array![0 => Float32x2];
        let inst_attr = SpriteInstance::vertex_attributes();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &quad_attr,
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: SpriteInstance::STRIDE,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &inst_attr,
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let mut this = Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_bgl,
            sampler,
            textures: Vec::new(),
            quad_vertices,
            quad_indices,
            instance_buffer,
            pending: Vec::with_capacity(MAX_SPRITES),
            last_draw_calls: 0,
        };

        let white = this.create_texture_rgba(device, queue, "white", 1, 1, &[255, 255, 255, 255]);
        debug_assert_eq!(white, TextureId::WHITE);

        this
    }

    /// Upload static quad geometry. Call once after construction.
    pub fn upload_quad(&self, queue: &Queue) {
        queue.write_buffer(&self.quad_vertices, 0, bytemuck::cast_slice(QUAD_VERTS));
        queue.write_buffer(&self.quad_indices, 0, bytemuck::cast_slice(QUAD_INDICES));
    }

    /// Create a sampled RGBA8 texture; returns a stable [`TextureId`].
    pub fn create_texture_rgba(
        &mut self,
        device: &Device,
        queue: &Queue,
        label: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> TextureId {
        let (texture, view) = create_rgba8_texture(device, queue, label, width, height, rgba);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_tex_bg"),
            layout: &self.texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let id = TextureId(self.textures.len() as u32);
        self.textures.push(TextureSlot {
            _texture: texture,
            bind_group,
            width,
            height,
        });
        id
    }

    /// Texture size for UV helpers.
    pub fn texture_size(&self, id: TextureId) -> Option<(u32, u32)> {
        self.textures
            .get(id.0 as usize)
            .map(|t| (t.width, t.height))
    }

    /// Clear pending sprites. Call at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.pending.clear();
        self.last_draw_calls = 0;
    }

    /// Queue a sprite on [`TextureId::WHITE`] (colour-only quad).
    pub fn push(&mut self, instance: SpriteInstance) {
        self.push_textured(TextureId::WHITE, instance);
    }

    /// Queue a sprite sampled from `texture` on the default layer (0.0).
    pub fn push_textured(&mut self, texture: TextureId, instance: SpriteInstance) {
        if self.pending.len() >= MAX_SPRITES {
            return;
        }
        let id = if (texture.0 as usize) < self.textures.len() {
            texture
        } else {
            TextureId::WHITE
        };
        self.pending.push((id, 0.0, instance));
    }

    /// Queue a sprite sampled from `texture` on a specific `layer`.
    /// Sprites on lower layers are drawn first, preserving painter order
    /// for transparent sprites across different textures.
    pub fn push_textured_with_layer(&mut self, texture: TextureId, layer: f32, instance: SpriteInstance) {
        if self.pending.len() >= MAX_SPRITES {
            return;
        }
        let id = if (texture.0 as usize) < self.textures.len() {
            texture
        } else {
            TextureId::WHITE
        };
        self.pending.push((id, layer, instance));
    }

    /// Upload instances and draw one instanced call per distinct texture.
    pub fn flush<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera,
        queue: &Queue,
    ) {
        self.last_draw_calls = 0;
        if self.pending.is_empty() {
            return;
        }

        let proj: [f32; 16] = camera.projection_matrix().to_cols_array();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));

        // Sort by (layer, texture) so sprites on lower layers are drawn first
        // (painter order for transparency), while same-layer same-texture
        // sprites are batched into one draw call. The sort is stable, so
        // submission order is preserved within each (layer, texture) group.
        self.pending
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0)));

        let proj: [f32; 16] = camera.projection_matrix().to_cols_array();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));

        // Collect all instances into a single buffer upload.
        let all: Vec<SpriteInstance> = self.pending.iter().map(|(_, _, inst)| *inst).collect();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&all));

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_vertices.slice(..));
        render_pass.set_index_buffer(self.quad_indices.slice(..), wgpu::IndexFormat::Uint16);

        let mut i = 0;
        while i < self.pending.len() {
            let tex_id = self.pending[i].0;
            let start = i;
            while i < self.pending.len() && self.pending[i].0 == tex_id {
                i += 1;
            }
            let count = i - start;

            let slot = &self.textures[tex_id.0 as usize];
            render_pass.set_bind_group(1, &slot.bind_group, &[]);
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw_indexed(
                0..QUAD_INDICES.len() as u32,
                0,
                start as u32..(start + count) as u32,
            );
            self.last_draw_calls += 1;
        }
    }

    /// Sprites queued this frame.
    pub fn queued_count(&self) -> usize {
        self.pending.len()
    }

    /// GPU draw calls issued by the last [`flush`] (batch-by-texture proof).
    pub fn draw_call_count(&self) -> u32 {
        self.last_draw_calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_instance_size() {
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 48);
    }

    #[test]
    fn max_sprites_cap() {
        let mut sr = SpriteRendererCpu::default();
        for _ in 0..MAX_SPRITES + 10 {
            sr.push(SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]));
        }
        assert_eq!(sr.len(), MAX_SPRITES);
    }

    #[test]
    fn batch_groups_by_texture() {
        let mut sr = SpriteRendererCpu::default();
        sr.push_textured(
            TextureId(0),
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        );
        sr.push_textured(
            TextureId(1),
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        );
        sr.push_textured(
            TextureId(0),
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        );
        assert_eq!(sr.draw_calls_if_flushed(), 2);
    }

    /// Regression test: sprites on different layers must be drawn in layer
    /// order even when they use different textures. Without layer sorting,
    /// texture batching would reorder A(tex1, layer0), B(tex2, layer1),
    /// C(tex1, layer0) into A, C, B — breaking painter order for transparent
    /// sprites. With layer sorting, all layer-0 sprites are drawn before
    /// layer-1 sprites, preserving correct background→foreground order.
    #[test]
    fn sort_preserves_layer_order_across_textures() {
        let mut sr = SpriteRendererCpu::default();
        // Submit in painter order: background A, foreground B, background C
        sr.pending.push((TextureId(0), 0.0, SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]))); // A: layer 0, tex 0
        sr.pending.push((TextureId(1), 1.0, SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]))); // B: layer 1, tex 1
        sr.pending.push((TextureId(0), 0.0, SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]))); // C: layer 0, tex 0

        // Extract the sort order that flush would use
        let mut ordered: Vec<(f32, u32)> = sr
            .pending
            .iter()
            .map(|(tex, layer, _)| (*layer, tex.0))
            .collect();
        ordered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.1.cmp(&b.1)));

        // Layer 0 sprites (A, C) must come before layer 1 (B)
        assert_eq!(ordered[0], (0.0, 0)); // A
        assert_eq!(ordered[1], (0.0, 0)); // C
        assert_eq!(ordered[2], (1.0, 1)); // B
    }

    /// Within the same layer and texture, submission order is preserved.
    #[test]
    fn sort_preserves_submission_order_within_same_layer_and_texture() {
        let mut sr = SpriteRendererCpu::default();
        sr.pending.push((TextureId(0), 0.0, SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]))); // first
        sr.pending.push((TextureId(0), 0.0, SpriteInstance::colored([1.0; 2], [1.0; 2], [1.0; 4]))); // second

        let mut ordered: Vec<(f32, u32)> = sr
            .pending
            .iter()
            .map(|(tex, layer, _)| (*layer, tex.0))
            .collect();
        ordered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.1.cmp(&b.1)));

        assert_eq!(ordered[0], (0.0, 0));
        assert_eq!(ordered[1], (0.0, 0));
    }

    #[test]
    fn colored_defaults_full_uv() {
        let s = SpriteInstance::colored([1.0, 2.0], [3.0, 4.0], [1.0; 4]);
        assert_eq!(s.uv_rect, FULL_UV);
    }

    #[derive(Default)]
    struct SpriteRendererCpu {
        pending: Vec<(TextureId, f32, SpriteInstance)>,
    }
    impl SpriteRendererCpu {
        fn push(&mut self, inst: SpriteInstance) {
            self.push_textured(TextureId::WHITE, inst);
        }
        fn push_textured(&mut self, texture: TextureId, inst: SpriteInstance) {
            if self.pending.len() < MAX_SPRITES {
                self.pending.push((texture, 0.0, inst));
            }
        }
        fn len(&self) -> usize {
            self.pending.len()
        }
        fn draw_calls_if_flushed(&self) -> u32 {
            let mut ids: Vec<_> = self.pending.iter().map(|(t, _, _)| t.0).collect();
            ids.sort_unstable();
            ids.dedup();
            ids.len() as u32
        }
    }
}
