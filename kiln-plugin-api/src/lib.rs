//! Kiln Plugin API — interfaces for WASM plugins.
//!
//! This crate defines the interfaces that WASM plugins must implement
//! to extend the Kiln Engine editor and runtime.
//!
//! Plugins can provide:
//! - Custom editor panels
//! - Custom node types for the visual graph editor
//! - Custom asset importers
//! - Custom render passes
//! - Custom KilnScript host functions

use kiln_core::Project;
use kiln_runtime::{GameWorld, SliceSession};
use kiln_ui::Ui;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during plugin operations.
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),
    #[error("Plugin API version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Current plugin API version.
/// Plugins must declare compatibility with this version.
pub const PLUGIN_API_VERSION: u32 = 1;

/// Plugin manifest - metadata about a plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (e.g., "com.example.myplugin")
    pub id: String,
    /// Human-readable plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Author information
    pub author: String,
    /// Minimum Kiln Engine version required
    pub min_kiln_version: String,
    /// Plugin API version this plugin was built against
    pub api_version: u32,
    /// Plugin entry point (WASM export name)
    pub entry_point: String,
    /// Required permissions/capabilities
    pub capabilities: Vec<PluginCapability>,
    /// Custom configuration schema (JSON schema)
    pub config_schema: Option<serde_json::Value>,
}

/// Capabilities a plugin can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u32)]
pub enum PluginCapability {
    /// Access to editor UI and panels
    EditorUI = 1,
    /// Access to asset import/export
    AssetIO = 2,
    /// Access to scene graph and entities
    SceneAccess = 3,
    /// Access to KilnScript VM and host functions
    Scripting = 4,
    /// Access to render pipeline
    Rendering = 5,
    /// Access to physics simulation
    Physics = 6,
    /// Access to audio system
    Audio = 7,
    /// Access to network/multiplayer
    Network = 8,
    /// File system access (sandboxed)
    FileSystem = 9,
    /// Custom native code execution
    NativeCode = 10,
}

/// Plugin configuration provided by the user.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PluginConfig {
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Plugin-specific configuration (JSON)
    pub settings: serde_json::Value,
}

/// Context provided to plugins during initialization and runtime.
pub struct PluginContext {
    /// Project being edited
    pub project: Option<kiln_core::Project>,
    /// Editor UI instance
    pub ui: Option<kiln_ui::Ui>,
    /// Game world (for runtime plugins)
    pub world: Option<kiln_runtime::GameWorld>,
    /// Script session (for script-related plugins)
    pub slice_session: Option<kiln_runtime::SliceSession>,
    /// Plugin-specific configuration
    pub config: PluginConfig,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Temporary directory for plugin use
    pub temp_dir: std::path::PathBuf,
}

/// Trait that all editor plugins must implement.
#[async_trait::async_trait]
pub trait EditorPlugin: Send + Sync {
    /// Returns the plugin manifest.
    fn manifest(&self) -> &PluginManifest;

    /// Initialize the plugin.
    /// Called once when the plugin is loaded.
    async fn initialize(&mut self, context: &mut PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin is unloaded.
    async fn shutdown(&mut self) -> Result<(), PluginError>;

    /// Called each frame in the editor.
    /// Return true if the plugin wants to render UI.
    fn ui(&mut self, ui: &mut kiln_ui::Ui, context: &mut PluginContext) -> bool;

    /// Handle editor events (keyboard, mouse, etc.)
    fn handle_event(&mut self, event: &EditorEvent, context: &mut PluginContext) -> bool;

    /// Called when a project is opened/closed.
    fn on_project_change(&mut self, project: Option<&kiln_core::Project>) {}

    /// Get custom configuration UI for the plugin settings.
    fn settings_ui(&mut self, ui: &mut kiln_ui::Ui) {}
}

/// Events that can be sent to plugins.
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// Key pressed
    KeyPressed { key: String, modifiers: Vec<String> },
    /// Key released
    KeyReleased { key: String },
    /// Mouse moved
    MouseMoved { x: f32, y: f32 },
    /// Mouse clicked
    MouseClicked { button: MouseButton, x: f32, y: f32 },
    /// File dropped
    FileDropped { paths: Vec<std::path::PathBuf> },
    /// Project saved
    ProjectSaved,
    /// Scene changed
    SceneChanged { scene_id: String },
    /// Entity selected
    EntitySelected { entity_id: String },
    /// Custom event
    Custom { name: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}


/// Plugin entry point that must be exported by the WASM module.
/// 
/// The plugin must export a function with this signature:
/// ```rust
/// #[no_mangle]
/// pub extern "C" fn kiln_plugin_create() -> *mut dyn EditorPlugin
/// ```
/// 
/// The returned pointer will be managed by the engine.
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut dyn EditorPlugin;

/// Plugin registry for managing loaded plugins.
pub struct PluginRegistry {
    plugins: std::collections::HashMap<String, Box<dyn EditorPlugin>>,
    manifests: std::collections::HashMap<String, PluginManifest>,
    load_order: Vec<String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
            manifests: std::collections::HashMap::new(),
            load_order: Vec::new(),
        }
    }
}

