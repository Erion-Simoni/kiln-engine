//! Saved clip authoring and texture preview using the runtime animation player.
use crate::image_preview::PreviewImages;
use kiln_core::{Clip, Frame, FrameEvent};
use kiln_runtime::AnimationPlayer;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Default)]
pub struct AnimationEditorState {
    pub clips: HashMap<String, Clip>,
    pub selected_clip: Option<String>,
    pub selected_frame: usize,
    pub preview_playing: bool,
    pub preview_time: f32,
    pub spritesheet_path: String,
    pub spritesheet_size: (u32, u32),
    pub show_new_clip_dialog: bool,
    pub new_clip_name: String,
    pub preview_events: Vec<FrameEvent>,
    images: PreviewImages,
    player: AnimationPlayer,
    signature: String,
    undo: Vec<HashMap<String, Clip>>,
    redo: Vec<HashMap<String, Clip>>,
    history_action: bool,
    history_origin: String,
}
impl AnimationEditorState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn selected_clip(&self) -> Option<&Clip> {
        self.selected_clip
            .as_ref()
            .and_then(|name| self.clips.get(name))
    }
    pub fn selected_clip_mut(&mut self) -> Option<&mut Clip> {
        self.selected_clip
            .as_ref()
            .and_then(|name| self.clips.get_mut(name))
    }
    pub fn add_clip(&mut self, name: String, texture: String) {
        let mut clip = Clip::new(name.clone(), texture);
        clip.add_frame(Frame {
            rect: [0, 0, 16, 16],
            pivot: [0.5; 2],
            duration: 0.1,
            events: vec![],
        });
        self.clips.insert(name.clone(), clip);
        self.selected_clip = Some(name);
    }
    pub fn remove_clip(&mut self, name: &str) {
        self.clips.remove(name);
        if self.selected_clip.as_deref() == Some(name) {
            self.selected_clip = None;
        }
    }
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            self.player.update(dt);
            self.preview_time = self.player.elapsed();
            self.preview_events = self.player.take_events();
        }
    }
    /// Compatibility entry for headless panel callers.
    pub fn render_animation_editor(
        ui: &mut egui::Ui,
        state: &mut Self,
        clips: &mut HashMap<String, Clip>,
        dt: f32,
    ) {
        if clips.is_empty() {
            clips.extend(state.clips.clone());
        }
        Self::render_with_root(ui, state, clips, dt, None);
    }
    pub fn render_with_root(
        ui: &mut egui::Ui,
        state: &mut Self,
        clips: &mut HashMap<String, Clip>,
        dt: f32,
        root: Option<&Path>,
    ) {
        let origin = root
            .map(|root| root.to_string_lossy().into_owned())
            .unwrap_or_default();
        if origin != state.history_origin {
            state.undo.clear();
            state.redo.clear();
            state.history_origin = origin;
        }
        let before = clips.clone();
        state.history_action = false;
        Self::render_contents(ui, state, clips, dt, root);
        if !state.history_action
            && serde_json::to_value(&before).ok() != serde_json::to_value(&clips).ok()
        {
            state.record_edit(before);
        }
    }

    pub fn record_edit(&mut self, before: HashMap<String, Clip>) {
        self.undo.push(before);
        self.redo.clear();
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
    }
    pub fn undo_clips(&mut self, clips: &mut HashMap<String, Clip>) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(clips, previous));
        }
    }
    pub fn redo_clips(&mut self, clips: &mut HashMap<String, Clip>) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(clips, next));
        }
    }

    fn render_contents(
        ui: &mut egui::Ui,
        state: &mut Self,
        clips: &mut HashMap<String, Clip>,
        dt: f32,
        root: Option<&Path>,
    ) {
        state.clips = clips.clone();
        if state
            .selected_clip
            .as_ref()
            .is_some_and(|name| !clips.contains_key(name))
        {
            state.selected_clip = None;
        }
        let mut names: Vec<_> = clips.keys().cloned().collect();
        names.sort();
        ui.heading("Animation Editor");
        ui.horizontal(|ui| {
            if ui.button("Undo Clip Edit").clicked() {
                state.undo_clips(clips);
                state.history_action = true;
            }
            if ui.button("Redo Clip Edit").clicked() {
                state.redo_clips(clips);
                state.history_action = true;
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("clip_select")
                .selected_text(state.selected_clip.as_deref().unwrap_or("Choose clip"))
                .show_ui(ui, |ui| {
                    for name in &names {
                        if ui
                            .selectable_value(&mut state.selected_clip, Some(name.clone()), name)
                            .changed()
                        {
                            state.selected_frame = 0;
                            state.preview_time = 0.0;
                        }
                    }
                });
            if ui.button("New Clip").clicked() {
                state.show_new_clip_dialog = true;
            }
            if ui.button("Delete Clip").clicked() {
                if let Some(name) = state.selected_clip.take() {
                    clips.remove(&name);
                    state.clips.remove(&name);
                }
            }
        });
        if state.show_new_clip_dialog {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut state.new_clip_name);
            });
            ui.horizontal(|ui| {
                ui.label("Sheet path");
                ui.text_edit_singleline(&mut state.spritesheet_path);
            });
            if ui.button("Create").clicked()
                && !state.new_clip_name.trim().is_empty()
                && !state.spritesheet_path.is_empty()
                && !clips.contains_key(&state.new_clip_name)
            {
                state.add_clip(state.new_clip_name.clone(), state.spritesheet_path.clone());
                clips.extend(state.clips.clone());
                state.show_new_clip_dialog = false;
                state.new_clip_name.clear();
            }
        }
        let Some(name) = state.selected_clip.clone() else {
            ui.label("Create or select a clip.");
            return;
        };
        let Some(mut clip) = clips.get(&name).cloned() else {
            return;
        };
        state.selected_frame = state
            .selected_frame
            .min(clip.frames.len().saturating_sub(1));
        ui.horizontal(|ui| {
            ui.label("Sheet");
            ui.text_edit_singleline(&mut clip.texture);
            ui.checkbox(&mut clip.loops, "Loop");
        });
        egui::ComboBox::from_id_salt("next_clip")
            .selected_text(clip.next.as_deref().unwrap_or("Hold last frame"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut clip.next, None, "Hold last frame");
                for next in &names {
                    ui.selectable_value(&mut clip.next, Some(next.clone()), format!("Then {next}"));
                }
            });
        ui.small("Next clip runs when a non-looping clip ends.");
        let image = state.images.load(ui.ctx(), root, &clip.texture);
        if let Ok(image) = &image {
            state.spritesheet_size = (image.size[0], image.size[1]);
        }
        ui.horizontal(|ui| {
            if ui.button("Add Frame").clicked() {
                clip.add_frame(Frame {
                    rect: [0, 0, 16, 16],
                    pivot: [0.5; 2],
                    duration: 0.1,
                    events: vec![],
                });
                state.selected_frame = clip.frames.len() - 1;
            }
            if ui.button("Delete Frame").clicked() && clip.frames.len() > 1 {
                clip.frames
                    .remove(state.selected_frame.min(clip.frames.len() - 1));
                state.selected_frame = state.selected_frame.saturating_sub(1);
            }
            if ui.button("Move Left").clicked() && state.selected_frame > 0 {
                clip.frames
                    .swap(state.selected_frame, state.selected_frame - 1);
                state.selected_frame -= 1;
            }
            if ui.button("Move Right").clicked() && state.selected_frame + 1 < clip.frames.len() {
                clip.frames
                    .swap(state.selected_frame, state.selected_frame + 1);
                state.selected_frame += 1;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for index in 0..clip.frames.len() {
                ui.selectable_value(&mut state.selected_frame, index, format!("Frame {index}"));
            }
        });
        if let Some(frame) = clip.frames.get_mut(state.selected_frame) {
            ui.horizontal(|ui| {
                for (value, label) in frame.rect.iter_mut().zip(["X ", "Y ", "Width ", "Height "]) {
                    ui.add(egui::DragValue::new(value).prefix(label));
                }
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut frame.duration)
                        .prefix("Seconds ")
                        .speed(0.01)
                        .range(0.001..=3600.0),
                );
                for (value, label) in frame.pivot.iter_mut().zip(["Pivot X ", "Y "]) {
                    ui.add(
                        egui::DragValue::new(value)
                            .prefix(label)
                            .speed(0.01)
                            .range(0.0..=1.0),
                    );
                }
            });
            let mut remove = None;
            for (index, event) in frame.events.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label("Event");
                    ui.text_edit_singleline(&mut event.name);
                    let payload = event.payload.get_or_insert_with(String::new);
                    ui.text_edit_singleline(payload);
                    if ui.small_button("Remove").clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                frame.events.remove(index);
            }
            if ui.button("Add Frame Event").clicked() {
                frame.events.push(FrameEvent {
                    name: "event".into(),
                    payload: None,
                });
            }
        }
        let size = image
            .as_ref()
            .map(|image| image.size)
            .unwrap_or([u32::MAX; 2]);
        if let Err(error) = clip.validate(size) {
            ui.colored_label(egui::Color32::YELLOW, error);
        } else {
            clips.insert(name.clone(), clip.clone());
            state.clips = clips.clone();
        }
        let signature = serde_json::to_string(&(clips.clone(), state.selected_clip.clone()))
            .unwrap_or_default();
        if signature != state.signature {
            state.signature = signature;
            state.player = AnimationPlayer::new();
            for clip in clips.values() {
                let size = state
                    .images
                    .load(ui.ctx(), root, &clip.texture)
                    .map(|image| image.size)
                    .unwrap_or([1, 1]);
                state
                    .player
                    .add_bound_clip(clip.clone(), kiln_render::TextureId::WHITE, size);
            }
            state.player.play(&name);
            if state.preview_time > 0.0 {
                state.player.seek(state.preview_time);
            }
        }
        ui.horizontal(|ui| {
            if ui
                .button(if state.preview_playing {
                    "Pause"
                } else {
                    "Play"
                })
                .clicked()
            {
                state.preview_playing = !state.preview_playing;
            }
            if ui.button("Stop").clicked() {
                state.preview_playing = false;
                state.preview_time = 0.0;
                state.player.stop();
                state.player.play(&name);
            }
            let mut time = state.preview_time;
            if ui
                .add(
                    egui::DragValue::new(&mut time)
                        .prefix("Time ")
                        .speed(0.01)
                        .range(0.0..=clip.total_duration()),
                )
                .changed()
            {
                state.preview_time = time;
                state.player.seek(time);
            }
        });
        state.update_preview(dt);
        let current = state.player.current_clip().and_then(|name| clips.get(name));
        let current_image =
            current.and_then(|clip| state.images.load(ui.ctx(), root, &clip.texture).ok());
        let (area, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().min(360.0), 240.0),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(area, 0.0, egui::Color32::from_gray(25));
        if let (Some(image), Some(frame), Some(uv)) = (
            current_image,
            state.player.frame(),
            state.player.current_uv_rect(),
        ) {
            let scale = (area.width() / frame.rect[2] as f32)
                .min(area.height() / frame.rect[3] as f32)
                .min(8.0);
            let rect = egui::Rect::from_center_size(
                area.center(),
                egui::vec2(frame.rect[2] as f32 * scale, frame.rect[3] as f32 * scale),
            );
            ui.painter().image(
                image.texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(uv[0], uv[1]), egui::pos2(uv[2], uv[3])),
                egui::Color32::WHITE,
            );
            let pivot = rect.min
                + egui::vec2(
                    rect.width() * frame.pivot[0],
                    rect.height() * frame.pivot[1],
                );
            ui.painter()
                .circle_stroke(pivot, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::RED));
        } else if let Err(error) = image {
            ui.label(error);
        }
        for event in &state.preview_events {
            ui.label(format!(
                "{}: {}",
                event.name,
                event.payload.as_deref().unwrap_or("")
            ));
        }
        if state.preview_playing {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod panel_tests {
    use super::*;
    #[test]
    fn rendering_keeps_the_selected_clip_and_syncs_it_to_the_project() {
        let mut state = AnimationEditorState::new();
        state.add_clip("idle".into(), "sprite.png".into());
        let mut clips = HashMap::new();
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                AnimationEditorState::render_animation_editor(ui, &mut state, &mut clips, 0.016)
            });
        });
        assert_eq!(state.selected_clip().unwrap().texture, "sprite.png");
        assert_eq!(clips["idle"].texture, "sprite.png");
    }
}

