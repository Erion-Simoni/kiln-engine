//! Real GPU regression: different views must survive one command submission.
use crate::{camera::Camera, SpriteInstance, SpriteRenderer};
#[test]
fn prepared_views_keep_distinct_camera_and_instance_uploads_until_submit() {
    let gpu = wgpu::Instance::default();
    let Some(adapter) =
        pollster::block_on(gpu.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert!(
            std::env::var_os("KILN_REQUIRE_GPU_TESTS").is_none(),
            "GPU adapter required"
        );
        return;
    };
    eprintln!("prepared views adapter: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
            .unwrap();
    let mut renderer = SpriteRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm);
    renderer.upload_quad(&queue);
    let a = Camera::new(32.0, 32.0);
    let mut b = a.clone();
    b.position = [128.0, 0.0];
    b.zoom = 2.0;
    renderer.begin_frame();
    assert!(renderer.prepare_view(&device, &a).unwrap().is_none());
    renderer.push(SpriteInstance::colored(
        [16.0, 16.0],
        [16.0; 2],
        [0.0, 1.0, 0.0, 1.0],
    ));
    let first = renderer.prepare_view(&device, &a).unwrap().unwrap();
    renderer.begin_frame();
    renderer.push(SpriteInstance::colored(
        [136.0, 8.0],
        [8.0; 2],
        [1.0, 0.0, 0.0, 1.0],
    ));
    let second = renderer.prepare_view(&device, &b).unwrap().unwrap();
    assert_eq!(first.queued_count(), 1);
    assert_eq!(second.draw_calls(), 1);
    renderer.begin_frame(); // Prepared geometry survives clearing the CPU queue.
    let size = wgpu::Extent3d {
        width: 64,
        height: 32,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("two_views"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("two_views_read"),
        size: 256 * 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("two_views_pass"),
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
        pass.set_viewport(0.0, 0.0, 32.0, 32.0, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, 32, 32);
        renderer.draw_prepared(&mut pass, &first).unwrap();
        pass.set_viewport(32.0, 0.0, 32.0, 32.0, 0.0, 1.0);
        pass.set_scissor_rect(32, 0, 32, 32);
        renderer.draw_prepared(&mut pass, &second).unwrap();
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer: &read,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(32),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let bytes = slice.get_mapped_range();
    let mut green = Vec::new();
    let mut red = Vec::new();
    for y in 0..32 {
        for x in 0..64 {
            let at = y * 256 + x * 4;
            let p = &bytes[at..at + 4];
            if p[1] > 220 && p[0] < 30 {
                green.push((x, y));
            }
            if p[0] > 220 && p[1] < 30 {
                red.push((x, y));
            }
        }
    }
    assert_eq!(green.len(), 256);
    assert_eq!(red.len(), 256);
    assert!(green
        .iter()
        .all(|&(x, y)| (8..24).contains(&x) && (8..24).contains(&y)));
    assert!(red
        .iter()
        .all(|&(x, y)| (40..56).contains(&x) && (8..24).contains(&y)));
    drop(bytes);
    read.unmap();
    let texture = renderer.create_texture_rgba(&device, &queue, "replaced", 1, 1, &[255; 4]);
    renderer.push_textured(
        texture,
        SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4]),
    );
    renderer.clear_project_textures();
    assert_eq!(renderer.queued_count(), 0);
    let foreign = SpriteRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm);
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        assert!(renderer.draw_prepared(&mut pass, &first).is_err());
        assert!(foreign.draw_prepared(&mut pass, &second).is_err());
    }
    let mut invalid = a;
    invalid.zoom = 0.0;
    assert!(renderer.prepare_view(&device, &invalid).is_err());
}
