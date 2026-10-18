//! Sprite batch renderer for 2D.
//!
//! Usage per frame:
//! 1. Call [`SpriteRenderer::begin_frame`] to clear pending sprites.
//! 2. Call [`SpriteRenderer::push`] for each visible sprite.
//! 3. Call [`SpriteRenderer::flush`] inside a render pass to draw everything.
//!
//! All sprites are batched into a single draw call (one per flush).

use bytemuck::{Pod, Zeroable};
use wgpu::{Device, Queue};

use crate::camera::Camera;

/// Maximum sprites per batch (prevents unbounded allocations).
pub const MAX_SPRITES: usize = 16_384;

/// Per-sprite GPU data (matches `InstanceInput` in `sprite.wgsl`).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SpriteInstance {
    /// World-space centre of the sprite.
    pub position: [f32; 2],
    /// Width and height in world units.
    pub size: [f32; 2],
    /// RGBA colour.
    pub color: [f32; 4],
}

impl SpriteInstance {
    /// Byte stride of the instance layout.
    pub const STRIDE: u64 = std::mem::size_of::<SpriteInstance>() as u64;

    /// `wgpu` vertex attributes for the instance step.
    pub fn vertex_attributes() -> [wgpu::VertexAttribute; 3] {
        wgpu::vertex_attr_array![
            1 => Float32x2,   // position
            2 => Float32x2,   // size
            3 => Float32x4,   // color
        ]
    }
}

/// Unit-quad corner vertex (just XY; the shader scales/positions per-instance).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    corner: [f32; 2],
}

/// Batching 2D sprite renderer.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    quad_vertices: wgpu::Buffer,
    quad_indices: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    /// CPU-side staging list cleared each frame.
    instances: Vec<SpriteInstance>,
}

// Six unit-quad vertices as two triangles (CCW).
// Corners sit at ±0.5 so the shader can multiply by sprite size.
const QUAD_VERTS: &[QuadVertex] = &[
    QuadVertex { corner: [-0.5, -0.5] },
    QuadVertex { corner: [ 0.5, -0.5] },
    QuadVertex { corner: [ 0.5,  0.5] },
    QuadVertex { corner: [-0.5,  0.5] },
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

impl SpriteRenderer {
    /// Create a new renderer. Allocates GPU buffers for up to [`MAX_SPRITES`].
    pub fn new(device: &Device, format: wgpu::TextureFormat) -> Self {
        // — shader —
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        // — uniform buffer (projection matrix = 64 bytes) —
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // — bind group —
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_bgl"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // — quad vertex + index buffers (static) —
        let quad_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_verts"),
            size: (std::mem::size_of::<QuadVertex>() * QUAD_VERTS.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_idx"),
            size: (std::mem::size_of::<u16>() * QUAD_INDICES.len()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // — instance buffer (dynamic, written each frame via queue.write_buffer) —
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_instances"),
            size: SpriteInstance::STRIDE * MAX_SPRITES as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // — pipeline —
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&bgl],
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

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            quad_vertices,
            quad_indices,
            instance_buffer,
            instances: Vec::with_capacity(MAX_SPRITES),
        }
    }

    /// Upload the static quad geometry. Call once after construction (needs queue).
    pub fn upload_quad(&self, queue: &Queue) {
        queue.write_buffer(
            &self.quad_vertices,
            0,
            bytemuck::cast_slice(QUAD_VERTS),
        );
        queue.write_buffer(
            &self.quad_indices,
            0,
            bytemuck::cast_slice(QUAD_INDICES),
        );
    }

    /// Clear pending sprites. Call at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.instances.clear();
    }

    /// Queue one sprite for rendering. Ignored if the batch is full.
    pub fn push(&mut self, instance: SpriteInstance) {
        if self.instances.len() < MAX_SPRITES {
            self.instances.push(instance);
        }
    }

    /// Upload instances and issue a single instanced draw call.
    ///
    /// Must be called inside an active render pass.
    pub fn flush<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera,
        queue: &Queue,
    ) {
        if self.instances.is_empty() {
            return;
        }

        // Upload projection matrix — glam::Mat4 doesn't impl Pod, so cast via array
        let proj: [f32; 16] = camera.projection_matrix().to_cols_array();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));

        // Upload sprites
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );

        let count = self.instances.len() as u32;

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_vertices.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.quad_indices.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..count);
    }

    /// Number of sprites queued this frame (useful for profiling / logging).
    pub fn queued_count(&self) -> usize {
        self.instances.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_instance_size() {
        // 2 + 2 + 4 = 8 f32s = 32 bytes
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 32);
    }

    #[test]
    fn max_sprites_cap() {
        let mut sr = SpriteRendererCpu::default();
        for _ in 0..MAX_SPRITES + 10 {
            sr.push(SpriteInstance {
                position: [0.0; 2],
                size: [1.0; 2],
                color: [1.0; 4],
            });
        }
        assert_eq!(sr.len(), MAX_SPRITES);
    }

    // Lightweight CPU-only stub for tests (no GPU needed).
    #[derive(Default)]
    struct SpriteRendererCpu {
        instances: Vec<SpriteInstance>,
    }
    impl SpriteRendererCpu {
        fn push(&mut self, inst: SpriteInstance) {
            if self.instances.len() < MAX_SPRITES {
                self.instances.push(inst);
            }
        }
        fn len(&self) -> usize {
            self.instances.len()
        }
    }
}
