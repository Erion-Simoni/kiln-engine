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
use crate::texture::{create_rgba8_texture_with_options, TextureId, FULL_UV};

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
    /// Optional material ID for custom shader rendering (0 = default shader).
    pub material_id: u32,
    /// Rotation in radians, applied around the sprite centre.
    pub rotation: f32,
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
            material_id: 0,
            rotation: 0.0,
        }
    }

    /// Apply an entity transform to this sprite's local size and rotation.
    pub fn with_transform(mut self, rotation: f32, scale: [f32; 2]) -> Self {
        self.rotation = rotation;
        self.size[0] *= scale[0];
        self.size[1] *= scale[1];
        self
    }

    /// `wgpu` vertex attributes for the instance step.
    pub fn vertex_attributes() -> &'static [wgpu::VertexAttribute; 6] {
        static ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            1 => Float32x2,   // position
            2 => Float32x2,   // size
            3 => Float32x4,   // color
            4 => Float32x4,   // uv_rect
            5 => Uint32,      // material_id
            6 => Float32,     // rotation
        ];
        &ATTRIBUTES
    }

    /// `wgpu` vertex buffer layout for instanced rendering.
    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: SpriteInstance::STRIDE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::vertex_attributes(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    corner: [f32; 2],
}

struct MaterialPipeline {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
}

