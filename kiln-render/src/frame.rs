//! Default 2D frame graph (M2 sprites + M9 post stubs) built from pass config.

use std::collections::HashSet;

use wgpu::{CommandEncoder, TextureView};

use crate::graph::{
    CompiledGraph, PassDecl, RenderGraph, ResourceDesc, ResourceId, ResourceKind, ResourceRef,
    ResourceUsage, TextureFormatKey,
};
use crate::pass_config::PassEnableConfig;
use crate::profiler::GpuProfiler;
use crate::tier::RenderTier;

/// Logical resources used by the default 2D graph.
#[derive(Debug, Clone, Copy)]
pub struct FrameResources {
    pub scene: ResourceId,
    pub bloom_tmp: ResourceId,
    pub post_tmp: ResourceId,
    pub output: ResourceId,
}

/// Built 2D graph ready to compile / execute.
pub struct Frame2dGraph {
    pub graph: RenderGraph,
    pub resources: FrameResources,
    pub enabled: HashSet<String>,
    pub tier: RenderTier,
}

impl Frame2dGraph {
    /// Build the default 2D pass list filtered by [`PassEnableConfig`] for `tier`.
    pub fn build(width: u32, height: u32, tier: RenderTier, config: &PassEnableConfig) -> Self {
        let enabled = config.enabled_for(tier);
        let mut graph = RenderGraph::new();

        let scene = graph.add_resource(ResourceDesc {
            name: "scene_color".into(),
            kind: ResourceKind::Texture {
                width,
                height,
                format: TextureFormatKey::Rgba8Unorm,
            },
            transient: true,
        });
        let bloom_tmp = graph.add_resource(ResourceDesc {
            name: "bloom_tmp".into(),
            kind: ResourceKind::Texture {
                width: (width / 2).max(1),
                height: (height / 2).max(1),
                format: TextureFormatKey::Rgba16Float,
            },
            transient: true,
        });
        let post_tmp = graph.add_resource(ResourceDesc {
            name: "post_tmp".into(),
            kind: ResourceKind::Texture {
                width,
                height,
                format: TextureFormatKey::Rgba8Unorm,
            },
            transient: true,
        });
        let output = graph.add_resource(ResourceDesc {
            name: "output".into(),
            kind: ResourceKind::Texture {
                width,
                height,
                format: TextureFormatKey::Surface,
            },
            transient: false,
        });

        if enabled.contains("main") {
            graph.add_pass(PassDecl {
                name: "main".into(),
                resources: vec![ResourceRef {
                    id: scene,
                    usage: ResourceUsage::Write,
                }],
            });
        }
        if enabled.contains("bloom") {
            graph.add_pass(PassDecl {
                name: "bloom".into(),
                resources: vec![
                    ResourceRef {
                        id: scene,
                        usage: ResourceUsage::Read,
                    },
                    ResourceRef {
                        id: bloom_tmp,
                        usage: ResourceUsage::Write,
                    },
                ],
            });
        }

        // Screen-space post chain: first writer fills post_tmp from scene; later
        // enabled passes only Read post_tmp (stub timing markers, no RMW cycle).
        let mut post_written = false;
        for name in ["fog", "outline", "lut"] {
            if !enabled.contains(name) {
                continue;
            }
            if !post_written {
                graph.add_pass(PassDecl {
                    name: name.into(),
                    resources: vec![
                        ResourceRef {
                            id: scene,
                            usage: ResourceUsage::Read,
                        },
                        ResourceRef {
                            id: post_tmp,
                            usage: ResourceUsage::Write,
                        },
                    ],
                });
                post_written = true;
            } else {
                graph.add_pass(PassDecl {
                    name: name.into(),
                    resources: vec![ResourceRef {
                        id: post_tmp,
                        usage: ResourceUsage::Read,
                    }],
                });
            }
        }

        if enabled.contains("composite") {
            let mut resources = vec![
                ResourceRef {
                    id: scene,
                    usage: ResourceUsage::Read,
                },
                ResourceRef {
                    id: output,
                    usage: ResourceUsage::Write,
                },
            ];
            if enabled.contains("bloom") {
                resources.push(ResourceRef {
                    id: bloom_tmp,
                    usage: ResourceUsage::Read,
                });
            }
            if post_written {
                resources.push(ResourceRef {
                    id: post_tmp,
                    usage: ResourceUsage::Read,
                });
            }
            graph.add_pass(PassDecl {
                name: "composite".into(),
                resources,
            });
        }

        Self {
            graph,
            resources: FrameResources {
                scene,
                bloom_tmp,
                post_tmp,
                output,
            },
            enabled,
            tier,
        }
    }

    pub fn compile(&self) -> Result<CompiledGraph, crate::graph::GraphError> {
        self.graph.compile()
    }
}

/// Context passed to each pass execute callback.
pub struct PassExecContext<'a> {
    pub name: &'a str,
    pub encoder: &'a mut CommandEncoder,
    pub output_view: &'a TextureView,
    pub tier: RenderTier,
}

/// Execute compiled pass order with optional GPU profiling.
///
/// `run_pass` performs the real GPU work for each enabled pass name.
pub fn execute_passes(
    compiled: &CompiledGraph,
    encoder: &mut CommandEncoder,
    output_view: &TextureView,
    tier: RenderTier,
    mut profiler: Option<&mut GpuProfiler>,
    mut run_pass: impl FnMut(PassExecContext<'_>),
) {
    if let Some(p) = profiler.as_mut() {
        p.begin_frame();
    }

    for pid in &compiled.pass_order {
        let name = compiled.pass_names[pid.0 as usize].as_str();
        let begin = profiler.as_mut().and_then(|p| p.begin_pass(encoder, name));

        run_pass(PassExecContext {
            name,
            encoder,
            output_view,
            tier,
        });

        if let Some(p) = profiler.as_mut() {
            p.end_pass(encoder, begin);
        }
    }
    // Caller resolves after any extra passes (e.g. egui) via GpuProfiler::resolve.
}

/// Minimal clear-color pass used by screenshot / sample when no sprite batch is wired.
pub fn clear_view(encoder: &mut CommandEncoder, view: &TextureView, color: wgpu::Color) {
    let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("graph_clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_tier_graph_only_main_and_composite() {
        let cfg = PassEnableConfig::default_2d();
        let f = Frame2dGraph::build(128, 128, RenderTier::Low, &cfg);
        let names: HashSet<_> = f.graph.passes().iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("main"));
        assert!(names.contains("composite"));
        assert!(!names.contains("bloom"));
        assert!(!names.contains("lut"));
        let c = f.compile().unwrap();
        assert_eq!(c.ordered_pass_names(), vec!["main", "composite"]);
    }

    #[test]
    fn high_tier_includes_post_passes() {
        let cfg = PassEnableConfig::default_2d();
        let f = Frame2dGraph::build(128, 128, RenderTier::High, &cfg);
        let c = f.compile().unwrap();
        let names = c.ordered_pass_names();
        assert!(names.contains(&"bloom"));
        assert!(names.contains(&"outline"));
        assert!(names.contains(&"lut"));
        assert_eq!(*names.last().unwrap(), "composite");
    }

    #[test]
    fn data_can_disable_bloom_on_high() {
        let toml = r#"
[[passes]]
name = "main"
tiers = ["high"]
[[passes]]
name = "composite"
tiers = ["high"]
"#;
        let cfg = PassEnableConfig::from_toml(toml).unwrap();
        let f = Frame2dGraph::build(64, 64, RenderTier::High, &cfg);
        let names: Vec<_> = f.graph.passes().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["main", "composite"]);
    }
}
