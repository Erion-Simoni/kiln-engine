//! M28 sample: render graph compile + GPU pass timings + screenshot hash.

use kiln_render::{
    clear_view, execute_passes, render_graph_screenshot, Frame2dGraph, PassEnableConfig,
    RenderTier,
};

fn main() {
    println!("Kiln M28 — Render graph + GPU profiling");

    let cfg = PassEnableConfig::default_2d();
    println!("\nPass enable (from data):");
    for p in &cfg.passes {
        println!("  {} → {:?}", p.name, p.tiers);
    }

    for tier in [RenderTier::Low, RenderTier::Medium, RenderTier::High] {
        let frame = Frame2dGraph::build(1280, 720, tier, &cfg);
        let compiled = frame.compile().expect("compile");
        println!("\n{:?} tier pass order: {:?}", tier, compiled.ordered_pass_names());
        println!(
            "  transient slots (aliased): {} physical for {} resources",
            compiled.slot_count,
            compiled.resources.len()
        );
    }

    // Custom data disables bloom on High
    let custom = PassEnableConfig::from_toml(
        r#"
[[passes]]
name = "main"
tiers = ["high"]
[[passes]]
name = "composite"
tiers = ["high"]
"#,
    )
    .expect("toml");
    let slim = Frame2dGraph::build(64, 64, RenderTier::High, &custom);
    println!(
        "\nHigh with bloom disabled via data: {:?}",
        slim.compile().unwrap().ordered_pass_names()
    );

    println!("\nOffscreen screenshot (Low, 64×64)…");
    let (rgba, hash) = render_graph_screenshot(64, 64, RenderTier::Low, &cfg);
    println!("  bytes: {}  sha256: {}", rgba.len(), hash);
    println!(
        "  pixel[0]: rgba({}, {}, {}, {})",
        rgba[0], rgba[1], rgba[2], rgba[3]
    );

    // Timed graph execute (may be empty if TIMESTAMP_QUERY unsupported)
    demo_gpu_timings(&cfg);
}

fn demo_gpu_timings(cfg: &PassEnableConfig) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }));
    let Some(adapter) = adapter else {
        println!("\nNo GPU adapter; skip live timings.");
        return;
    };

    let features = kiln_render::profiler::requested_profiler_features(&adapter);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("m28_sample"),
            required_features: features,
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .expect("device");

    let mut profiler = kiln_render::GpuProfiler::new(&device, &queue, 16);
    println!(
        "\nTIMESTAMP_QUERY supported: {}",
        profiler.supported()
    );

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sample_rt"),
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

    let frame = Frame2dGraph::build(256, 256, RenderTier::High, cfg);
    let compiled = frame.compile().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sample_enc"),
    });

    execute_passes(
        &compiled,
        &mut encoder,
        &view,
        RenderTier::High,
        Some(&mut profiler),
        |ctx| match ctx.name {
            "main" => clear_view(
                ctx.encoder,
                ctx.output_view,
                wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                },
            ),
            _ => clear_view(
                ctx.encoder,
                ctx.output_view,
                wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                },
            ),
        },
    );
    profiler.resolve(&mut encoder);
    queue.submit(Some(encoder.finish()));
    profiler.poll_timings(&device);

    if profiler.timings().is_empty() {
        println!("  (no timings resolved — feature missing or query failed)");
    } else {
        println!("  Per-pass GPU cost:");
        for t in profiler.timings() {
            println!("    {}: {:.2} µs", t.name, t.duration_us());
        }
    }
}