struct TextureSlot {
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

/// Batching 2D sprite renderer with per-texture draw calls and custom material support.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,
    textures: Vec<TextureSlot>,
    uniform_bgl: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    material_pipelines: std::collections::HashMap<u32, MaterialPipeline>,
    material_names: std::collections::HashMap<String, u32>,
    material_key: Vec<u8>,
    material_error: Option<String>,
    quad_vertices: wgpu::Buffer,
    quad_indices: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    /// Pending sprites in transparent painter order. Only adjacent matching
    /// texture/material runs batch; material changes never reorder sprites.
    pending: Vec<(TextureId, u32, f32, SpriteInstance)>,
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
    /// Drop project bindings when a host rebuilds all affected sprites.
    pub fn clear_project_textures(&mut self) {
        self.material_key.clear();
        self.material_names.clear();
        self.material_pipelines.clear();
        self.textures.truncate(1);
    }
    /// Create a new renderer (includes [`TextureId::WHITE`]).
    pub fn new(device: &Device, queue: &Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    crate::materials::VERTEX,
                    crate::materials::DEFAULT_FRAGMENT
                )
                .into(),
            ),
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
                        attributes: inst_attr,
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
            textures: Vec::new(),
            uniform_bgl,
            format,
            material_names: Default::default(),
            material_key: Vec::new(),
            material_error: None,
            material_pipelines: std::collections::HashMap::new(),
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
        self.create_texture_rgba_with_options(
            device,
            queue,
            label,
            [width, height],
            rgba,
            [true, false],
        )
    }

    /// Per-asset nearest/linear sampling and generated mipmaps.
    pub fn create_texture_rgba_with_options(
        &mut self,
        device: &Device,
        queue: &Queue,
        label: &str,
        size: [u32; 2],
        rgba: &[u8],
        options: [bool; 2],
    ) -> TextureId {
        let [width, height] = size;
        let (texture, view) =
            create_rgba8_texture_with_options(device, queue, label, size, rgba, options[1]);
        let filter = if options[0] {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("asset_sampler"),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: filter,
            ..Default::default()
        });
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
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let id = TextureId(self.textures.len() as u32);
        self.textures.push(TextureSlot {
            _texture: texture,
            view,
            sampler,
            bind_group,
            width,
            height,
        });
        id
    }

    /// Release asset textures, retaining the built-in white texel. All other
    /// texture IDs are invalidated; call only when replacing the owning resource library.
    pub fn clear_asset_textures(&mut self) {
        self.pending.clear();
        self.material_key.clear();
        self.material_names.clear();
        self.material_pipelines.clear();
        self.textures.truncate(1);
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
        self.pending.push((id, instance.material_id, 0.0, instance));
    }

    /// Queue a sprite sampled from `texture` on a specific `layer`.
    /// Uses the sprite's `material_id` field for material selection.
    pub fn push_textured_with_layer(
        &mut self,
        texture: TextureId,
        layer: f32,
        instance: SpriteInstance,
    ) {
        if self.pending.len() >= MAX_SPRITES {
            return;
        }
        let id = if (texture.0 as usize) < self.textures.len() {
            texture
        } else {
            TextureId::WHITE
        };
        self.pending
            .push((id, instance.material_id, layer, instance));
    }

    /// Queue a sprite sampled from `texture` on a specific `layer` with an explicit material ID.
    pub fn push_textured_with_material(
        &mut self,
        texture: TextureId,
        layer: f32,
        material_id: u32,
        instance: SpriteInstance,
    ) {
        if self.pending.len() >= MAX_SPRITES {
            return;
        }
        let id = if (texture.0 as usize) < self.textures.len() {
            texture
        } else {
            TextureId::WHITE
        };
        self.pending.push((id, material_id, layer, instance));
    }

    /// Current validated ID for an authored material name.
    pub fn material_id(&self, name: &str) -> Option<u32> {
        self.material_names.get(name).copied()
    }
    /// Most recent material preparation error; invalid editor drafts remain visible.
    pub fn material_error(&self) -> Option<&str> {
        self.material_error.as_deref()
    }
    /// Prepare only when authored definitions or resolved textures change.
    pub async fn prepare_project_materials(
        &mut self,
        device: &Device,
        project: &kiln_core::Project,
        resolve: impl Fn(&str) -> Option<TextureId>,
    ) -> Result<(), String> {
        let paths = crate::materials::texture_paths(project);
        let textures: Vec<_> = paths
            .iter()
            .map(|p| (p, resolve(p).map(|id| id.0)))
            .collect();
        let mut refs: Vec<_> = project
            .scenes
            .values()
            .flat_map(|s| &s.entities)
            .chain(project.prefabs.values().map(|p| &p.root_entity))
            .flat_map(|e| {
                e.components.iter().filter_map(move |c| {
                    if let kiln_core::Component::Sprite {
                        material: Some(name),
                        ..
                    } = c
                    {
                        Some((&e.id, name))
                    } else {
                        None
                    }
                })
            })
            .collect();
        refs.sort();
        let key = serde_json::to_vec(&(&project.shaders, &project.materials, textures, refs))
            .map_err(|e| e.to_string())?;
        if self.material_key == key {
            return self.material_error.clone().map_or(Ok(()), Err);
        }
        self.material_key = key;
        self.material_names.clear();
        self.material_pipelines.clear();
        let result = self.build_project_materials(device, project, resolve).await;
        self.material_error = result.as_ref().err().cloned();
        result
    }
    async fn build_project_materials(
        &mut self,
        device: &Device,
        project: &kiln_core::Project,
        resolve: impl Fn(&str) -> Option<TextureId>,
    ) -> Result<(), String> {
        let plans = crate::materials::plans(project)?;
        let mut pipelines = std::collections::HashMap::new();
        let mut names = std::collections::HashMap::new();
        for (index, plan) in plans.iter().enumerate() {
            let id = index as u32 + 1;
            let pipeline = self
                .build_material(device, plan, &resolve)
                .await
                .map_err(|e| format!("Material '{}': {e}", plan.name))?;
            pipelines.insert(id, pipeline);
            names.insert(plan.name.clone(), id);
        }
        self.material_names = names;
        self.material_pipelines = pipelines;
        Ok(())
    }
    async fn build_material(
        &self,
        device: &Device,
        plan: &crate::materials::MaterialPlan,
        resolve: &impl Fn(&str) -> Option<TextureId>,
    ) -> Result<MaterialPipeline, String> {
        let mut slots = Vec::new();
        for path in &plan.textures {
            let id = resolve(path).ok_or_else(|| format!("Missing material texture '{path}'"))?;
            slots.push(
                self.textures
                    .get(id.0 as usize)
                    .ok_or("Invalid material texture ID")?,
            );
        }
        let entries: Vec<_> = (0..slots.len() * 2)
            .map(|index| wgpu::BindGroupLayoutEntry {
                binding: index as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: if index % 2 == 0 {
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    }
                } else {
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                },
                count: None,
            })
            .collect();
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_textures"),
            entries: &entries,
        });
        let resources: Vec<_> = (0..slots.len() * 2)
            .map(|index| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: if index % 2 == 0 {
                    wgpu::BindingResource::TextureView(&slots[index / 2].view)
                } else {
                    wgpu::BindingResource::Sampler(&slots[index / 2].sampler)
                },
            })
            .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&plan.name),
            layout: &bgl,
            entries: &resources,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&plan.name),
            bind_group_layouts: &[&self.uniform_bgl, &self.texture_bgl, &bgl],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&plan.name),
            source: wgpu::ShaderSource::Wgsl(plan.source.clone().into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&plan.name),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &sprite_layouts(),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: &plan.entry,
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let error = device.pop_error_scope();
        #[cfg(not(target_arch = "wasm32"))]
        device.poll(wgpu::Maintain::Wait);
        if let Some(error) = error.await {
            return Err(error.to_string());
        }
        Ok(MaterialPipeline { pipeline, bindings })
    }

    /// Upload instances and draw one instanced call per distinct (material, texture) pair.
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

        // Keep transparent painter order; only adjacent texture/material runs
        // can batch without changing which sprite appears on top.
        sort_painter_order(&mut self.pending);

        let proj: [f32; 16] = camera.projection_matrix().to_cols_array();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));

        // Collect all instances into a single buffer upload.
        let all: Vec<SpriteInstance> = self.pending.iter().map(|(_, _, _, inst)| *inst).collect();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&all));

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_vertices.slice(..));
        render_pass.set_index_buffer(self.quad_indices.slice(..), wgpu::IndexFormat::Uint16);

        let mut i = 0;
        while i < self.pending.len() {
            let tex_id = self.pending[i].0;
            let mat_id = self.pending[i].1;
            let start = i;
            while i < self.pending.len()
                && self.pending[i].0 == tex_id
                && self.pending[i].1 == mat_id
            {
                i += 1;
            }
            let count = i - start;

            if let Some(material) = self.material_pipelines.get(&mat_id) {
                render_pass.set_pipeline(&material.pipeline);
                render_pass.set_bind_group(2, &material.bindings, &[]);
            } else {
                render_pass.set_pipeline(&self.pipeline);
            }

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

fn sprite_layouts() -> [wgpu::VertexBufferLayout<'static>; 2] {
    const QUAD: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0=>Float32x2];
    [
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &QUAD,
        },
        SpriteInstance::vertex_layout(),
    ]
}

