//! Offscreen RGBA capture + hash for visual regression tests.

use sha2::{Digest, Sha256};

use crate::frame::{clear_view, execute_passes, Frame2dGraph};
use crate::pass_config::PassEnableConfig;
use crate::profiler::{requested_profiler_features, GpuProfiler};
use crate::tier::RenderTier;

/// Golden hash for the Low-tier 2D graph screenshot (64×64, known clear colors).
///
/// Target is `Rgba8Unorm` (linear); main clears to RGB (0.20, 0.40, 0.60) →
/// bytes (51, 102, 153, 255). Refresh via `KILN_UPDATE_GOLDEN=1`.
pub const GOLDEN_LOW_64_SHA256: &str =
    "d17424c3d09af2033da87d25f93284d553c367319acf9529f19efa8c08eb9478";

/// Whether a GPU adapter is available on this machine.
///
/// The screenshot tests need a real adapter. On a headless machine (a build
/// server, a container without a GPU) there is none, and the tests skip
/// instead of failing.
#[cfg(test)]
fn has_gpu_adapter() -> bool {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .is_some()
}

/// Render the default 2D graph offscreen and return raw RGBA8 bytes + sha256 hex.
pub fn render_graph_screenshot(
    width: u32,
    height: u32,
    tier: RenderTier,
    config: &PassEnableConfig,
) -> (Vec<u8>, String) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("adapter");

    let features = requested_profiler_features(&adapter);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("screenshot_device"),
            required_features: features,
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .expect("device");

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot_target"),
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let frame = Frame2dGraph::build(width, height, tier, config);
    let compiled = frame.compile().expect("compile");

    let mut profiler = GpuProfiler::new(&device, &queue, 16);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("screenshot_enc"),
    });

    // Deterministic colors per pass so the hash is stable across machines.
    execute_passes(
        &compiled,
        &mut encoder,
        &view,
        tier,
        Some(&mut profiler),
        |ctx| match ctx.name {
            "main" => clear_view(
                ctx.encoder,
                ctx.output_view,
                wgpu::Color {
                    r: 0.20,
                    g: 0.40,
                    b: 0.60,
                    a: 1.0,
                },
            ),
            "bloom" | "fog" | "outline" | "lut" => {
                // Stubs: no visual change (load existing).
                let _ = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ctx.name),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: ctx.output_view,
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
            }
            "composite" => {
                // Final store already on output from main for Low; Load keeps pixels.
                let _ = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: ctx.output_view,
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
            }
            _ => {}
        },
    );

    let unpadded_bytes_per_row = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let buffer_size = padded_bytes_per_row as u64 * height as u64;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screenshot_readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    profiler.resolve(&mut encoder);
    queue.submit(Some(encoder.finish()));
    profiler.poll_timings(&device);

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().expect("map");

    let data = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        rgba.extend_from_slice(&data[start..end]);
    }
    drop(data);
    readback.unmap();

    let hash = hex_sha256(&rgba);
    (rgba, hash)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Expected solid-color RGBA for Low-tier main clear (sRGB 8-bit approx).
pub fn expected_low_pixel() -> [u8; 4] {
    // 0.20/0.40/0.60 in Clear → typically ~51, 102, 153 in UNORM (exact depends on path).
    // We validate via full-buffer hash computed once and stored; see test.
    [51, 102, 153, 255]
}

/// Compute golden hash locally (used to refresh constant after intentional visual change).
pub fn compute_low_golden() -> String {
    let cfg = PassEnableConfig::default_2d();
    let (_rgba, hash) = render_graph_screenshot(64, 64, RenderTier::Low, &cfg);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_low_tier_stable_hash() {
        if !has_gpu_adapter() {
            eprintln!("skipping: this machine has no GPU adapter");
            return;
        }

        let cfg = PassEnableConfig::default_2d();
        let (rgba, hash) = render_graph_screenshot(64, 64, RenderTier::Low, &cfg);
        assert_eq!(rgba.len(), 64 * 64 * 4);

        // First run records; subsequent runs must match. If this fails after an
        // intentional visual change, update GOLDEN via compute_low_golden().
        let golden = std::env::var("KILN_UPDATE_GOLDEN").ok();
        if golden.as_deref() == Some("1") {
            eprintln!("KILN_LOW_GOLDEN={hash}");
            panic!(
                "re-run without KILN_UPDATE_GOLDEN after pasting hash into GOLDEN_LOW_64_SHA256"
            );
        }

        let p = [rgba[0], rgba[1], rgba[2], rgba[3]];
        assert_eq!(p, expected_low_pixel(), "clear color mismatch");

        let (rgba2, hash2) = render_graph_screenshot(64, 64, RenderTier::Low, &cfg);
        assert_eq!(hash, hash2, "two renders must match (no visual regression)");
        assert_eq!(rgba, rgba2);

        if GOLDEN_LOW_64_SHA256.len() == 64
            && GOLDEN_LOW_64_SHA256.chars().all(|c| c.is_ascii_hexdigit())
        {
            assert_eq!(hash, GOLDEN_LOW_64_SHA256, "visual regression vs golden");
        }
    }

    #[test]
    fn high_and_low_graphs_differ_when_post_clears() {
        // With stub Load ops they may match; ensure compile paths differ at least.
        let cfg = PassEnableConfig::default_2d();
        let low = Frame2dGraph::build(32, 32, RenderTier::Low, &cfg);
        let high = Frame2dGraph::build(32, 32, RenderTier::High, &cfg);
        assert_ne!(
            low.compile().unwrap().ordered_pass_names(),
            high.compile().unwrap().ordered_pass_names()
        );
    }
}
