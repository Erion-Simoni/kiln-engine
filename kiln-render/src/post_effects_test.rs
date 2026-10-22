//! Per-effect tests for the post-processing chain.
//!
//! The chain has three effects, and each one is checked on its own: the bloom
//! spreads light, the fog tints the image and the outline marks the edge of a
//! shape. The tests render a scene of known colours and compare the result
//! with the same scene rendered without the effect, so a broken effect fails
//! the test instead of silently doing nothing.

use crate::post::{PostConfig, PostFramebuffer};
use crate::post_gpu::PostGpuPipeline;
use crate::RenderTier;
use wgpu::{CommandEncoder, Device, Queue, TextureView};

/// A scene pixel: its position and its colour.
type ScenePixel = (u32, u32, [f32; 3]);

/// The number of quads the scene shader can hold.
const MAX_QUADS: usize = 64;

/// Render a scene through the post chain and read the result back.
///
/// Returns `None` when the machine has no GPU adapter, so the test is skipped
/// instead of failing.
fn render_post(
    width: u32,
    height: u32,
    scene: &[ScenePixel],
    config: PostConfig,
) -> Option<Vec<Vec<[u8; 4]>>> {
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
            label: Some("post_effect_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .expect("device");

    let framebuffer = PostFramebuffer::new(&device, width, height, RenderTier::High);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("post_effect_output"),
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
    pipeline.set_config(config);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("post_effect_enc"),
    });

    // The scene is drawn as quads, so the framebuffer is cleared first and
    // the coloured parts are drawn on top of it.
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post_effect_scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: framebuffer.color_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
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
    draw_scene_pixels(
        &device,
        &queue,
        &mut encoder,
        framebuffer.color_view(),
        width,
        height,
        scene,
    );

    pipeline.apply(&device, &queue, &mut encoder, &framebuffer, &output_view);

    let bytes_per_row = width * 4;
    let padded = bytes_per_row.div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("post_effect_readback"),
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
    let mut rows = Vec::with_capacity(height as usize);
    for y in 0..height as usize {
        let start = y * padded as usize;
        let mut row = Vec::with_capacity(width as usize);
        for x in 0..width as usize {
            let offset = start + x * 4;
            row.push([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
        }
        rows.push(row);
    }
    drop(data);
    readback.unmap();
    Some(rows)
}

/// Draw coloured quads into a texture, one per scene pixel.
fn draw_scene_pixels(
    device: &Device,
    queue: &Queue,
    encoder: &mut CommandEncoder,
    view: &TextureView,
    width: u32,
    height: u32,
    scene: &[ScenePixel],
) {
    if scene.is_empty() {
        return;
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("post_effect_shader"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
// Two entries per scene pixel: the quad and its colour.
@group(0) @binding(0) var<uniform> quads: array<vec4<f32>, 128>;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[vi];
    let q = quads[ii * 2u];
    let offset = vec2<f32>(q.x, q.y);
    let size = vec2<f32>(q.z, q.w);
    var out: VsOut;
    out.clip_pos = vec4<f32>(p * size + offset, 0.0, 1.0);
    out.color = quads[ii * 2u + 1u].rgb;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#
            .into(),
        ),
    });

    // One quad per scene pixel: the centre of the pixel and its size in
    // normalised device coordinates.
    let mut quads: Vec<f32> = Vec::with_capacity(scene.len() * 8);
    for (x, y, color) in scene {
        let cx = *x as f32 + 0.5;
        let cy = *y as f32 + 0.5;
        quads.push((cx / width as f32) * 2.0 - 1.0);
        quads.push(1.0 - (cy / height as f32) * 2.0);
        quads.push(1.0 / width as f32);
        quads.push(1.0 / height as f32);
        quads.push(color[0]);
        quads.push(color[1]);
        quads.push(color[2]);
        quads.push(0.0);
    }
    // The shader reads a fixed array, so the buffer always holds that many
    // quads and the unused ones are zero.
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("post_effect_uniform"),
        size: (MAX_QUADS * 32) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut padded = quads.clone();
    padded.resize(MAX_QUADS * 8, 0.0);
    queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&padded));

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("post_effect_bgl"),
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
        label: Some("post_effect_bg"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post_effect_pipeline"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("post_effect_layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("post_effect_quads"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..scene.len() as u32);
    drop(pass);
}

