//! Project Hub panel — create, open, and manage projects.

use egui::{Color32, RichText, ScrollArea, Ui, Vec2};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A recent project entry.
#[derive(Debug, Clone)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: u64, // Unix timestamp
}

/// State of the project hub panel.
#[derive(Debug, Default)]
pub struct ProjectHubState {
    /// List of recent projects.
    pub recent_projects: Vec<RecentProject>,
    /// Whether to show the new project dialog.
    pub show_new_project_dialog: bool,
    /// New project name.
    pub new_project_name: String,
    /// New project directory.
    pub new_project_dir: PathBuf,
    /// New project template.
    pub new_project_template: String,
    /// Whether to show project settings.
    pub show_project_settings: bool,
    /// Current project being edited (for settings).
    pub current_project_path: Option<PathBuf>,
}

impl ProjectHubState {
    pub fn new() -> Self {
        Self {
            recent_projects: Self::load_recent(),
            ..Default::default()
        }
    }

    fn load_recent() -> Vec<RecentProject> {
        // TODO: Load from config file
        Vec::new()
    }

    fn save_recent(&self) {
        // TODO: Save to config file
    }

    /// Add a project to recent list.
    pub fn add_recent(&mut self, path: PathBuf, name: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove if already exists
        self.recent_projects.retain(|p| p.path != path);

        // Add to front
        self.recent_projects.insert(0, RecentProject {
            path: path.clone(),
            name,
            last_opened: now,
        });

        // Keep only last 10
        if self.recent_projects.len() > 10 {
            self.recent_projects.truncate(10);
        }

        self.save_recent();
    }

    /// Remove a project from recent list.
    pub fn remove_recent(&mut self, path: &Path) {
        self.recent_projects.retain(|p| p.path != path);
        self.save_recent();
    }

    /// Get available templates.
    pub fn templates() -> Vec<(&'static str, &'static str)> {
        vec![
            ("empty_2d", "Empty 2D Project"),
            ("platformer", "Platformer Template"),
            ("topdown", "Top-Down Template"),
        ]
    }
}


/// Render the project hub panel (welcome screen / project manager).
pub fn render_project_hub(
    ui: &mut Ui,
    state: &mut ProjectHubState,
    open_project_callback: &mut dyn FnMut(PathBuf),
    create_project_callback: &mut dyn FnMut(String, PathBuf, String),
) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.heading(RichText::new("Kiln Engine").size(48.0).strong());
        ui.label(RichText::new("Lightweight 2D game engine").size(18.0).color(Color32::GRAY));
        ui.add_space(40.0);
    });

    ui.separator();
    ui.add_space(20.0);

    // Main action buttons
    ui.columns(2, |cols| {
        // New Project
        cols[0].vertical_centered(|ui| {
            ui.add_space(20.0);
            if ui
                .add_sized([200.0, 120.0], egui::Button::new(
                    RichText::new("📄\n\nNew Project").size(24.0).strong()
                ))
                .clicked()
            {
                state.show_new_project_dialog = true;
                state.new_project_name.clear();
                state.new_project_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                state.new_project_template = "empty_2d".to_string();
            }
            ui.add_space(8.0);
            ui.label("Create a new Kiln project");
        });

        // Open Project
        cols[1].vertical_centered(|ui| {
            ui.add_space(20.0);
            if ui
                .add_sized([200.0, 120.0], egui::Button::new(
                    RichText::new("📁\n\nOpen Project").size(24.0).strong()
                ))
                .clicked()
            {
                // TODO: Native file dialog
                if let Some(path) = native_file_dialog::open_file_dialog() {
                    if path.exists() {
                        state.add_recent(path.clone(), path.file_name().unwrap_or_default().to_string_lossy().to_string());
                        open_project_callback(path);
                    }
                }
            }
            ui.add_space(8.0);
            ui.label("Open an existing project");
        });
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    // Recent projects
    if !state.recent_projects.is_empty() {
        ui.heading("Recent Projects");
        ui.add_space(8.0);

        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for project in &state.recent_projects {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&project.name).size(16.0).strong());
                        ui.label(RichText::new(project.path.display().to_string()).size(12.0).color(Color32::GRAY));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Open").clicked() {
                            state.add_recent(path.clone(), project.name.clone());
                            open_project_callback(project.path.clone());
                        }
                        if ui.small_button("Remove").clicked() {
                            state.remove_recent(&project.path);
                        }
                    });
                });
                ui.separator();
            }
        });

        ui.add_space(16.0);
    }

    // New project dialog
    if state.show_new_project_dialog {
        egui::Window::new("New Project")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.label("Project name:");
                ui.text_edit_singleline(&mut state.new_project_name);

                ui.add_space(8.0);
                ui.label("Location:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut state.new_project_dir.to_string_lossy().to_string());
                    if ui.button("Browse...").clicked() {
                        // TODO: Native folder dialog
                    }
                });

                ui.add_space(8.0);
                ui.label("Template:");
                egui::ComboBox::from_label("")
                    .selected_text(
                        ProjectHubState::templates()
                            .iter()
                            .find(|(id, _)| *id == state.new_project_template)
                            .map(|(_, name)| *name)
                            .unwrap_or("Empty 2D Project"),
                    )
                    .show_ui(ui, |ui| {
                        for (id, name) in ProjectHubState::templates() {
                            ui.selectable_value(&mut state.new_project_template, id.to_string(), name);
                        }
                    });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.show_new_project_dialog = false;
                    }
                    if ui.button("Create").clicked() {
                        if !state.new_project_name.is_empty() && state.new_project_dir.exists() {
                            let path = state.new_project_dir.join(&state.new_project_name);
                            create_project_callback(
                                state.new_project_name.clone(),
                                path,
                                state.new_project_template.clone(),
                            );
                            state.show_new_project_dialog = false;
                        }
                    }
                });
            });
    }
}

/// Native file dialog stub (replace with actual implementation).
mod native_file_dialog {
    use std::path::PathBuf;
    pub fn open_file_dialog() -> Option<PathBuf> {
        // TODO: Use rfd or native dialogs
        None
    }
}


