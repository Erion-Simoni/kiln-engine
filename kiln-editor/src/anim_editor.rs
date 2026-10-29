//! Animation editor panel — sprite sheet clip authoring, frame editing, preview.

use egui::{Color32, DragValue, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use kiln_anim::{Clip, Frame, FrameEvent};
use std::collections::HashMap;

const FRAME_WIDTH: f32 = 100.0;
const FRAME_HEIGHT: f32 = 100.0;
const TIMELINE_HEIGHT: f32 = 60.0;

/// State of the animation editor.
#[derive(Debug, Default)]
pub struct AnimationEditorState {
    /// All clips being edited, keyed by clip name.
    pub clips: HashMap<String, Clip>,
    /// Name of the currently selected clip.
    pub selected_clip: Option<String>,
    /// Preview playback state.
    pub preview_playing: bool,
    /// Preview elapsed time.
    pub preview_time: f32,
    /// Spritesheet texture path for the current clip.
    pub spritesheet_path: String,
    /// Spritesheet dimensions (width, height).
    pub spritesheet_size: (u32, u32),
    /// Whether to show the new clip dialog.
    pub show_new_clip_dialog: bool,
    /// Name for the new clip.
    pub new_clip_name: String,
}

impl AnimationEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected clip, if any.
    pub fn selected_clip(&self) -> Option<&Clip> {
        self.selected_clip.as_ref().and_then(|name| self.clips.get(name))
    }

    /// Get the currently selected clip mutably, if any.
    pub fn selected_clip_mut(&mut self) -> Option<&mut Clip> {
        if let Some(name) = self.selected_clip.clone() {
            self.clips.get_mut(&name)
        } else {
            None
        }
    }

    /// Add a new clip.
    pub fn add_clip(&mut self, name: String, texture: String) {
        let clip = Clip::new(name.clone(), texture);
        self.clips.insert(name.clone(), clip);
        self.selected_clip = Some(name);
    }

    /// Remove a clip by name.
    pub fn remove_clip(&mut self, name: &str) {
        self.clips.remove(name);
        if self.selected_clip.as_deref() == Some(name) {
            self.selected_clip = None;
        }
    }

    /// Update preview time.
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            if let Some(clip) = self.selected_clip() {
                self.preview_time += dt;
                if self.preview_time > clip.total_duration() {
                    self.preview_time = 0.0;
                }
            }
        }
    }


