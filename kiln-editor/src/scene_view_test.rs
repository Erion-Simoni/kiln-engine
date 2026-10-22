//! A test of the editor's scene viewport.
//!
//! The scene view draws the world with the engine's sprite renderer, so the
//! test renders a sprite with the same camera and checks that it lands where
//! the editor puts it.

use kiln_render::{SpriteInstance, SpriteRenderer, TextureId};

use kiln_editor::scene_view::world_camera;

/// Render a sprite with the scene view's camera and read the result back.
///
/// Returns `None` when the machine has no GPU adapter, so the test is
/// skipped instead of failing.
fn render_scene(width: u32, height: u32) -> Option<Vec<Vec<[u8; 4]>>> {
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
            label: Some("scene_view_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .expect("device");

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene_view_target"),
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
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = SpriteRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm);
    renderer.upload_quad(&queue);

    // The camera the scene view uses, and a sprite in the middle of the
    // world it shows.
    let camera = world_camera(width as f32, height as f32);
    renderer.begin_frame();
    renderer.push_textured(
        TextureId::WHITE,
        SpriteInstance::colored(
            [width as f32 / 2.0, height as f32 / 2.0],
            [8.0, 8.0],
            [1.0, 1.0, 1.0, 1.0],
        ),
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("scene_view_enc"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene_view_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        renderer.flush(&mut pass, &camera, &queue);
    }

    let bytes_per_row = width * 4;
    let padded = bytes_per_row.div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene_view_readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
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

/// The scene view draws the world where the camera says it is.
#[test]
fn the_scene_view_draws_the_world_in_the_middle_of_the_panel() {
    let width = 32u32;
    let height = 32u32;

    let Some(pixels) = render_scene(width, height) else {
        eprintln!("skipping scene view test: no adapter");
        return;
    };

    // The middle of the view is where the sprite was drawn.
    let middle = pixels[height as usize / 2][width as usize / 2];
    assert!(
        middle[0] > 0 || middle[1] > 0 || middle[2] > 0,
        "the sprite was not drawn in the middle of the view: {middle:?}"
    );

    // The corner is outside the sprite, so it stays black.
    let corner = pixels[0][0];
    assert_eq!(
        corner,
        [0, 0, 0, 255],
        "the corner was not left alone: {corner:?}"
    );
}