/// The bloom of a bright spot spreads to its neighbours.
///
/// The scene is a single bright pixel in the middle of a black image. With
/// bloom on, the pixels around it are lit too; with bloom off, they stay
/// black.
#[test]
fn bloom_spreads_light_to_the_neighbours() {
    let width = 16u32;
    let height = 16u32;
    let center = (8u32, 8u32);

    let mut bloom = PostConfig::default();
    bloom.bloom_threshold = 0.5;
    bloom.bloom_intensity = 1.0;
    bloom.fog_density = 0.0;
    bloom.outline_thickness = 0.0;

    let mut plain = bloom;
    plain.bloom_intensity = 0.0;

    let scene = vec![(center.0, center.1, [1.0, 1.0, 1.0])];

    let Some(with_bloom) = render_post(width, height, &scene, bloom) else {
        eprintln!("skipping bloom test: no adapter");
        return;
    };
    let Some(without_bloom) = render_post(width, height, &scene, plain) else {
        return;
    };

    let neighbour = (center.0 + 2, center.1);
    let lit = with_bloom[neighbour.1 as usize][neighbour.0 as usize][0];
    let dark = without_bloom[neighbour.1 as usize][neighbour.0 as usize][0];
    assert!(
        lit > dark,
        "the bloom did not spread: {lit} is not brighter than {dark}"
    );
}

/// The fog tints the whole image towards the fog colour.
///
/// A green scene with a red fog is redder than the same scene without fog.
#[test]
fn fog_tints_the_whole_image() {
    let width = 8u32;
    let height = 8u32;

    let mut fog = PostConfig::default();
    fog.bloom_intensity = 0.0;
    fog.outline_thickness = 0.0;
    fog.fog_density = 0.5;
    fog.fog_color = [1.0, 0.0, 0.0];

    let mut plain = fog;
    plain.fog_density = 0.0;

    // A green scene, so the red fog colour shows up as red.
    let scene = vec![(4u32, 4u32, [0.0, 1.0, 0.0])];

    let Some(with_fog) = render_post(width, height, &scene, fog) else {
        eprintln!("skipping fog test: no adapter");
        return;
    };
    let Some(without_fog) = render_post(width, height, &scene, plain) else {
        return;
    };

    let fogged = with_fog[4][4];
    let plain_pixel = without_fog[4][4];
    assert!(
        fogged[0] > plain_pixel[0],
        "the fog did not add red: {fogged:?} is not redder than {plain_pixel:?}"
    );
}

/// The outline marks the edge of a shape, not its flat middle.
///
/// The scene is a black image with a white block in the middle. The outline
/// colours the pixels where the shape meets the background and leaves the flat
/// middle alone.
#[test]
fn outline_marks_the_edge_of_a_shape() {
    let width = 16u32;
    let height = 16u32;
    let middle = (8u32, 8u32);

    let mut outline = PostConfig::default();
    outline.bloom_intensity = 0.0;
    outline.fog_density = 0.0;
    outline.outline_thickness = 1.0;
    outline.outline_threshold = 0.1;
    outline.outline_color = [1.0, 0.0, 0.0, 1.0];

    let mut plain = outline;
    plain.outline_thickness = 0.0;

    // A white block in the middle of a black image.
    let mut scene = Vec::new();
    for y in 6..10u32 {
        for x in 6..10u32 {
            scene.push((x, y, [1.0, 1.0, 1.0]));
        }
    }

    let Some(with_outline) = render_post(width, height, &scene, outline) else {
        eprintln!("skipping outline test: no adapter");
        return;
    };
    let Some(without_outline) = render_post(width, height, &scene, plain) else {
        return;
    };

    // The pixel on the edge of the block is outlined. The outline colour is
    // red, so the green channel drops where the outline is.
    let edge = (6u32, 8u32);
    let outlined = with_outline[edge.1 as usize][edge.0 as usize];
    let plain_edge = without_outline[edge.1 as usize][edge.0 as usize];
    assert!(
        outlined[1] < plain_edge[1],
        "the edge was not outlined: {outlined:?} is not redder than {plain_edge:?}"
    );

    // The middle of the block is flat, so it is not outlined.
    let flat = with_outline[middle.1 as usize][middle.0 as usize];
    let plain_middle = without_outline[middle.1 as usize][middle.0 as usize];
    assert_eq!(
        flat, plain_middle,
        "the flat middle of the shape was outlined"
    );
}
