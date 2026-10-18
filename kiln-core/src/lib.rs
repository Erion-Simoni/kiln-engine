//! Kiln core crate
//! Math, time, shared types, ECS glue

pub mod math;
pub mod time;
pub mod id;
pub mod project;
pub mod transform;

pub use math::*;
pub use time::*;
pub use id::*;
pub use project::*;
pub use transform::*;
