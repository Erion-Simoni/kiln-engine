//! Build/Export panel — in-editor wrapper for `kiln-cli export`.

use egui::{Color32, ComboBox, DragValue, RichText, ScrollArea, Ui};
use std::path::PathBuf;

/// Build configuration for export.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Target platforms to build for.
    pub platforms: Vec<BuildPlatform>,
    /// Output directory.
    pub output_dir: PathBuf,
    /// Whether to create packaging (zip, .app, AppImage).
    pub packaging: bool,
    /// Whether to create signed builds (Android/iOS).
    pub signed: bool,
    /// Game name for packaging.
    pub game_name: String,
    /// Game version.
    pub game_version: String,
    /// Bundle identifier (macOS/iOS).
    pub bundle_id: String,
    /// Whether to show advanced options.
    pub show_advanced: bool,
}

/// Build platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPlatform {
    Web,
    Windows,
    MacOS,
    Linux,
    Android,
    IOS,
}

impl BuildPlatform {
    pub fn name(&self) -> &'static str {
        match self {
            BuildPlatform::Web => "Web (WASM)",
            BuildPlatform::Windows => "Windows (x64)",
            BuildPlatform::MacOS => "macOS (x64)",
            BuildPlatform::Linux => "Linux (x64)",
            BuildPlatform::Android => "Android (APK)",
            BuildPlatform::IOS => "iOS (Xcode project)",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Web,
            Self::Windows,
            Self::MacOS,
            Self::Linux,
            Self::Android,
            Self::IOS,
        ]
    }

    pub fn target_triple(&self) -> Option<&'static str> {
        match self {
            Self::Web => Some("wasm32-unknown-unknown"),
            Self::Windows => Some("x86_64-pc-windows-msvc"),
            Self::MacOS => Some("x86_64-apple-darwin"),
            Self::Linux => Some("x86_64-unknown-linux-gnu"),
            Self::Android => Some("aarch64-linux-android"),
            Self::IOS => Some("aarch64-apple-ios"),
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Windows => "kiln-player.exe",
            _ => "kiln-player",
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            platforms: vec![BuildPlatform::Web, BuildPlatform::Windows, BuildPlatform::MacOS, BuildPlatform::Linux],
            output_dir: PathBuf::from("export"),
            packaging: true,
            signed: false,
            game_name: "MyGame".to_string(),
            game_version: "0.1.0".to_string(),
            bundle_id: "com.kiln.game".to_string(),
            show_advanced: false,
        }
    }
}


/// State of the build panel.
#[derive(Debug, Default)]
pub struct BuildPanel {
    config: BuildConfig,
    building: bool,
    build_log: Vec<String>,
    last_error: Option<String>,
}

impl BuildPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the build panel.
    pub fn render(&mut self, ui: &mut Ui, project: &mut kiln_core::Project) -> Option<BuildConfig> {
        ui.heading("Build / Export");
        ui.add_space(8.0);

        // Update config from project
        if self.config.game_name != project.name {
            self.config.game_name = project.name.clone();
        }
        if self.config.game_version != project.version {
            self.config.game_version = project.version.clone();
        }
        if self.config.bundle_id == "com.kiln.game" {
            self.config.bundle_id = format!("com.kiln.{}", project.id);
        }

        // Platform selection
        ui.horizontal(|ui| {
            ui.label("Platforms:");
            for platform in BuildPlatform::all() {
                let mut selected = self.config.platforms.contains(&platform);
                if ui.checkbox(&mut selected, platform.name()).changed() {
                    if selected {
                        self.config.platforms.push(platform);
                    } else {
                        self.config.platforms.retain(|p| *p != platform);
                    }
                }
            }
        });

        ui.add_space(8.0);

        // Output directory
        ui.horizontal(|ui| {
            ui.label("Output directory:");
            ui.text_edit_singleline(&mut self.config.output_dir.to_string_lossy().to_string());
            if ui.button("Browse...").clicked() {
                // TODO: Native file dialog
            }
        });

        ui.add_space(8.0);

        // Basic options
        ui.checkbox(&mut self.config.packaging, "Create packaging (zip, .app, AppImage)");
        ui.checkbox(&mut self.config.signed, "Signed build (Android/iOS)");

        ui.add_space(8.0);

        // Advanced options
        ui.collapsing("Advanced", |ui| {
            ui.horizontal(|ui| {
                ui.label("Game name:");
                ui.text_edit_singleline(&mut self.config.game_name);
            });
            ui.horizontal(|ui| {
                ui.label("Game version:");
                ui.text_edit_singleline(&mut self.config.game_version);
            });
            ui.horizontal(|ui| {
                ui.label("Bundle identifier:");
                ui.text_edit_singleline(&mut self.config.bundle_id);
            });
        });

        ui.add_space(8.0);

        // Build button
        if !self.building {
            if ui.button(RichText::new("Build").size(18.0).strong()).clicked() {
                self.start_build();
            }
        } else {
            ui.add_enabled(false, egui::Button::new(RichText::new("Building...").size(18.0).strong()));
        }

        ui.add_space(8.0);

        // Build log
        if !self.build_log.is_empty() {
            ui.separator();
            ui.label("Build Log:");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for line in &self.build_log {
                    let color = if line.starts_with("ERROR:") { Color32::RED } else if line.starts_with("WARNING:") { Color32::YELLOW } else { Color32::WHITE };
                    ui.label(RichText::new(line).color(color).monospace());
                }
            });
        }

        if let Some(error) = &self.last_error {
            ui.separator();
            ui.label(RichText::new(format!("Error: {}", error)).color(Color32::RED));
        }

        None
    }

    fn start_build(&mut self) {
        self.building = true;
        self.build_log.clear();
        self.last_error = None;
        
        self.build_log.push("Starting build...".to_string());
        self.build_log.push(format!("Platforms: {:?}", self.config.platforms));
        self.build_log.push(format!("Output: {}", self.config.output_dir.display()));
        
        // TODO: Actually run the build (could use std::process::Command or async)
        // For now, just simulate
        self.build_log.push("Build completed!".to_string());
        self.building = false;
    }
}


