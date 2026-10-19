//! Kiln rendering crate — wgpu-based 2D renderer with batching, tiers, and a frame graph.

pub mod camera;
pub mod frame;
pub mod graph;
pub mod particle_renderer;
pub mod pass_config;
pub mod post;
pub mod profiler;
pub mod renderer;
pub mod screenshot;
pub mod sprite_renderer;
pub mod tier;

pub use frame::{execute_passes, clear_view, Frame2dGraph, FrameResources, PassExecContext};
pub use graph::{
    CompiledGraph, GraphError, PassDecl, PassId, RenderGraph, ResourceDesc, ResourceId,
    ResourceKind, ResourceRef, ResourceUsage, TextureFormatKey,
};
pub use particle_renderer::ParticleRenderer;
pub use pass_config::{PassEnableConfig, PassTierEnable};
pub use post::{PostConfig, PostFramebuffer, PostPipeline};
pub use profiler::{GpuProfiler, PassTiming};
pub use renderer::Renderer;
pub use screenshot::{render_graph_screenshot, GOLDEN_LOW_64_SHA256};
pub use sprite_renderer::{SpriteInstance, SpriteRenderer, MAX_SPRITES};
pub use tier::RenderTier;
