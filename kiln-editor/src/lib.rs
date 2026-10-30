//! Kiln Editor — native desktop editor app.

pub mod app;
pub mod assets;
pub mod editing;
pub mod history;
pub mod hierarchy;
pub mod inspector;
pub mod graph_editor;
pub mod panels;
pub mod prefab_editor;
pub mod scene_view;
pub mod tilemap_editor;
pub mod anim_editor;
pub mod build_panel;
pub mod project_hub;
pub mod profiler_panel;
pub mod debugger_panel;
pub mod vfx_editor;
pub mod audio_editor;
pub mod physics_editor;
pub mod ui_layout_editor;

pub use app::{scene_from_world, EditorApp};
pub use assets::{AssetBrowser, AssetEntry, AssetKind};
pub use editing::{add_entity, next_entity_id, remove_entity, set_role, EntityDefaults, add_role, remove_role, add_tilemap, remove_tilemap, add_transform, add_sprite, remove_transform, remove_sprite};
pub use history::SceneHistory;
pub use prefab_editor::PrefabEditorState;
pub use scene_view::{SceneTexture, SceneTextures, ViewportRect};
pub use tilemap_editor::TilemapEditorState;
pub use anim_editor::{AnimationEditorState, render_animation_editor};
pub use build_panel::{BuildPanel, BuildConfig, BuildPlatform};
pub use project_hub::{ProjectHubState, render_project_hub};
pub use profiler_panel::ProfilerPanel;
pub use debugger_panel::DebuggerPanel;
pub use vfx_editor::VfxEditorState;
pub use audio_editor::AudioEditorState;
pub use physics_editor::PhysicsEditorState;
pub use ui_layout_editor::UiLayoutEditorState;
