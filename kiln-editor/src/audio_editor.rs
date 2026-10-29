//! Audio Editor panel — sound effect/music management, volume control, preview.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui};
use std::path::PathBuf;

/// Audio asset entry.
#[derive(Debug, Clone)]
pub struct AudioAsset {
    pub name: String,
    pub path: PathBuf,
    pub is_music: bool,
    pub volume: f32,
    pub looped: bool,
}

/// Audio mixer channel.
#[derive(Debug, Clone)]
pub struct AudioChannel {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
}

/// State of the audio editor panel.
#[derive(Debug, Default)]
pub struct AudioEditorState {
    /// Sound effects list.
    pub sound_effects: Vec<AudioAsset>,
    /// Music tracks list.
    pub music_tracks: Vec<AudioAsset>,
    /// Mixer channels.
    pub channels: Vec<AudioChannel>,
    /// Master volume.
    pub master_volume: f32,
    /// Whether preview is playing.
    pub preview_playing: bool,
    /// Currently previewed asset.
    pub preview_asset: Option<String>,
    /// Show new asset dialog.
    pub show_new_asset_dialog: bool,
    /// New asset name.
    pub new_asset_name: String,
    /// New asset path.
    pub new_asset_path: String,
    /// New asset type (sound effect / music).
    pub new_asset_is_music: bool,
}

impl AudioEditorState {
    pub fn new() -> Self {
        let mut state = Self {
            sound_effects: Vec::new(),
            music_tracks: Vec::new(),
            channels: vec![
                AudioChannel { name: "Master".to_string(), volume: 1.0, muted: false, solo: false },
                AudioChannel { name: "SFX".to_string(), volume: 1.0, muted: false, solo: false },
                AudioChannel { name: "Music".to_string(), volume: 1.0, muted: false, solo: false },
            ],
            master_volume: 1.0,
            preview_playing: false,
            preview_asset: None,
            show_new_asset_dialog: false,
            new_asset_name: String::new(),
            new_asset_path: String::new(),
            new_asset_is_music: false,
        };
        // Add default channels
        state.channels = vec![
            AudioChannel { name: "Master".to_string(), volume: 1.0, muted: false, solo: false },
            AudioChannel { name: "SFX".to_string(), volume: 1.0, muted: false, solo: false },
            AudioChannel { name: "Music".to_string(), volume: 1.0, muted: false, solo: false },
        ];
        state
    }


    /// Render the audio editor panel.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audio Editor");
        ui.add_space(8.0);

