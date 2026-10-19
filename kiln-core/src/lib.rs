//! Kiln core crate
//! Math, time, shared types, ECS glue

pub mod math;
pub mod time;
pub mod id;
pub mod project;
pub mod transform;
pub mod paths;

pub use math::*;
pub use time::*;
pub use id::*;
pub use project::{Project, Scene, Entity, Component, WindowSettings};
pub use transform::*;
pub use paths::{save_dir, settings_dir, cache_dir};
