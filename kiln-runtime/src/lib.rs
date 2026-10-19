//! Kiln runtime crate
//! Frame loop, scene, input, audio hooks

pub mod loop_;
pub mod input;
pub mod world;
pub mod sprite;
pub mod settings;
pub mod jobs;
pub mod schedule;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "physics")]
pub mod physics;

pub use loop_::*;
pub use input::*;
pub use world::*;
pub use sprite::*;
pub use settings::*;
pub use jobs::{load_file_async, JobHandle, TaskPool};
pub use schedule::{build_stages, Schedule, SchedulePlan, SystemEntry};

#[cfg(feature = "audio")]
pub use audio::*;

#[cfg(feature = "physics")]
pub use physics::*;