        // Master volume
        ui.horizontal(|ui| {
            ui.label("Master Volume:");
            ui.add(egui::DragValue::new(&mut self.master_volume)
                .speed(0.01)
                .range(0.0..=1.0)
                .suffix("x"));
            if ui.button("🔇 Mute").clicked() {
                self.master_volume = 0.0;
            }
        );

        ui.add_space(8.0);

        // Mixer channels
        ui.heading("Mixer");
        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
            egui::Grid::new("mixer_grid")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Channel").strong());
                    ui.label(egui::RichText::new("Volume").strong());
                    ui.label(egui::RichText::new("Mute").strong());
                    ui.label(egui::RichText::new("Solo").strong());
                    ui.label(egui::RichText::new("Actions").strong());
                    ui.end_row();

                    for (idx, channel) in self.channels.iter_mut().enumerate() {
                        ui.label(&channel.name);
                        ui.add(egui::DragValue::new(&mut channel.volume)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .suffix("x"));
                        ui.checkbox(&mut channel.muted, "");
                        ui.checkbox(&mut channel.solo, "");
                        if ui.small_button("Reset").clicked() {
                            channel.volume = 1.0;
                            channel.muted = false;
                            channel.solo = false;
                        }
                        ui.end_row();
                    }
                });
        });

        ui.add_space(8.0);


        ui.add_space(8.0);

        // Sound Effects
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Sound Effects");
            if ui.button("➕ Add SFX").clicked() {
                self.show_new_asset_dialog = true;
                self.new_asset_is_music = false;
                self.new_asset_name.clear();
                self.new_asset_path.clear();
            }
        );

        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, asset) in self.sound_effects.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let playing = self.preview_asset == Some(asset.name.clone()) && self.preview_playing;
                    if ui.selectable_label(playing, &asset.name).clicked() {
                        // Toggle preview
                        if self.preview_asset == Some(asset.name.clone()) {
                            self.preview_playing = !self.preview_playing;
                            self.preview_asset = Some(asset.name.clone());
                        } else {
                            self.preview_playing = true;
                            self.preview_asset = Some(asset.name.clone());
                        }
                    }

                    ui.label(format!("Vol: {:.0}%", asset.volume * 100.0));
                    ui.add(egui::DragValue::new(&mut asset.volume).speed(0.01).range(0.0..=1.0).suffix("x"));

                    if ui.small_button("▶").clicked() {
                        self.preview_playing = true;
                        self.preview_asset = Some(asset.name.clone());
                    }
                    if ui.small_button("⏹").clicked() {
                        self.preview_playing = false;
                        self.preview_asset = None;
                    }
                    if ui.small_button("🗑").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.sound_effects.remove(idx);
            }

            if self.sound_effects.is_empty() {
                ui.label("No sound effects. Click 'Add SFX' to add one.");
            }
        });

        ui.add_space(8.0);


        // Music Tracks
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Music Tracks");
            if ui.button("➕ Add Music").clicked() {
                self.show_new_asset_dialog = true;
                self.new_asset_is_music = true;
                self.new_asset_name.clear();
                self.new_asset_path.clear();
            }
        );

        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, asset) in self.music_tracks.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let playing = self.preview_asset == Some(asset.name.clone()) && self.preview_playing;
                    if ui.selectable_label(playing, &asset.name).clicked() {
                        if self.preview_asset == Some(asset.name.clone()) {
                            self.preview_playing = !self.preview_playing;
                            self.preview_asset = Some(asset.name.clone());
                        } else {
                            self.preview_playing = true;
                            self.preview_asset = Some(asset.name.clone());
                        }
                    }

                    ui.add(egui::DragValue::new(&mut asset.volume).speed(0.01).range(0.0..=1.0).suffix("x"));
                    ui.checkbox(&mut asset.looped, "Loop");

                    if ui.small_button("▶").clicked() {
                        self.preview_playing = true;
                        self.preview_asset = Some(asset.name.clone());
                    }
                    if ui.small_button("⏹").clicked() {
                        self.preview_playing = false;
                        self.preview_asset = None;
                    }
                    if ui.small_button("🗑").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.music_tracks.remove(idx);
            }

            if self.music_tracks.is_empty() {
                ui.label("No music tracks. Click 'Add Music' to add one.");
            }
        });

        ui.add_space(8.0);

        // New asset dialog
        if self.show_new_asset_dialog {
            egui::Window::new(if self.new_asset_is_music { "Add Music Track" } else { "Add Sound Effect" })
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_asset_name);

                    ui.add_space(4.0);
                    ui.label("File Path:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_asset_path);
                        if ui.button("Browse...").clicked() {
                            // TODO: Native file dialog
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.radio_value(&mut self.new_asset_is_music, false, "Sound Effect");
                        ui.radio_value(&mut self.new_asset_is_music, true, "Music Track");
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_new_asset_dialog = false;
                            self.new_asset_name.clear();
                            self.new_asset_path.clear();
                        }
                        if ui.button("Add").clicked() {
                            if !self.new_asset_name.is_empty() && !self.new_asset_path.is_empty() {
                                let asset = AudioAsset {
                                    name: self.new_asset_name.clone(),
                                    path: std::path::PathBuf::from(&self.new_asset_path),
                                    is_music: self.new_asset_is_music,
                                    volume: 1.0,
                                    looped: false,
                                };
                                if self.new_asset_is_music {
                                    self.music_tracks.push(asset);
                                } else {
                                    self.sound_effects.push(asset);
                                }
                                self.show_new_asset_dialog = false;
                                self.new_asset_name.clear();
                                self.new_asset_path.clear();
                            }
                        }
                    });
            });
        }
    }
}


