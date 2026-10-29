//! Particle/VFX Editor panel — emitter configuration, particle preview.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2};
use kiln_vfx::{EmissionShape, Emitter, EmitterConfig, ParticleSystem, ParticleSystemConfig};

/// State of the VFX editor panel.
#[derive(Debug, Default)]
pub struct VfxEditorState {
    /// Particle system being edited.
    pub particle_system: ParticleSystem,
    /// Whether the preview is playing.
    pub preview_playing: bool,
    /// Preview time accumulator.
    pub preview_time: f32,
    /// Selected emitter index.
    pub selected_emitter: Option<usize>,
    /// Whether to show the new emitter dialog.
    pub show_new_emitter_dialog: bool,
    /// New emitter configuration being edited.
    pub new_emitter_config: EmitterConfig,
    /// Preview frame time (fixed at 60 FPS).
    pub preview_dt: f32,
}

impl VfxEditorState {
    pub fn new() -> Self {
        Self {
            particle_system: ParticleSystem::default(),
            preview_playing: false,
            preview_time: 0.0,
            selected_emitter: None,
            show_new_emitter_dialog: false,
            new_emitter_config: EmitterConfig::default(),
            preview_dt: 1.0 / 60.0,
        }
    }

    /// Get the currently selected emitter mutably.
    pub fn selected_emitter_mut(&mut self) -> Option<&mut Emitter> {
        if let Some(idx) = self.selected_emitter {
            self.particle_system.emitters.get_mut(idx)
        } else {
            None
        }
    }

    /// Get the currently selected emitter.
    pub fn selected_emitter(&self) -> Option<&Emitter> {
        if let Some(idx) = self.selected_emitter {
            self.particle_system.emitters.get(idx)
        } else {
            None
        }
    }

