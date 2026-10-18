//! Kiln rendering crate — wgpu-based 2D renderer with batching and tier support.

pub mod camera;
pub mod renderer;
pub mod sprite_renderer;

pub use renderer::Renderer;
pub use sprite_renderer::{SpriteInstance, SpriteRenderer, MAX_SPRITES};
