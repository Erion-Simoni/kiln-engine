//! Kiln Editor — native desktop editor app.

pub mod app;
pub mod assets;
pub mod editing;
pub mod history;
pub mod hierarchy;
pub mod inspector;
pub mod panels;
pub mod prefab_editor;
pub mod scene_view;
pub mod tilemap_editor;

pub use app::{scene_from_world, EditorApp};
pub use assets::{AssetBrowser, AssetEntry, AssetKind};
pub use editing::{add_entity, next_entity_id, remove_entity, set_role, EntityDefaults};
pub use history::SceneHistory;
pub use prefab_editor::PrefabEditorState;
pub use scene_view::{SceneTexture, SceneTextures, ViewportRect};
pub use tilemap_editor::TilemapEditorState;
