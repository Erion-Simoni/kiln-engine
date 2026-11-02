//! Version Control panel — Git integration for project files.

use egui::Ui;

// ---------------------------------------------------------------------------
// Stub: no Git support (default build). The `git` feature requires the system
// OpenSSL development headers (libssl-dev) to compile git2/libgit2-sys.
// ---------------------------------------------------------------------------
#[cfg(not(feature = "git"))]
pub struct VersionControlPanel;

#[cfg(not(feature = "git"))]
impl VersionControlPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, _ui: &egui::Ui) {}
}

// ---------------------------------------------------------------------------
// Full Git implementation — only compiled with `cargo ... --features git`.
// ---------------------------------------------------------------------------
#[cfg(feature = "git")]
use egui::{Color32, RichText, ScrollArea};

#[cfg(feature = "git")]
use git2::{Repository, Status, StatusOptions};

#[cfg(feature = "git")]
use std::collections::HashMap;

#[cfg(feature = "git")]
use std::path::{Path, PathBuf};

/// State of the Version Control panel.
#[cfg(feature = "git")]
#[derive(Debug, Default)]
pub struct VersionControlPanel {
    repo_path: Option<PathBuf>,
    statuses: Vec<FileStatus>,
    refreshing: bool,
    error: Option<String>,
    commit_message: String,
    show_commit_dialog: bool,
    branch_name: String,
    show_checkout_dialog: bool,
    remote_url: String,
    show_remote_dialog: bool,
}

/// Status of a file in the repository.
#[cfg(feature = "git")]
#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: git2::Status,
    pub staged: bool,
}

