//! 2D animation system for Kiln.
//!
//! Provides spritesheet clips, tween library, and animation state machine support.

mod clip;
mod tween;

pub use clip::{Clip, Frame, FrameEvent};
pub use tween::{Ease, Tween};

/// Error type for animation operations.
#[derive(Debug, thiserror::Error)]
pub enum AnimError {
    #[error("Clip not found: {0}")]
    ClipNotFound(String),
    #[error("Frame index out of bounds: {0}")]
    FrameOutOfBounds(usize),
    #[error("Invalid tween duration: {0}")]
    InvalidDuration(f32),
}
