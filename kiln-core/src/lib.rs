//! Kiln core crate
//! Math, time, shared types, ECS glue

pub mod math;
pub mod time;
pub mod id;
pub mod project;
pub mod transform;
pub mod paths;
pub mod prefab;
pub mod device_tier;
pub mod access;

pub use math::*;
pub use time::*;
pub use id::*;
pub use project::{Project, Scene, Entity, Component, WindowSettings, NestedScene, SceneOffset};
pub use project::{MobileSettings, Orientation, IosSettings, PrivacyDescriptions, LaunchScreen, AndroidSettings};
pub use transform::*;
pub use paths::{save_dir, settings_dir, cache_dir};
pub use prefab::{Prefab, PrefabOverride, ComponentOverride, PrefabInstance};
pub use device_tier::{DeviceTier, DeviceCapabilities, detect_device_tier, get_device_capabilities};
pub use access::{AccessKind, AccessSet, ComponentAccess, any_conflicts};