/// Stable sorting preserves transparent submission order within each layer.
fn sort_painter_order(pending: &mut [(TextureId, u32, f32, SpriteInstance)]) {
    pending.sort_by(|a, b| a.2.total_cmp(&b.2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_submission_order_survives_texture_and_material_changes() {
        let instance = SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]);
        let mut pending = vec![
            (TextureId(8), 4, 1.0, instance),
            (TextureId(2), 0, -1.0, instance),
            (TextureId(1), 2, 1.0, instance),
            (TextureId(8), 1, 1.0, instance),
        ];
        sort_painter_order(&mut pending);
        assert_eq!(
            pending
                .iter()
                .map(|entry| (entry.0 .0, entry.1))
                .collect::<Vec<_>>(),
            [(2, 0), (8, 4), (1, 2), (8, 1)]
        );
    }

    fn rendered_bounds(instances: &[SpriteInstance]) -> Option<Vec<[u32; 4]>> {
        const WIDTH: u32 = 96;
        const HEIGHT: u32 = 96;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("sprite_transform_test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("device");
        let mut renderer = SpriteRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm);
        renderer.upload_quad(&queue);
        let camera = Camera::new(WIDTH as f32, HEIGHT as f32);
        let padded_bytes_per_row = (WIDTH * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let mut bounds = Vec::with_capacity(instances.len());

        for instance in instances {
            renderer.begin_frame();
            renderer.push(*instance);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sprite_transform_test_target"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_transform_test_readback"),
                size: u64::from(padded_bytes_per_row) * u64::from(HEIGHT),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sprite_transform_test_encoder"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sprite_transform_test_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                renderer.flush(&mut pass, &camera, &queue);
            }
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &readback,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_bytes_per_row),
                        rows_per_image: Some(HEIGHT),
                    },
                },
                wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
            device.poll(wgpu::Maintain::Wait);
            rx.recv().expect("readback").expect("mapping");
            let data = readback.slice(..).get_mapped_range();
            let mut min_x = WIDTH;
            let mut min_y = HEIGHT;
            let mut max_x = 0;
            let mut max_y = 0;
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let offset = y as usize * padded_bytes_per_row as usize + x as usize * 4;
                    if data[offset] > 200 {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }
            drop(data);
            readback.unmap();
            bounds.push([min_x, min_y, max_x + 1, max_y + 1]);
        }
        Some(bounds)
    }

    #[test]
    fn sprite_instance_size() {
        // Four float vectors occupy 48 bytes, followed by material ID and rotation.
        assert_eq!(std::mem::offset_of!(SpriteInstance, material_id), 48);
        assert_eq!(std::mem::offset_of!(SpriteInstance, rotation), 52);
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 56);
        assert_eq!(SpriteInstance::STRIDE, 56);
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
        sr.pending.push((
            TextureId(0),
            0,
            0.0,
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        )); // A: layer 0, tex 0
        sr.pending.push((
            TextureId(1),
            0,
            1.0,
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        )); // B: layer 1, tex 1
        sr.pending.push((
            TextureId(0),
            0,
            0.0,
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        )); // C: layer 0, tex 0

        // Extract the sort order that flush would use
        let mut ordered: Vec<(f32, u32, u32)> = sr
            .pending
            .iter()
            .map(|(tex, mat, layer, _)| (*layer, *mat, tex.0))
            .collect();
        ordered.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });

        // Layer 0 sprites (A, C) must come before layer 1 (B)
        assert_eq!(ordered[0], (0.0, 0, 0)); // A
        assert_eq!(ordered[1], (0.0, 0, 0)); // C
        assert_eq!(ordered[2], (1.0, 0, 1)); // B
    }

    /// Within the same layer and texture, submission order is preserved.
    #[test]
    fn sort_preserves_submission_order_within_same_layer_and_texture() {
        let mut sr = SpriteRendererCpu::default();
        sr.pending.push((
            TextureId(0),
            0,
            0.0,
            SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
        )); // first
        sr.pending.push((
            TextureId(0),
            0,
            0.0,
            SpriteInstance::colored([1.0; 2], [1.0; 2], [1.0; 4]),
        )); // second

        let mut ordered: Vec<(f32, u32, u32)> = sr
            .pending
            .iter()
            .map(|(tex, mat, layer, _)| (*layer, *mat, tex.0))
            .collect();
        ordered.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });

        assert_eq!(ordered[0], (0.0, 0, 0));
        assert_eq!(ordered[1], (0.0, 0, 0));
    }

    #[test]
    fn colored_defaults_full_uv() {
        let s = SpriteInstance::colored([1.0, 2.0], [3.0, 4.0], [1.0; 4]);
        assert_eq!(s.uv_rect, FULL_UV);
        assert_eq!(s.rotation, 0.0);
    }

    #[test]
    fn transform_scales_size_and_sets_rotation() {
        let sprite = SpriteInstance::colored([0.0; 2], [8.0, 4.0], [1.0; 4])
            .with_transform(std::f32::consts::FRAC_PI_2, [2.0, 0.5]);
        assert_eq!(sprite.size, [16.0, 2.0]);
        assert_eq!(sprite.rotation, std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn shader_rotates_and_scales_sprite_pixel_bounds() {
        let base = SpriteInstance::colored([48.0, 48.0], [32.0, 8.0], [1.0, 0.0, 0.0, 1.0]);
        let rotated = base.with_transform(std::f32::consts::FRAC_PI_2, [1.0, 1.0]);
        let scaled = base.with_transform(0.0, [2.0, 1.0]);
        let Some(bounds) = rendered_bounds(&[base, rotated, scaled]) else {
            eprintln!("skipping sprite transform pixel test: no GPU adapter");
            return;
        };
        let [base_bounds, rotated_bounds, scaled_bounds]: [[u32; 4]; 3] =
            bounds.try_into().expect("one bound per test sprite");
        let dimensions = |bounds: [u32; 4]| [bounds[2] - bounds[0], bounds[3] - bounds[1]];
        assert_eq!(dimensions(base_bounds), [32, 8]);
        assert_eq!(dimensions(rotated_bounds), [8, 32]);
        assert_eq!(dimensions(scaled_bounds), [64, 8]);
    }

    #[derive(Default)]
    struct SpriteRendererCpu {
        pending: Vec<(TextureId, u32, f32, SpriteInstance)>,
    }
    impl SpriteRendererCpu {
        fn push(&mut self, inst: SpriteInstance) {
            self.push_textured(TextureId::WHITE, inst);
        }
        fn push_textured(&mut self, texture: TextureId, inst: SpriteInstance) {
            if self.pending.len() < MAX_SPRITES {
                self.pending.push((texture, inst.material_id, 0.0, inst));
            }
        }
        fn len(&self) -> usize {
            self.pending.len()
        }
        fn draw_calls_if_flushed(&self) -> u32 {
            let mut ids: Vec<_> = self.pending.iter().map(|(t, _, _, _)| t.0).collect();
            ids.sort_unstable();
            ids.dedup();
            ids.len() as u32
        }
    }
}

#[cfg(test)]
mod material_target_tests {
    use super::*;
    #[test]
    fn authored_material_renders_into_float_post_target() {
        pollster::block_on(async {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                })
                .await
                .expect("Software GPU adapter required for material target test");
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .unwrap();
            let mut project = kiln_core::Project::default();
            project.shaders.insert("red".into(),kiln_core::project::Shader {id:"red".into(),name:"red".into(),source:"@fragment fn fs_main()->@location(0)vec4<f32>{return vec4<f32>(1.0,0.0,0.0,1.0);}".into(),entry_point:"fs_main".into(),parameters:Default::default()});
            project.materials.insert(
                "red".into(),
                kiln_core::project::SpriteMaterial {
                    id: "red".into(),
                    name: "red".into(),
                    shader: "red".into(),
                    parameters: Default::default(),
                },
            );
            let mut sprites =
                SpriteRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba16Float);
            sprites.upload_quad(&queue);
            sprites
                .prepare_project_materials(&device, &project, |_| None)
                .await
                .unwrap();
            sprites.begin_frame();
            sprites.push(SpriteInstance {
                position: [0.5, 0.5],
                size: [1.0, 1.0],
                color: [1.0; 4],
                material_id: sprites.material_id("red").unwrap(),
                uv_rect: FULL_UV,
                rotation: 0.0,
            });
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 256,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
                sprites.flush(&mut pass, &Camera::new(1.0, 1.0), &queue);
            }
            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            let slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
            device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let mapped = slice.get_mapped_range();
            let channels: Vec<_> = mapped[..8]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            assert_eq!(channels, [0x3c00, 0, 0, 0x3c00]);
        });
    }
}
