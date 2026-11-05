//! Kiln core crate
//! Math, time, shared types, ECS glue

pub mod access;
pub mod audio;
pub mod camera;
pub mod clip;
pub mod device_tier;
pub mod hierarchy_transform;
pub mod id;
pub mod input;
pub mod math;
pub mod mesh;
pub mod paths;
pub mod physics_data;
pub mod prefab;
pub mod project;
pub mod tilemap;
pub mod time;
pub mod transform;
pub mod ui;

pub use access::{any_conflicts, AccessKind, AccessSet, ComponentAccess};
pub use clip::{Clip, ClipError, Frame, FrameEvent};
pub use device_tier::{
    detect_device_tier, get_device_capabilities, DeviceCapabilities, DeviceTier,
};
pub use id::*;
pub use input::{Action, ActionMap, Binding, BindingType};
pub use math::*;
pub use mesh::{
    AlphaMode, Material, Mesh, MeshBounds, MeshHandle, Model, PrimitiveTopology, SubMesh, Vertex,
    VertexFormat,
};
pub use paths::{cache_dir, save_dir, settings_dir};
pub use prefab::{ComponentOverride, Prefab, PrefabInstance, PrefabOverride};
pub use project::SpriteMaterial;
pub use project::{
    AndroidSettings, IosSettings, LaunchScreen, MaterialParam, MobileSettings, Orientation,
    PrivacyDescriptions, Shader,
};
pub use project::{Component, Entity, NestedScene, Project, Scene, SceneOffset, WindowSettings};
pub use tilemap::{TileChunk, TileMap, TileMapLayer, TileSet, CHUNK_SIZE, EMPTY_TILE};
pub use time::*;
pub use transform::*;

pub mod vfx;
pub mod visual;