impl PluginRegistry {
    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn EditorPlugin>) -> Result<(), PluginError> {
        let manifest = plugin.manifest().clone();
        let id = manifest.id.clone();
        
        if self.plugins.contains_key(&id) {
            return Err(crate::PluginError::InitializationFailed(
                format!("Plugin with id '{}' already registered", id)
            ));
        }
        
        // Check API version compatibility
        if manifest.api_version != PLUGIN_API_VERSION {
            return Err(PluginError::VersionMismatch {
                expected: PLUGIN_API_VERSION,
                actual: manifest.api_version,
            });
        }
        
        self.plugins.insert(id.clone(), plugin);
        self.manifests.insert(id.clone(), manifest);
        self.load_order.push(id);
        Ok(())
    }
    
    /// Unregister a plugin.
    pub fn unregister(&mut self, id: &str) -> Option<Box<dyn EditorPlugin>> {
        self.manifests.remove(id);
        self.load_order.retain(|x| x != id);
        self.plugins.remove(id)
    }
    
    /// Get a plugin by ID.
    pub fn get(&self, id: &str) -> Option<&dyn EditorPlugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }
    
    /// Get mutable reference to a plugin.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut dyn EditorPlugin> {
        self.plugins.get_mut(id).map(|p| p.as_mut())
    }
    
    /// Get all registered plugin manifests.
    pub fn manifests(&self) -> Vec<&PluginManifest> {
        self.manifests.values().collect()
    }
    
    /// Get plugins in load order.
    pub fn plugins_in_order(&self) -> Vec<&dyn EditorPlugin> {
        self.load_order.iter()
            .filter_map(|id| self.plugins.get(id).map(|p| p.as_ref()))
            .collect()
    }
    
    /// Initialize all plugins.
    pub async fn initialize_all(&mut self, context: &mut PluginContext) -> Result<(), PluginError> {
        for id in &self.load_order.clone() {
            if let Some(plugin) = self.plugins.get_mut(id) {
                plugin.initialize(context).await?;
            }
        }
        Ok(())
    }
    
    /// Shutdown all plugins.
    pub async fn shutdown_all(&mut self) -> Result<(), PluginError> {
        for id in self.load_order.iter().rev() {
            if let Some(plugin) = self.plugins.get_mut(id) {
                plugin.shutdown().await?;
            }
        }
        Ok(())
    }
    
    /// Call ui on all plugins.
    pub fn ui_all(&mut self, ui: &mut kiln_ui::Ui, context: &mut PluginContext) {
        for id in &self.load_order {
            if let Some(plugin) = self.plugins.get_mut(id) {
                plugin.ui(ui, context);
            }
        }
    }
    
    /// Send event to all plugins.
    pub fn handle_event_all(&mut self, event: &EditorEvent, context: &mut PluginContext) {
        for id in &self.load_order {
            if let Some(plugin) = self.plugins.get_mut(id) {
                plugin.handle_event(event, context);
            }
        }
    }
}

/// Macro for declaring a plugin's entry point.
/// 
/// Usage:
/// ```rust
/// kiln_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! kiln_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn kiln_plugin_create() -> *mut dyn EditorPlugin {
            let plugin: Box<dyn EditorPlugin> = Box::new(<$plugin_type>::default());
            Box::into_raw(Box::new(plugin))
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_manifest_serialization() {
        let manifest = PluginManifest {
            id: "test.plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            min_kiln_version: "0.1.0".to_string(),
            api_version: PLUGIN_API_VERSION,
            entry_point: "kiln_plugin_create".to_string(),
            capabilities: vec![PluginCapability::EditorUI],
            config_schema: None,
        };
        
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest.id, deserialized.id);
    }
}


