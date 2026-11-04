//! Saved effect authoring with the same particle simulation as gameplay.
use kiln_core::visual::ParticleEffect;
use kiln_vfx::{EmissionShape, EmitterConfig, ParticleSystem};
#[derive(Default)]
pub struct VfxEditorState {
    pub particle_system: ParticleSystem,
    pub preview_playing: bool,
    pub selected_emitter: Option<usize>,
    signature: String,
    clock: kiln_runtime::play_clock::PlayClock,
}
impl VfxEditorState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            for _ in 0..self.clock.advance(dt as f64) {
                self.particle_system
                    .update(kiln_runtime::play_clock::PLAY_STEP as f32);
            }
        }
    }
    pub fn render_effect(
        &mut self,
        ui: &mut egui::Ui,
        owner: &str,
        effect: &mut ParticleEffect,
    ) -> bool {
        let before = effect.clone();
        ui.heading("Particle/VFX Editor");
        ui.small("Changes are saved on the selected scene entity.");
        ui.horizontal(|ui| {
            ui.checkbox(&mut effect.playing, "Emit on Play");
            ui.add(
                egui::DragValue::new(&mut effect.max_particles)
                    .prefix("Maximum ")
                    .range(1..=100_000),
            );
            ui.add(egui::DragValue::new(&mut effect.layer).prefix("Sorting layer "));
        });
        ui.horizontal(|ui| {
            if ui.button("Add Emitter").clicked() && effect.emitters.len() < 64 {
                effect.emitters.push(EmitterConfig::default());
                self.selected_emitter = Some(effect.emitters.len() - 1);
            }
            if ui.button("Remove Emitter").clicked() {
                if let Some(index) = self
                    .selected_emitter
                    .filter(|index| *index < effect.emitters.len())
                {
                    effect.emitters.remove(index);
                    self.selected_emitter = None;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for index in 0..effect.emitters.len() {
                ui.selectable_value(
                    &mut self.selected_emitter,
                    Some(index),
                    format!("Emitter {index}"),
                );
            }
        });
        if self.selected_emitter.is_none() && !effect.emitters.is_empty() {
            self.selected_emitter = Some(0);
        }
        if let Some(config) = self
            .selected_emitter
            .and_then(|index| effect.emitters.get_mut(index))
        {
            egui::ComboBox::from_id_salt("emission_shape")
                .selected_text(format!("{:?}", config.shape))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.shape, EmissionShape::Point, "Point");
                    ui.selectable_value(
                        &mut config.shape,
                        EmissionShape::Circle { radius: 10.0 },
                        "Circle",
                    );
                    ui.selectable_value(
                        &mut config.shape,
                        EmissionShape::Rectangle {
                            width: 10.0,
                            height: 10.0,
                        },
                        "Rectangle",
                    );
                });
            match &mut config.shape {
                EmissionShape::Circle { radius } => {
                    ui.add(
                        egui::DragValue::new(radius)
                            .prefix("Radius ")
                            .range(0.0..=8192.0),
                    );
                }
                EmissionShape::Rectangle { width, height } => {
                    ui.add(
                        egui::DragValue::new(width)
                            .prefix("Width ")
                            .range(0.0..=8192.0),
                    );
                    ui.add(
                        egui::DragValue::new(height)
                            .prefix("Height ")
                            .range(0.0..=8192.0),
                    );
                }
                _ => {}
            }
            ui.add(
                egui::DragValue::new(&mut config.rate)
                    .prefix("Particles / second ")
                    .range(0.0..=100_000.0),
            );
            for (label, range) in [
                ("Speed", &mut config.speed_range),
                ("Angle (radians)", &mut config.angle_range),
                ("Lifetime", &mut config.lifetime_range),
                ("Size", &mut config.size_range),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    for value in range {
                        ui.add(egui::DragValue::new(value).speed(0.05));
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Gravity");
                for value in &mut config.gravity {
                    ui.add(egui::DragValue::new(value).speed(0.1));
                }
            });
            ui.color_edit_button_rgba_unmultiplied(&mut config.color);
            ui.checkbox(&mut config.fade_out, "Fade over lifetime");
        }
        if let Err(error) = effect.validate() {
            ui.colored_label(egui::Color32::YELLOW, error);
            *effect = before;
            return false;
        }
        let signature = serde_json::to_string(&(owner, &effect)).unwrap_or_default();
        if self.signature != signature {
            self.signature = signature;
            self.particle_system = kiln_runtime::ParticleSystemComponent::from_effect(effect)
                .unwrap()
                .system()
                .clone();
            self.particle_system.emitting = true;
            self.clock = Default::default();
        }
        ui.horizontal(|ui| {
            if ui
                .button(if self.preview_playing {
                    "Pause Preview"
                } else {
                    "Play Preview"
                })
                .clicked()
            {
                self.preview_playing = !self.preview_playing;
                self.clock = Default::default();
            }
            if ui.button("Restart Preview").clicked() {
                self.signature.clear();
            }
            ui.label(format!(
                "{} particles",
                self.particle_system.particle_count()
            ));
        });
        self.update_preview(ui.input(|input| input.stable_dt));
        let (area, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().min(400.0), 280.0),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(area, 0.0, egui::Color32::from_gray(20));
        let painter = ui.painter().with_clip_rect(area);
        for particle in self.particle_system.particles() {
            let position = area.center() + egui::vec2(particle.position[0], -particle.position[1]);
            let color = egui::Color32::from_rgba_unmultiplied(
                (particle.color[0] * 255.0) as u8,
                (particle.color[1] * 255.0) as u8,
                (particle.color[2] * 255.0) as u8,
                (particle.color[3] * 255.0) as u8,
            );
            painter.rect_filled(
                egui::Rect::from_center_size(position, egui::Vec2::splat(particle.size)),
                0.0,
                color,
            );
        }
        if self.preview_playing {
            ui.ctx().request_repaint();
        }
        *effect != before
    }
}
