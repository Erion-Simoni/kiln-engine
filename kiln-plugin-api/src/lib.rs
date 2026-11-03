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
pub const PLUGIN_API_VERSION: u32 = 2;

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
    pub ui: Option<kiln_ui::UiRenderer>,
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
    fn ui(&mut self, ui: &mut kiln_ui::UiRenderer, context: &mut PluginContext) -> bool;

    /// Handle editor events (keyboard, mouse, etc.)
    fn handle_event(&mut self, event: &EditorEvent, context: &mut PluginContext) -> bool;

    /// Called when a project is opened/closed.
    fn on_project_change(&mut self, _project: Option<&kiln_core::Project>) {}

    /// Get custom configuration UI for the plugin settings.
    fn settings_ui(&mut self, _ui: &mut kiln_ui::UiRenderer) {}
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
    Custom {
        name: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Plugin entry point returning opaque plugin-owned storage.
/// The host may only pass this pointer to the matching destroy entry point.
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut std::ffi::c_void;

/// Entry point that destroys storage returned by the matching plugin creator.
pub type PluginDestroyFn = unsafe extern "C" fn(*mut std::ffi::c_void);

/// Allocate an opaque handle without exposing Rust trait-object layout in C.
pub fn plugin_into_handle(plugin: Box<dyn EditorPlugin>) -> *mut std::ffi::c_void {
    Box::into_raw(Box::new(plugin)).cast()
}

/// Release a plugin-owned handle. A null handle is a no-op.
///
/// # Safety
/// A non-null handle must have been created by this module's plugin_into_handle,
/// must not have been freed, and must not be used again after this call.
pub unsafe fn destroy_plugin_handle(handle: *mut std::ffi::c_void) {
    if !handle.is_null() {
        // SAFETY: the caller guarantees the allocation's origin and lifetime.
        unsafe {
            drop(Box::from_raw(handle.cast::<Box<dyn EditorPlugin>>()));
        }
    }
}

/// Plugin registry for managing loaded plugins.
#[derive(Default)]
pub struct PluginRegistry {
    pub plugins: std::collections::HashMap<String, Box<dyn EditorPlugin>>,
    manifests: std::collections::HashMap<String, PluginManifest>,
    load_order: Vec<String>,
}

impl PluginRegistry {
    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn EditorPlugin>) -> Result<(), PluginError> {
        let manifest = plugin.manifest().clone();
        let id = manifest.id.clone();

        if self.plugins.contains_key(&id) {
            return Err(crate::PluginError::InitializationFailed(format!(
                "Plugin with id '{}' already registered",
                id
            )));
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
    pub fn get(&self, id: &str) -> Option<&(dyn EditorPlugin + '_)> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    /// Get mutable reference to a plugin.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut (dyn EditorPlugin + '_)> {
        match self.plugins.get_mut(id) {
            Some(plugin) => Some(plugin.as_mut()),
            None => None,
        }
    }

    /// Get all registered plugin manifests.
    /// Get all registered plugin manifests.
    pub fn manifests(&self) -> Vec<&PluginManifest> {
        self.manifests.values().collect()
    }

    /// Get plugins in load order.
    pub fn plugins_in_order(&self) -> Vec<&(dyn EditorPlugin + '_)> {
        self.load_order
            .iter()
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
    pub fn ui_all(&mut self, ui: &mut kiln_ui::UiRenderer, context: &mut PluginContext) {
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

/// Declare a plugin's opaque create/destroy entry points.
/// The plugin type must implement EditorPlugin and Default. Callers must release
/// each created handle using kiln_plugin_destroy in the same plugin module.
#[macro_export]
macro_rules! kiln_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn kiln_plugin_create() -> *mut ::std::ffi::c_void {
            let plugin: Box<dyn $crate::EditorPlugin> = Box::new(<$plugin_type>::default());
            $crate::plugin_into_handle(plugin)
        }

        /// Release an opaque plugin handle.
        ///
        /// # Safety
        /// The handle must be null or an unreleased value from kiln_plugin_create
        /// in this same module. It must not be used again after this call.
        #[no_mangle]
        pub unsafe extern "C" fn kiln_plugin_destroy(handle: *mut ::std::ffi::c_void) {
            // SAFETY: the exported function forwards the caller's contract.
            unsafe {
                $crate::destroy_plugin_handle(handle);
            }
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

#[cfg(test)]
mod abi_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    struct TestPlugin {
        manifest: PluginManifest,
    }

    impl Default for TestPlugin {
        fn default() -> Self {
            Self {
                manifest: PluginManifest {
                    id: "test.opaque-handle".into(),
                    name: "ABI regression".into(),
                    version: "0.1.0".into(),
                    description: String::new(),
                    author: String::new(),
                    min_kiln_version: "0.1.0".into(),
                    api_version: PLUGIN_API_VERSION,
                    entry_point: "kiln_plugin_create".into(),
                    capabilities: Vec::new(),
                    config_schema: None,
                },
            }
        }
    }

    impl Drop for TestPlugin {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait::async_trait]
    impl EditorPlugin for TestPlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        async fn initialize(&mut self, _context: &mut PluginContext) -> Result<(), PluginError> {
            Ok(())
        }
        async fn shutdown(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
        fn ui(&mut self, _ui: &mut kiln_ui::UiRenderer, _context: &mut PluginContext) -> bool {
            false
        }
        fn handle_event(&mut self, _event: &EditorEvent, _context: &mut PluginContext) -> bool {
            false
        }
    }

    crate::kiln_plugin!(TestPlugin);

    #[test]
    fn exported_plugin_handle_uses_matching_abi_and_drops_exactly_once() {
        let create: PluginCreateFn = kiln_plugin_create;
        let destroy: PluginDestroyFn = kiln_plugin_destroy;
        let before = DROPS.load(Ordering::SeqCst);
        // SAFETY: these pointers are the matching entry points in this module.
        let handle = unsafe { create() };
        assert!(!handle.is_null());
        assert_eq!(DROPS.load(Ordering::SeqCst), before);
        // SAFETY: null is allowed and handle is the unreleased value from create.
        unsafe {
            destroy(std::ptr::null_mut());
            destroy(handle);
        }
        assert_eq!(DROPS.load(Ordering::SeqCst), before + 1);
    }
}
