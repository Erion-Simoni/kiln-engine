//! Kiln rendering crate — wgpu-based 2D renderer with batching and tier support.

pub mod camera;
pub mod post;
pub mod renderer;
pub mod sprite_renderer;
pub mod tier;

pub use post::{PostConfig, PostFramebuffer, PostPipeline};
pub use renderer::Renderer;
pub use sprite_renderer::{SpriteInstance, SpriteRenderer, MAX_SPRITES};
pub use tier::RenderTier;
