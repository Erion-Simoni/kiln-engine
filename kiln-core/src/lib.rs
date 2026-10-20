//! Kiln core crate
//! Math, time, shared types, ECS glue

pub mod access;
pub mod device_tier;
pub mod id;
pub mod math;
pub mod paths;
pub mod prefab;
pub mod project;
pub mod tilemap;
pub mod time;
pub mod transform;

pub use access::{any_conflicts, AccessKind, AccessSet, ComponentAccess};
pub use device_tier::{
    detect_device_tier, get_device_capabilities, DeviceCapabilities, DeviceTier,
};
pub use id::*;
pub use math::*;
pub use paths::{cache_dir, save_dir, settings_dir};
pub use prefab::{ComponentOverride, Prefab, PrefabInstance, PrefabOverride};
pub use project::{
    AndroidSettings, IosSettings, LaunchScreen, MobileSettings, Orientation, PrivacyDescriptions,
};
pub use project::{Component, Entity, NestedScene, Project, Scene, SceneOffset, WindowSettings};
pub use tilemap::{TileChunk, TileMap, TileMapLayer, TileSet, CHUNK_SIZE, EMPTY_TILE};
pub use time::*;
pub use transform::*;
