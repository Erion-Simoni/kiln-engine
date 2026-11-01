//! 2D animation system for Kiln.
//!
//! Provides spritesheet clips, tween library, and animation state machine support.

mod clip;
mod tween;

pub use clip::{Clip, ClipError, Frame, FrameEvent};
pub use tween::{Ease, Tween};

/// The gameplay role of an entity, e.g. `"player"` or `"coin"`.
///
/// Roles let a scene's script find entities by purpose instead of by index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role(pub String);

impl Role {
    /// A role with the given name.
    pub fn new(role: impl Into<String>) -> Self {
        Self(role.into())
    }

    /// The role name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The stable id of the scene entity an entity was spawned from.
///
/// The id survives save/load, so a save file can refer to the same entity
/// after the world is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceId(pub String);

impl SourceId {
    /// A source id with the given value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The source id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 2D transform (position).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Position in world space.
    pub position: [f32; 2],
}

impl Transform {
    pub fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }
}

/// 2D velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity {
    /// Linear velocity.
    pub linear: [f32; 2],
}

impl Velocity {
    pub fn new(vx: f32, vy: f32) -> Self {
        Self { linear: [vx, vy] }
    }
}

/// Joint type for 2D physics joints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointType {
    /// Keeps two bodies at a fixed distance from each other.
    Distance,
    /// Revolute joint (hinge) - allows rotation around a point.
    Revolute,
    /// Prismatic joint (slider) - allows linear motion along an axis.
    Prismatic,
}

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
