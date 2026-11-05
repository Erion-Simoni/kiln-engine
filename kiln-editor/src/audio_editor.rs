//! Project-backed audio authoring and independent native preview.

use kiln_core::{project::AudioAsset, Project};
use std::{io::Write, path::Path};

/// Validate an import before changing the project or copying an external file.
/// Existing internal files retain their project-relative paths; external files
/// are copied under assets/audio without overwriting unrelated resources.
pub fn import_audio(
    project: &mut Project,
    root: &Path,
    source: &Path,
    name: &str,
    music: bool,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Choose an audio name".into());
    }
    if project.audio_assets.iter().any(|asset| asset.name == name) {
        return Err("Audio names must be unique".into());
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let source = if source.is_absolute() {
        source.to_owned()
    } else {
        root.join(source)
    };
    let source = source.canonicalize().map_err(|error| error.to_string())?;
    let bytes = std::fs::read(&source).map_err(|error| error.to_string())?;
    let mut asset = AudioAsset {
        id: kiln_core::next_id().to_string(),
        name: name.into(),
        path: source.to_string_lossy().into(),
        is_music: music,
        looped: music,
        ..Default::default()
    };
    #[cfg(feature = "audio")]
    kiln_runtime::AudioEngine::default().register_bytes(&asset, bytes.clone())?;
    if bytes.is_empty() {
        return Err("Audio file is empty".into());
    }
    let path = if let Ok(relative) = source.strip_prefix(&root) {
        relative.to_owned()
    } else {
        let directory = root.join("assets/audio");
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let directory = directory
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if !directory.starts_with(&root) {
            return Err("Audio directory escapes the project".into());
        }
        let destination = directory.join(source.file_name().ok_or("Invalid file name")?);
        if std::fs::symlink_metadata(&destination)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err("Audio destination is a symbolic link; choose another file name".into());
        }
        if destination.exists() {
            if std::fs::read(&destination).map_err(|error| error.to_string())? != bytes {
                return Err(
                    "A different audio file already has that name; rename the source first".into(),
                );
            }
        } else {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|error| error.to_string())?;
            if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
                let _ = std::fs::remove_file(&destination);
                return Err(error.to_string());
            }
        }
        destination
            .strip_prefix(&root)
            .map_err(|error| error.to_string())?
            .to_owned()
    };
    asset.path = path.to_string_lossy().replace('\\', "/");
    let id = asset.id.clone();
    project
        .asset_ids
        .entry(asset.path.clone())
        .or_insert_with(|| id.clone());
    project.audio_assets.push(asset);
    Ok(id)
}

/// Transient form/preview state only. Authored audio lives in Project.
#[derive(Default)]
pub struct AudioEditorState {
    name: String,
    source: String,
    music: bool,
    bus: String,
    error: Option<String>,
    project_id: String,
    #[cfg(feature = "audio")]
    preview: Option<kiln_runtime::AudioEngine>,
}

