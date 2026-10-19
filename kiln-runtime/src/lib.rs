//! Kiln runtime crate
//! Frame loop, scene, input, audio hooks

pub mod input;
pub mod jobs;
pub mod loop_;
pub mod schedule;
pub mod settings;
pub mod sprite;
pub mod world;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "physics")]
pub mod physics;

pub use input::*;
pub use jobs::{load_file_async, JobHandle, TaskPool};
pub use loop_::*;
pub use schedule::{build_stages, Schedule, SchedulePlan, SystemEntry};
pub use settings::*;
pub use sprite::*;
pub use world::*;

#[cfg(feature = "audio")]
pub use audio::*;

#[cfg(feature = "physics")]
pub use physics::*;
