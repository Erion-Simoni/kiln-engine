//! Plugin Manager panel — manage and configure WASM plugins.

use dirs;
use kiln_plugin_api::PluginRegistry;
use std::collections::HashMap;
use std::path::PathBuf;

/// State of the Plugin Manager panel.
#[derive(Default)]
pub struct PluginManagerPanel {
    /// Registry of loaded plugins.
    registry: PluginRegistry,
    /// Path to the plugins directory.
    plugins_dir: PathBuf,
    /// Whether to show the install dialog.
    show_install_dialog: bool,
    /// Path to the plugin WASM file to install.
    install_path: String,
    /// Search filter for plugins.
    filter: String,
    /// Whether to show the settings dialog for a plugin.
    show_settings: Option<String>,
    /// Error message.
    error: Option<String>,
    configs: HashMap<String, kiln_plugin_api::PluginConfig>,
}

impl PluginManagerPanel {
    pub fn new() -> Self {
        let panel = Self {
            registry: PluginRegistry::default(),
            plugins_dir: dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("kiln")
                .join("plugins"),
            show_install_dialog: false,
            install_path: String::new(),
            filter: String::new(),
            show_settings: None,
            error: None,
            configs: HashMap::new(),
        };
        // Create plugins directory if it doesn't exist
        let _ = std::fs::create_dir_all(&panel.plugins_dir);
        panel
    }

    /// Load all plugins from the plugins directory.
    pub fn load_plugins(&mut self) {
        // TODO: Scan plugins directory for .wasm files and load them
        // For now, we just initialize the registry
    }

    /// Initialize all loaded plugins.
    pub async fn initialize_plugins(&mut self, context: &mut kiln_plugin_api::PluginContext) {
        if let Err(e) = self.registry.initialize_all(context).await {
            self.error = Some(e.to_string());
        }
    }

    /// Shutdown all plugins.
    pub async fn shutdown_plugins(&mut self) {
        if let Err(e) = self.registry.shutdown_all().await {
            eprintln!("Error shutting down plugins: {}", e);
        }
    }

    /// Send an event to all plugins.
    pub fn handle_event(
        &mut self,
        event: &kiln_plugin_api::EditorEvent,
        context: &mut kiln_plugin_api::PluginContext,
    ) {
        self.registry.handle_event_all(event, context);
    }

    /// Render the Plugin Manager panel.
    pub fn render(&mut self, ui: &mut egui::Ui, _context: &mut kiln_plugin_api::PluginContext) {
        ui.heading("Plugin Manager");
        ui.add_space(8.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ Install Plugin").clicked() {
                self.show_install_dialog = true;
                self.install_path.clear();
            }
            if ui.button("🔄 Reload All").clicked() {
                // TODO: Reload all plugins
            }
            ui.separator();
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Plugins: {}", self.registry.plugins.len()));
            });
        });

        ui.add_space(8.0);

        // Error display
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            ui.add_space(4.0);
        }

        // Installed plugins list
        ui.separator();
        ui.heading("Installed Plugins");
        ui.add_space(4.0);

        let manifests: Vec<_> = self.registry.manifests().into_iter().cloned().collect();
        let filtered: Vec<_> = manifests
            .iter()
            .filter(|m| {
                self.filter.is_empty()
                    || m.name.to_lowercase().contains(&self.filter.to_lowercase())
                    || m.id.to_lowercase().contains(&self.filter.to_lowercase())
            })
            .collect();

        if manifests.is_empty() {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label("No plugins installed.");
                ui.add_space(8.0);
                if ui.button("📦 Install Plugin").clicked() {
                    // Would show install dialog
                }
            });
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    egui::Grid::new("plugins_grid")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Plugin").strong());
                            ui.label(egui::RichText::new("Version").strong());
                            ui.label(egui::RichText::new("Status").strong());
                            ui.label(egui::RichText::new("Capabilities").strong());
                            ui.label(egui::RichText::new("Actions").strong());
                            ui.end_row();

                            for manifest in filtered {
                                let _is_enabled = self.is_plugin_enabled(&manifest.id);
                                let _status_text = if self.is_plugin_loaded(&manifest.id) {
                                    "Loaded"
                                } else {
                                    "Not Loaded"
                                };
                                let _status_color = if self.is_plugin_loaded(&manifest.id) {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::GRAY
                                };

                                ui.horizontal(|ui| {
                                    let mut enabled = self.is_plugin_enabled(&manifest.id);
                                    if ui.checkbox(&mut enabled, "").changed() {
                                        self.get_config_mut(&manifest.id).enabled = enabled;
                                    }
                                    ui.label(&manifest.name);
                                    ui.label(&manifest.version);
                                    ui.colored_label(
                                        if self.is_plugin_loaded(&manifest.id) {
                                            egui::Color32::GREEN
                                        } else {
                                            egui::Color32::GRAY
                                        },
                                        self.get_status_text(&manifest.id),
                                    );
                                    ui.label(
                                        manifest
                                            .capabilities
                                            .iter()
                                            .map(|c| format!("{:?}", c))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    );
                                    ui.horizontal(|ui| {
                                        if ui.small_button("⚙ Settings").clicked() {
                                            self.show_settings = Some(manifest.id.clone());
                                        }
                                        if ui.small_button("🗑 Uninstall").clicked() {
                                            // TODO: Uninstall
                                        }
                                    });
                                });
                                ui.end_row();
                            }
                        });
                });
        }

        ui.add_space(8.0);

        ui.add_space(8.0);

        // Install plugin dialog
        if self.show_install_dialog {
            egui::Window::new("Install Plugin")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Plugin file (.wasm):");
                    ui.text_edit_singleline(&mut self.install_path);
                    ui.horizontal(|ui| {
                        if ui.button("Browse...").clicked() {
                            // TODO: Native file dialog
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_install_dialog = false;
                        }
                        if ui.button("Install").clicked() {
                            // TODO: Install plugin
                            self.show_install_dialog = false;
                        }
                    });
                });
        }
    }

    // Helper methods
    fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        self.configs
            .get(plugin_id)
            .map(|config| config.enabled)
            .unwrap_or(true)
    }

    fn is_plugin_loaded(&self, plugin_id: &str) -> bool {
        self.registry.plugins.contains_key(plugin_id)
    }

    fn get_config_mut(&mut self, plugin_id: &str) -> &mut kiln_plugin_api::PluginConfig {
        self.configs.entry(plugin_id.to_string()).or_default()
    }
}

impl PluginManagerPanel {
    pub fn get_status_text(&self, plugin_id: &str) -> String {
        if self.registry.plugins.contains_key(plugin_id) {
            "Loaded".to_string()
        } else {
            "Not loaded".to_string()
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn plugin_enable_state_reads_the_saved_config() {
        let mut panel = PluginManagerPanel::default();
        assert!(panel.is_plugin_enabled("first"));
        panel.get_config_mut("first").enabled = false;
        assert!(!panel.is_plugin_enabled("first"));
        assert!(panel.is_plugin_enabled("second"));
    }

    #[test]
    fn plugin_settings_are_kept_separate() {
        let mut panel = PluginManagerPanel::default();
        panel.get_config_mut("first").enabled = true;
        assert!(!panel.get_config_mut("second").enabled);
        assert!(panel.get_config_mut("first").enabled);
    }
}