impl AudioEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dropping the preview also stops its independent output stream.
    pub fn stop_preview(&mut self) {
        #[cfg(feature = "audio")]
        {
            self.preview = None;
        }
    }

    #[cfg(feature = "audio")]
    fn preview(
        &mut self,
        project: &Project,
        root: &Path,
        asset: &AudioAsset,
    ) -> Result<(), String> {
        self.stop_preview();
        let mut audio = kiln_runtime::AudioEngine::default();
        audio.set_settings(project.audio_mixer.clone());
        let bytes = std::fs::read(root.join(&asset.path)).map_err(|error| error.to_string())?;
        audio.register_bytes(asset, bytes)?;
        audio.open_output()?;
        if asset.is_music {
            audio.play_music(&asset.name);
        } else {
            audio.play_sound(&asset.name);
        }
        self.preview = Some(audio);
        Ok(())
    }

    /// Edit the same assets and mixer that are saved and packed for the player.
    pub fn render(&mut self, ui: &mut egui::Ui, project: &mut Project, root: Option<&Path>) {
        if self.project_id != project.id {
            self.stop_preview();
            self.project_id.clone_from(&project.id);
            self.error = None;
        }
        ui.heading("Audio");
        ui.label(
            "Import a file, assign a bus, then Preview. Save Project preserves these settings.",
        );
        ui.add(egui::Slider::new(&mut project.audio_mixer.master_volume, 0.0..=1.0).text("Master"));
        let bus_names: Vec<String> = project.audio_mixer.buses.keys().cloned().collect();
        let mut remove_bus = None;
        for (name, bus) in &mut project.audio_mixer.buses {
            ui.horizontal(|ui| {
                ui.label(name);
                ui.add(egui::Slider::new(&mut bus.volume, 0.0..=1.0));
                ui.checkbox(&mut bus.muted, "Mute");
                ui.checkbox(&mut bus.solo, "Solo");
                if name != "SFX" && name != "Music" && ui.small_button("Remove bus").clicked() {
                    remove_bus = Some(name.clone());
                }
            });
        }
        if let Some(bus) = remove_bus {
            project.audio_mixer.buses.remove(&bus);
            for asset in &mut project.audio_assets {
                if asset.bus == bus {
                    asset.bus.clear();
                }
            }
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.bus);
            if ui.button("Add bus").clicked()
                && !self.bus.trim().is_empty()
                && self.bus.trim() != "Master"
            {
                project
                    .audio_mixer
                    .buses
                    .entry(self.bus.trim().into())
                    .or_default();
                self.bus.clear();
            }
        });
        #[cfg(feature = "audio")]
        if let Some(audio) = &mut self.preview {
            audio.set_settings(project.audio_mixer.clone());
        }
        ui.separator();
        let mut remove = None;
        #[cfg(feature = "audio")]
        let mut preview = None;
        for (index, asset) in project.audio_assets.iter_mut().enumerate() {
            ui.push_id(asset.id.clone(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(&asset.name);
                    ui.label(&asset.path);
                    ui.checkbox(&mut asset.is_music, "Music");
                    ui.add(egui::Slider::new(&mut asset.volume, 0.0..=1.0).text("Gain"));
                    if asset.is_music {
                        ui.checkbox(&mut asset.looped, "Loop");
                    }
                    egui::ComboBox::from_id_salt("bus")
                        .selected_text(asset.bus_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut asset.bus, String::new(), "Default");
                            for name in &bus_names {
                                ui.selectable_value(&mut asset.bus, name.clone(), name);
                            }
                        });
                    #[cfg(feature = "audio")]
                    if ui.button("Preview").clicked() {
                        preview = Some(asset.clone());
                    }
                    if ui.button("Remove").clicked() {
                        remove = Some(index);
                    }
                });
            });
        }
        if let Some(index) = remove {
            self.stop_preview();
            project.audio_assets.remove(index);
        }
        if ui.button("Stop preview").clicked() {
            self.stop_preview();
        }
        #[cfg(feature = "audio")]
        if let Some(asset) = preview {
            self.error = root
                .ok_or("Save the project first".into())
                .and_then(|root| self.preview(project, root, &asset))
                .err();
        }
        #[cfg(not(feature = "audio"))]
        ui.label(
            "Preview and decoder validation require an audio-enabled build (enabled by default).",
        );
        ui.separator();
        ui.label("Audio name");
        ui.text_edit_singleline(&mut self.name);
        ui.label("File path (project-relative or absolute)");
        ui.text_edit_singleline(&mut self.source);
        ui.checkbox(&mut self.music, "Music track");
        if ui.button("Import audio").clicked() {
            self.error = root
                .ok_or("Save the project first".into())
                .and_then(|root| {
                    import_audio(
                        project,
                        root,
                        Path::new(&self.source),
                        &self.name,
                        self.music,
                    )
                })
                .err();
            if self.error.is_none() {
                self.name.clear();
                self.source.clear();
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }
}