/// Render the animation editor panel.
pub fn render_animation_editor(
    ui: &mut Ui,
    state: &mut AnimationEditorState,
    project: &mut kiln_core::Project,
    dt: f32,
) {
    ui.heading("Animation Editor");
    ui.add_space(8.0);

    // Clip selector and toolbar
    ui.horizontal(|ui| {
        ui.label("Clip:");
        egui::ComboBox::from_label("")
            .selected_text(
                state
                    .selected_clip
                    .as_deref()
                    .unwrap_or("Select a clip..."),
            )
            .show_ui(ui, |ui| {
                for name in state.clips.keys().cloned().collect::<Vec<_>>() {
                    if ui.selectable_label(state.selected_clip.as_deref() == Some(&name), &name).clicked() {
                        state.selected_clip = Some(name);
                        state.preview_time = 0.0;
                    }
                }
            });

        if ui.button("+ New Clip").clicked() {
            state.show_new_clip_dialog = true;
            state.new_clip_name.clear();
        }

        if ui.button("Delete Clip").clicked() {
            if let Some(name) = state.selected_clip.take() {
                state.remove_clip(&name);
            }
        }
    });

    ui.separator();

    // New clip dialog
    if state.show_new_clip_dialog {
        egui::Window::new("New Animation Clip")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Clip name:");
                ui.text_edit_singleline(&mut state.new_clip_name);
                ui.label("Spritesheet path:");
                ui.text_edit_singleline(&mut state.spritesheet_path);
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if !state.new_clip_name.is_empty() && !state.spritesheet_path.is_empty() {
                            state.add_clip(state.new_clip_name.clone(), state.spritesheet_path.clone());
                            state.show_new_clip_dialog = false;
                            state.new_clip_name.clear();
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        state.show_new_clip_dialog = false;
                        state.new_clip_name.clear();
                    }
                });
            });
    }

    // Main editor area
    if let Some(clip) = state.selected_clip_mut() {
        ui.columns(2, |cols| {
            // Left column: frames list and properties
            cols[0].vertical(|ui| {
                ui.heading("Frames");
                ui.add_space(4.0);

                // Spritesheet info
                ui.horizontal(|ui| {
                    ui.label("Spritesheet:");
                    ui.text_edit_singleline(&mut clip.texture);
                });
                ui.horizontal(|ui| {
                    ui.label("Loops:");
                    ui.checkbox(&mut clip.loops, "");
                });

                ui.separator();

                // Frame list
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, frame) in clip.frames.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let frame_label = format!("Frame {}: {}x{} @ {:.2}s", 
                                idx, frame.rect[2], frame.rect[3], frame.duration);
                            if ui.selectable_label(false, frame_label).clicked() {
                                // Frame selected
                            }
                            if ui.small_button("✕").clicked() {
                                clip.frames.remove(idx);
                            }
                        });
                    }
                });

                ui.add_space(8.0);
                if ui.button("+ Add Frame").clicked() {
                    clip.add_frame(Frame {
                        rect: [0, 0, 32, 32],
                        pivot: [0.5, 0.5],
                        duration: 0.1,
                        events: Vec::new(),
                    });
                }

                ui.add_space(8.0);
                
                // Frame events - simplified for now
            });

            // Right column: preview and timeline
            cols[1].vertical(|ui| {
                ui.heading("Preview");
                ui.add_space(4.0);

                // Preview canvas
                let preview_rect = ui.allocate_rect(
                    Rect::from_min_size(ui.cursor().left_top(), Vec2::new(320.0, 240.0)),
                    Sense::hover(),
                );

                // Draw preview placeholder
                ui.painter().rect_filled(preview_rect.rect, 4.0, Color32::from_rgb(30, 30, 35));
                ui.painter().rect_stroke(preview_rect.rect, 4.0, Stroke::new(1.0, Color32::from_gray(80)));
                
                if let Some(frame) = get_frame_at_time(clip, state.preview_time) {
                    ui.add_space(8.0);
                    ui.label(format!("Frame: {} / {}", 
                        get_frame_index_at_time(clip, state.preview_time).unwrap_or(0),
                        clip.frames.len().saturating_sub(1)));
                    ui.label(format!("Time: {:.2}s / {:.2}s", state.preview_time, clip.total_duration()));
                }

                ui.add_space(8.0);

                // Playback controls
                ui.horizontal(|ui| {
                    if ui.button(if state.preview_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                        state.preview_playing = !state.preview_playing;
                    }
                    if ui.button("⏹ Stop").clicked() {
                        state.preview_playing = false;
                        state.preview_time = 0.0;
                    }
                });

                // Timeline scrubber
                ui.add_space(4.0);
                let total = clip.total_duration();
                if total > 0.0 {
                    let mut t = state.preview_time;
                    if ui.add(DragValue::new(&mut t).speed(0.01).range(0.0..=total).suffix("s")).changed() {
                        state.preview_time = t.clamp(0.0, total);
                    }
                    
                    // Visual timeline
                    let timeline_rect = Rect::from_min_size(
                        ui.cursor().left_top(),
                        Vec2::new(ui.available_width(), TIMELINE_HEIGHT),
                    );
                    ui.allocate_rect(timeline_rect, Sense::click_and_drag());
                    
                    // Timeline background
                    ui.painter().rect_filled(timeline_rect, 2.0, Color32::from_rgb(30, 30, 35));
                    ui.painter().rect_stroke(timeline_rect, 2.0, Stroke::new(1.0, Color32::from_gray(80)));
                    
                    // Frame markers
                    let mut t = 0.0;
                    for frame in &clip.frames {
                        let x = timeline_rect.left() + (t / total) * timeline_rect.width();
                        ui.painter().line_segment(
                            [Pos2::new(x, timeline_rect.top()), Pos2::new(x, timeline_rect.bottom())],
                            Stroke::new(1.0, Color32::from_gray(100)),
                        );
                        t += frame.duration;
                    }
                    
                    // Current time marker
                    let x = timeline_rect.left() + (state.preview_time / total) * timeline_rect.width();
                    ui.painter().line_segment(
                        [Pos2::new(x, timeline_rect.top()), Pos2::new(x, timeline_rect.bottom())],
                        Stroke::new(2.0, Color32::from_rgb(100, 200, 255)),
                    );
                }
            });
        });

        // Update preview time
        state.update_preview(dt);
    }
}


/// Get the frame at a given time.
fn get_frame_at_time(clip: &Clip, time: f32) -> Option<&Frame> {
    match clip.frame_at_time(time) {
        Ok((idx, _)) => clip.frames.get(idx),
        Err(_) => None,
    }
}

/// Get the frame index at a given time.
fn get_frame_index_at_time(clip: &Clip, time: f32) -> Option<usize> {
    clip.frame_at_time(time).ok().map(|(idx, _)| idx)
}