#[cfg(feature = "git")]
impl VersionControlPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_repo(&mut self, path: &Path) -> Result<(), String> {
        match Repository::open(path) {
            Ok(_) => {
                self.repo_path = Some(path.to_path_buf());
                self.refresh_status();
                Ok(())
            }
            Err(e) => Err(format!("Failed to open repository: {}", e)),
        }
    }

    fn refresh_status(&mut self) {
        self.statuses.clear();
        self.error = None;

        let Some(repo_path) = &self.repo_path else {
            return;
        };

        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => {
                self.error = Some(format!("Failed to open repo: {}", e));
                return;
            }
        };

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .include_unmodified(false)
            .recurse_untracked_dirs(true);

        let statuses = match repo.statuses(Some(&mut opts)) {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(format!("Failed to get status: {}", e));
                return;
            }
        };

        for entry in statuses.iter() {
            let path = entry
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            let status = entry.status();
            let staged = status.contains(git2::Status::INDEX_NEW)
                || status.contains(git2::Status::INDEX_MODIFIED)
                || status.contains(git2::Status::INDEX_DELETED)
                || status.contains(git2::Status::INDEX_RENAMED)
                || status.contains(git2::Status::INDEX_TYPECHANGE);

            self.statuses.push(FileStatus { path, status, staged });
        }
    }

    pub fn stage_file(&mut self, path: &str) -> Result<(), String> {
        let Some(repo_path) = &self.repo_path else {
            return Err("No repository open".to_string());
        };
        let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index.add_path(Path::new(path)).map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
        self.refresh_status();
        Ok(())
    }

    pub fn unstage_file(&mut self, path: &str) -> Result<(), String> {
        let Some(repo_path) = &self.repo_path else {
            return Err("No repository open".to_string());
        };
        let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index.remove_path(Path::new(path)).map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
        self.refresh_status();
        Ok(())
    }

    pub fn commit(&mut self, message: &str) -> Result<(), String> {
        if message.trim().is_empty() {
            return Err("Commit message cannot be empty".to_string());
        }
        let Some(repo_path) = &self.repo_path else {
            return Err("No repository open".to_string());
        };
        let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        let tree_id = index.write_tree().map_err(|e| e.to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
        let signature = repo.signature().map_err(|e| e.to_string())?;
        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .map_err(|e| e.to_string())?;
        self.refresh_status();
        Ok(())
    }

    pub fn checkout_branch(&mut self, branch_name: &str) -> Result<(), String> {
        let Some(repo_path) = &self.repo_path else {
            return Err("No repository open".to_string());
        };
        let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
        let (object, reference) = repo.revparse_ext(branch_name).map_err(|e| e.to_string())?;
        repo.checkout_tree(&object, None).map_err(|e| e.to_string())?;
        if let Some(ref_ref) = reference {
            repo.set_head(ref_ref.name().unwrap_or("")).map_err(|e| e.to_string())?;
        } else {
            repo.set_head_detached(object.id()).map_err(|e| e.to_string())?;
        }
        self.refresh_status();
        Ok(())
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Version Control");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if let Some(path) = &self.repo_path {
                ui.label(RichText::new("Repository: ").strong());
                ui.label(path.display().to_string());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        self.refresh_status();
                    }
                });
            } else {
                ui.label(RichText::new("No repository open").color(egui::Color32::GRAY));
            }
        });

        ui.add_space(8.0);

        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            ui.add_space(4.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Commit");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Commit").clicked() && !self.commit_message.trim().is_empty() {
                    if let Err(e) = self.commit(&self.commit_message) {
                        self.error = Some(e);
                    } else {
                        self.commit_message.clear();
                    }
                }
            });
        });

        ui.add_space(4.0);
        ui.label("Message:");
        ui.text_edit_multiline(&mut self.commit_message);

        ui.add_space(8.0);
        ui.separator();
        ui.heading("Changes");
        ui.add_space(4.0);

        if self.statuses.is_empty() && self.repo_path.is_some() {
            ui.label("No changes.");
        } else {
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                egui::Grid::new("git_status_grid")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("File").strong());
                        ui.label(RichText::new("Status").strong());
                        ui.label(RichText::new("Staged").strong());
                        ui.label(RichText::new("Actions").strong());
                        ui.end_row();
                        for file_status in &self.statuses {
                            let status_str = format!("{:?}", file_status.status);
                            let status_color = if file_status.staged {
                                Color32::GREEN
                            } else {
                                Color32::YELLOW
                            };
                            ui.label(&file_status.path);
                            ui.colored_label(status_color, status_str);
                            ui.label(if file_status.staged { "Yes" } else { "No" });
                            ui.horizontal(|ui| {
                                if file_status.staged {
                                    if ui.small_button("Unstage").clicked() {
                                        let _ = self.unstage_file(&file_status.path);
                                    }
                                } else {
                                    if ui.small_button("Stage").clicked() {
                                        let _ = self.stage_file(&file_status.path);
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        }

        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Branches");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Checkout").clicked() {
                    self.show_checkout_dialog = true;
                }
            });
        });

        if self.show_checkout_dialog {
            egui::Window::new("Checkout Branch")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(egui::Context::default(), |ui| {
                    ui.label("Branch name:");
                    ui.text_edit_singleline(&mut self.branch_name);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_checkout_dialog = false;
                        }
                        if ui.button("Checkout").clicked() {
                            if let Err(e) = self.checkout_branch(&self.branch_name) {
                                self.error = Some(e);
                            } else {
                                self.show_checkout_dialog = false;
                                self.branch_name.clear();
                            }
                        }
                    });
                });
        }

        ui.add_space(8.0);
        ui.separator();
        ui.heading("Remotes");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Remote URL:");
            ui.text_edit_singleline(&mut self.remote_url);
            if ui.button("Add Remote").clicked() && !self.remote_url.is_empty() {
                // TODO: Add remote
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("Push").clicked() {
                // TODO: Push
            }
            if ui.button("Pull").clicked() {
                // TODO: Pull
            }
            if ui.button("Fetch").clicked() {
                // TODO: Fetch
            }
        });

        ui.add_space(8.0);

        if let Some(repo_path) = &self.repo_path {
            ui.separator();
            ui.heading("Repository Info");
            ui.add_space(4.0);

            if let Ok(repo) = Repository::open(repo_path) {
                if let Ok(head) = repo.head() {
                    if let Some(branch) = head.shorthand() {
                        ui.horizontal(|ui| {
                            ui.label("Current branch:");
                            ui.strong(branch);
                        });
                        if let Ok(remotes) = repo.remotes() {
                            if !remotes.is_empty() {
                                ui.add_space(4.0);
                                ui.label("Remotes:");
                                for remote in remotes.iter() {
                                    if let Some(name) = remote {
                                        ui.horizontal(|ui| {
                                            ui.label(name);
                                            if let Ok(url) = repo.remote_url(name) {
                                                ui.label(url);
                                            }
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
