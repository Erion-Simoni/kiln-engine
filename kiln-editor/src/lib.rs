//! Kiln Editor — native desktop editor app.

pub mod app;
pub mod assets;
pub mod history;
pub mod panels;
pub mod prefab_editor;
pub mod scene_ops;
pub mod tilemap_editor;

pub use app::EditorApp;
pub use assets::{AssetBrowser, AssetEntry, AssetKind};
pub use history::History;
pub use prefab_editor::PrefabEditorState;
pub use scene_ops::{ComponentKind, Scene, SceneEntity};
pub use tilemap_editor::TilemapEditorState;