#[cfg(test)]
mod texture_preview_tests {
    use super::*;
    #[test]
    fn preview_draws_the_runtime_frame_from_the_decoded_sheet_and_undo_preserves_events() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("sheet.png"),
            kiln_assets::encode_png_rgba(32, 16, &vec![255; 32 * 16 * 4]).unwrap(),
        )
        .unwrap();
        let mut state = AnimationEditorState::new();
        state.add_clip("walk".into(), "sheet.png".into());
        let clip = state.clips.get_mut("walk").unwrap();
        clip.add_frame(Frame {
            rect: [16, 0, 16, 16],
            pivot: [0.5; 2],
            duration: 0.1,
            events: vec![FrameEvent {
                name: "step".into(),
                payload: Some("stone".into()),
            }],
        });
        let mut clips = state.clips.clone();
        state.preview_playing = true;
        let ctx = egui::Context::default();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                AnimationEditorState::render_with_root(
                    ui,
                    &mut state,
                    &mut clips,
                    0.125,
                    Some(root.path()),
                )
            });
        });
        let texture = state
            .images
            .load(&ctx, Some(root.path()), "sheet.png")
            .unwrap()
            .texture
            .id();
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh),
                _ => None,
            })
            .expect("textured preview mesh");
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.uv.x >= 0.5 && vertex.uv.x <= 1.0));
        assert_eq!(state.preview_events[0].name, "step");
        let before = clips.clone();
        clips.get_mut("walk").unwrap().frames[1].events.clear();
        state.record_edit(before.clone());
        state.undo_clips(&mut clips);
        assert_eq!(
            clips["walk"].frames[1].events[0].payload.as_deref(),
            Some("stone")
        );
        state.redo_clips(&mut clips);
        assert!(clips["walk"].frames[1].events.is_empty());
    }
}
