//! Kiln Editor — native desktop editor app.

pub mod anim_editor;
pub mod app;
mod asset_shelf;
pub mod assets;
pub mod audio_editor;
pub mod build_panel;
pub mod component_command;
pub mod debugger_panel;
pub mod editing;
pub mod editor_shell;
mod entity_clipboard;
pub mod entity_command;
pub mod graph_editor;
pub mod hierarchy;
pub mod history;
pub mod input_rebind_panel;
pub mod inspector;
pub mod panels;
pub mod physics_editor;
pub mod physics_inspector;
pub mod play_changes;
pub mod plugin_manager_panel;
pub mod prefab_editor;
pub mod prefab_editor_panel;
pub mod preferences;
pub mod preview_3d;
pub mod profiler_panel;
pub mod project_files;
pub mod project_hub;
pub mod project_lifecycle;
pub mod property_edit;
mod quick_actions;
pub mod recovery;
pub mod scene_document;
pub mod scene_view;
pub mod scene_workspace;
pub mod script_editor;
pub mod shader_editor;
pub mod sprite_editor;
pub mod tilemap_editor;
pub mod transform_edit;
pub mod transform_gizmo;
pub mod ui_layout_editor;
pub mod version_control_panel;
pub mod vfx_editor;

pub use anim_editor::AnimationEditorState;
pub use app::{scene_from_world, EditorApp};
pub use assets::{AssetBrowser, AssetEntry, AssetKind};
pub use audio_editor::AudioEditorState;
pub use build_panel::{BuildConfig, BuildPanel, BuildTarget};
pub use debugger_panel::DebuggerPanel;
pub use editing::{
    add_entity, add_role, add_sprite, add_tilemap, add_transform, next_entity_id, remove_entity,
    remove_role, remove_sprite, remove_tilemap, remove_transform, set_role, EntityDefaults,
};
pub use history::SceneHistory;
pub use input_rebind_panel::InputRebindPanel;
pub use physics_editor::PhysicsEditorState;
pub use prefab_editor::PrefabEditorState;
pub use prefab_editor_panel::PrefabEditorPanel;
pub use preferences::PreferencesPanel;
pub use profiler_panel::ProfilerPanel;
pub use project_hub::{render_project_hub, ProjectHubState};
pub use scene_view::{SceneTexture, SceneTextures, ViewportRect};
pub use sprite_editor::SpriteEditorState;
pub use tilemap_editor::TilemapEditorState;
pub use version_control_panel::VersionControlPanel;
pub use vfx_editor::VfxEditorState;

pub mod image_preview;

mod script_highlight;

mod scene_controls;
mod scene_selection;