    /// Update the preview.
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            self.particle_system.update(dt);
        }
    }


    /// Render the VFX editor panel.
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("Particle/VFX Editor");
        ui.add_space(8.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button(if self.preview_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                self.preview_playing = !self.preview_playing;
            }
            if ui.button("⏹ Stop").clicked() {
                self.preview_playing = false;
                self.particle_system.particles.clear();
            }
            ui.separator();
            if ui.button("➕ Add Emitter").clicked() {
                self.show_new_emitter_dialog = true;
                self.new_emitter_config = EmitterConfig::default();
            }
            if ui.button("🗑 Clear All").clicked() {
                self.particle_system.clear();
                self.selected_emitter = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Particles: {}", self.particle_system.particle_count()));
                ui.label(format!("Emitters: {}", self.particle_system.emitters.len()));
            });
        });

        ui.add_space(8.0);

        // Emitter list
        ui.horizontal(|ui| {
            ui.heading("Emitters");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑 Clear Selected").clicked() {
                    if let Some(idx) = self.selected_emitter {
                        self.particle_system.emitters.remove(idx);
                        self.selected_emitter = None;
                    }
                }
            });
        });

        ui.add_space(4.0);

        // Emitter list
        ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            for (idx, emitter) in self.particle_system.emitters.iter().enumerate() {
                let selected = self.selected_emitter == Some(idx);
                let mut emitter_selected = selected;
                
                ui.horizontal(|ui| {
                    if ui.selectable_label(emitter_selected, format!("Emitter {}", idx)).clicked() {
                        self.selected_emitter = Some(idx);
                    }
                    
                    // Show emitter info
                    let config = emitter.config();
                    ui.label(format!("Rate: {:.1}/s", config.rate));
                    ui.label(format!("Shape: {:?}", config.shape));
                    ui.label(format!("Particles: {}", self.particle_system.particle_count()));
                    
                    if ui.small_button("🗑").clicked() {
                        self.particle_system.emitters.remove(idx);
                        if self.selected_emitter == Some(idx) {
                            self.selected_emitter = None;
                        } else if self.selected_emitter.is_some() && self.selected_emitter.unwrap() > idx {
                            self.selected_emitter = Some(self.selected_emitter.unwrap() - 1);
                        }
                    }
                });
            }
            
            if self.particle_system.emitters.is_empty() {
                ui.label("No emitters. Click 'Add Emitter' to create one.");
            }
        });

        ui.add_space(8.0);


        // Selected emitter properties
        if let Some(emitter) = self.selected_emitter_mut() {
            ui.separator();
            ui.heading("Emitter Properties");
            ui.add_space(4.0);

            let config = emitter.config_mut();

            // Emission shape
            ui.horizontal(|ui| {
                ui.label("Emission Shape:");
                egui::ComboBox::from_label("")
                    .selected_text(match config.shape {
                        EmissionShape::Point => "Point",
                        EmissionShape::Circle { .. } => "Circle",
                        EmissionShape::Rectangle { .. } => "Rectangle",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut config.shape, EmissionShape::Point, "Point");
                        ui.selectable_value(&mut config.shape, EmissionShape::Circle { radius: 10.0 }, "Circle");
                        ui.selectable_value(&mut config.shape, EmissionShape::Rectangle { width: 10.0, height: 10.0 }, "Rectangle");
                    });
            });

            // Shape-specific parameters
            match &mut config.shape {
                EmissionShape::Point => {}
                EmissionShape::Circle { radius } => {
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        ui.add(DragValue::new(radius).speed(0.1).suffix(" px"));
                    });
                }
                EmissionShape::Rectangle { width, height } => {
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(DragValue::new(width).speed(0.1).suffix(" px"));
                        ui.label("Height:");
                        ui.add(DragValue::new(height).speed(0.1).suffix(" px"));
                    });
                }
            }

            ui.add_space(8.0);

            // Emission rate
            ui.horizontal(|ui| {
                ui.label("Rate (particles/sec):");
                ui.add(DragValue::new(&mut config.rate).speed(0.1).range(0.0..=1000.0).suffix("/s"));
            });

            // Speed range
            ui.horizontal(|ui| {
                ui.label("Speed Range:");
                ui.add(DragValue::new(&mut config.speed_range[0]).speed(0.1).suffix(" min"));
                ui.add(DragValue::new(&mut config.speed_range[1]).speed(0.1).suffix(" max"));
            });

            // Angle range
            ui.horizontal(|ui| {
                ui.label("Angle Range (rad):");
                ui.add(DragValue::new(&mut config.angle_range[0]).speed(0.01).suffix(" min"));
                ui.add(DragValue::new(&mut config.angle_range[1]).speed(0.01).suffix(" max"));
            });

            // Lifetime range
            ui.horizontal(|ui| {
                ui.label("Lifetime Range (s):");
                ui.add(DragValue::new(&mut config.lifetime_range[0]).speed(0.01).suffix(" min"));
                ui.add(DragValue::new(&mut config.lifetime_range[1]).speed(0.01).suffix(" max"));
            });

            // Size range
            ui.horizontal(|ui| {
                ui.label("Size Range:");
                ui.add(DragValue::new(&mut config.size_range[0]).speed(0.01).suffix(" min"));
                ui.add(DragValue::new(&mut config.size_range[1]).speed(0.01).suffix(" max"));
            });

            // Color
            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color = config.color;
                if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                    config.color = color;
                }
            });

            // Gravity
            ui.horizontal(|ui| {
                ui.label("Gravity (x, y):");
                ui.add(DragValue::new(&mut config.gravity[0]).speed(0.1).suffix(" x"));
                ui.add(DragValue::new(&mut config.gravity[1]).speed(0.1).suffix(" y"));
            });

            // Fade out
            ui.horizontal(|ui| {
                ui.label("Fade Out:");
                ui.checkbox(&mut config.fade_out, "");
            });

        } else {
            ui.label("Select an emitter to edit its properties, or add a new one.");
        }

        ui.add_space(8.0);


        ui.add_space(8.0);

        // Preview area
        ui.separator();
        ui.heading("Preview");
        ui.add_space(4.0);

        // Preview canvas
        let preview_size = Vec2::new(400.0, 300.0);
        let (rect, response) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
        
        // Draw preview background
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(20, 20, 30));
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, Color32::from_gray(80)));

        // Draw particles in preview
        let particles = self.particle_system.particles();
        let center = rect.center();
        let scale = 1.0; // 1 world unit = 1 pixel in preview

        for particle in particles {
            let pos = egui::pos2(
                center.x + particle.position[0] * scale,
                center.y - particle.position[1] * scale, // Flip Y for screen coords
            );
            let size = particle.size * scale;
            let color = Color32::from_rgba_premultiplied(
                (particle.color[0] * 255.0) as u8,
                (particle.color[1] * 255.0) as u8,
                (particle.color[2] * 255.0) as u8,
                (particle.color[3] * 255.0) as u8,
            );
            ui.painter().circle_filled(pos, size, color);
        }

        // Draw emitter positions
        for emitter in &self.particle_system.emitters {
            let pos = egui::pos2(
                center.x + emitter.position()[0] * scale,
                center.y - emitter.position()[1] * scale,
            );
            ui.painter().circle_stroke(pos, 8.0, egui::Stroke::new(2.0, Color32::from_rgb(100, 200, 255)));
            ui.painter().text(pos, egui::Align2::CENTER_CENTER, "E", egui::FontId::proportional(12.0), Color32::from_rgb(100, 200, 255));
        }

        // Click to position selected emitter
        if response.clicked() {
            if let Some(emitter) = self.selected_emitter_mut() {
                let click_pos = response.interact_pointer_pos().unwrap_or(rect.center());
                let world_x = (click_pos.x - center.x) / scale;
                let world_y = -(click_pos.y - center.y) / scale;
                emitter.set_position([world_x, world_y]);
            }
        }

        // Update preview
        self.update_preview(self.preview_dt);
    }
}


