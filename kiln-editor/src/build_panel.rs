//! Saved project export through the real CLI, with supervised progress and output.
mod job;
use egui::{Color32, RichText, ScrollArea, Ui};
use job::{ExportJob, Outcome};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub output_dir: PathBuf,
    pub cli_path: PathBuf,
    pub player_path: PathBuf,
}
impl Default for BuildConfig {
    fn default() -> Self {
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_owned))
            .unwrap_or_default();
        Self {
            output_dir: PathBuf::from("export/game"),
            cli_path: base.join(if cfg!(windows) {
                "kiln-cli.exe"
            } else {
                "kiln-cli"
            }),
            player_path: base.join(if cfg!(windows) {
                "kiln-player.exe"
            } else {
                "kiln-player"
            }),
        }
    }
}
#[derive(Debug, Default)]
pub struct BuildPanel {
    config: BuildConfig,
    job: Option<ExportJob>,
    build_log: Vec<String>,
    status: String,
    error: bool,
}
impl BuildPanel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fail(&mut self, error: String) {
        log::warn!("Export failed: {error}");
        self.status = error.clone();
        self.error = true;
        self.build_log.push(error);
    }
    pub fn start(
        &mut self,
        config: BuildConfig,
        manifest: &Path,
        project: &str,
    ) -> Result<(), String> {
        if self.job.is_some() {
            return Err("An export is already running".into());
        }
        self.build_log.clear();
        self.error = false;
        self.job = Some(ExportJob::start(&config, manifest, project)?);
        self.status = "Exporting saved project…".into();
        Ok(())
    }
    /// Poll even if the dock is hidden; process and pipe work never blocks rendering.
    pub fn poll(&mut self) {
        let result = self
            .job
            .as_mut()
            .and_then(|job| job.poll(&mut self.build_log));
        if let Some(result) = result {
            self.job = None;
            log::info!("Export result: {result:?}");
            match result {
                Outcome::Success(output) => {
                    self.status = format!("Export verified: {}", output.display())
                }
                Outcome::Cancelled => self.status = "Export cancelled".into(),
                Outcome::Failed(error) => self.fail(error),
            }
        }
    }
    pub fn render(&mut self, ui: &mut Ui, project: &kiln_core::Project) -> Option<BuildConfig> {
        ui.heading("Build / Export");
        ui.label(format!("{} • {}", project.name, project.version));
        ui.label(format!(
            "Target: {} ({})",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        ui.label("Standalone native folder. Save & Export includes edits in all scene tabs.");
        ui.label("Choose a new or empty output folder; existing exports are preserved.");
        ui.add_enabled_ui(self.job.is_none(), |ui| {
            path_field(ui, "Output directory:", &mut self.config.output_dir);
            ui.collapsing("Runtime tools", |ui| {
                path_field(ui, "Exporter:", &mut self.config.cli_path);
                path_field(ui, "Player runtime:", &mut self.config.player_path);
                ui.label(
                    "Use matching prebuilt tools beside the editor, or choose their paths here.",
                );
            });
        });
        ui.small("Browser export, installers and other target platforms are pending.");
        let mut request = None;
        if let Some(job) = &mut self.job {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&self.status);
                if ui.button("Cancel export").clicked() {
                    job.cancel();
                    self.status = "Cancelling export…".into();
                }
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        } else if ui.button(RichText::new("Save & Export").strong()).clicked() {
            request = Some(self.config.clone());
        }
        if !self.status.is_empty() {
            ui.colored_label(
                if self.error {
                    Color32::LIGHT_RED
                } else {
                    Color32::LIGHT_GREEN
                },
                &self.status,
            );
        }
        if !self.build_log.is_empty() {
            ui.separator();
            ui.label("Exporter diagnostics (bounded tail):");
            ScrollArea::vertical()
                .max_height(300.)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.build_log {
                        ui.monospace(line);
                    }
                });
        }
        request
    }
}
fn path_field(ui: &mut Ui, label: &str, path: &mut PathBuf) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut value = path.to_string_lossy().into_owned();
        if ui.text_edit_singleline(&mut value).changed() {
            *path = PathBuf::from(value);
        }
    });
}
