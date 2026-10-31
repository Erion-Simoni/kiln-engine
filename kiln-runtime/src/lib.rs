//! Kiln runtime crate
//! Frame loop, scene, input, audio hooks

pub mod input;
pub mod jobs;
pub mod loop_;
pub mod scene;
pub mod schedule;
pub mod settings;
pub mod slice;
pub mod sprite;
pub mod tilemap;
pub mod tilemap_collision;
pub mod tilemap_scene;
pub mod world;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "physics")]
pub mod physics;

pub use input::*;
pub use jobs::{load_file_async, JobHandle, TaskPool};
pub use loop_::*;
pub use scene::{count_scene_entities, spawn_scene, SpawnedEntity, SpriteFactory};
pub use schedule::{build_stages, Schedule, SchedulePlan, SystemEntry};
pub use settings::*;
pub use slice::*;
pub use sprite::*;
pub use tilemap::*;
pub use tilemap_collision::{entity_box, first_solid_hit, move_with_collision};
pub use tilemap_scene::{spawn_tilemaps, TilesetTextureResolver};
pub use world::*;

#[cfg(feature = "audio")]
pub use audio::*;

#[cfg(feature = "physics")]
pub use physics::*;