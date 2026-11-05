//! Saved screen authoring. Both UI tabs use this single project-backed model.
mod fonts;
mod inspector;
use egui::{Color32, Pos2, Rect, Sense, Vec2};
pub use fonts::import_font;
use kiln_core::{
    ui::{UiDocument, UiKind, UiScreen, UiWidget},
    Project,
};
use kiln_ui::authored::{UiFonts, UiInput, UiRuntime, UiViewport};
use std::{collections::HashMap, path::Path};

/// Transient selection/preview/history. Authored widgets live only in Project.ui.
#[derive(Default)]
pub struct UiLayoutEditorState {
    owner: String,
    pub selected_screen: String,
    pub selected_widget: String,
    new_screen: String,
    font_id: String,
    font_source: String,
    error: Option<String>,
    undo: Vec<UiDocument>,
    redo: Vec<UiDocument>,
    edit_base: Option<UiDocument>,
    last_document: Option<UiDocument>,
    preview_mode: bool,
    preview: UiRuntime,
    preview_source: Option<(UiDocument, String)>,
    fonts: UiFonts,
    font_images: HashMap<kiln_render::TextureId, egui::TextureHandle>,
    font_stamp: String,
    canvas_size: [f32; 2],
    snap: f32,
    drag_base: Option<[f32; 2]>,
}
impl UiLayoutEditorState {
    pub fn new() -> Self {
        Self {
            canvas_size: [640.0, 360.0],
            snap: 8.0,
            ..Default::default()
        }
    }
    fn remember(&mut self, before: UiDocument) {
        self.undo.push(before);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    /// Shared authoring transaction for UI controls and disk/archive acceptance tests.
    pub fn edit<T>(
        &mut self,
        project: &mut Project,
        apply: impl FnOnce(&mut UiDocument) -> Result<T, String>,
    ) -> Result<T, String> {
        self.finish_edit(project);
        let before = project.ui.clone();
        let mut candidate = before.clone();
        let value = apply(&mut candidate)?;
        candidate.validate()?;
        if candidate != before {
            self.remember(before);
            project.ui = candidate;
        }
        self.last_document = Some(project.ui.clone());
        Ok(value)
    }
    fn finish_edit(&mut self, project: &Project) {
        if let Some(before) = self.edit_base.take() {
            if before != project.ui {
                self.remember(before);
            }
        }
    }
    pub fn undo(&mut self, project: &mut Project) -> bool {
        self.finish_edit(project);
        if let Some(doc) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut project.ui, doc));
            self.last_document = Some(project.ui.clone());
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self, project: &mut Project) -> bool {
        self.finish_edit(project);
        if let Some(doc) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut project.ui, doc));
            self.last_document = Some(project.ui.clone());
            true
        } else {
            false
        }
    }
    fn sync(&mut self, project: &Project) {
        if self.owner != project.id
            || self
                .last_document
                .as_ref()
                .is_some_and(|doc| *doc != project.ui)
        {
            self.undo.clear();
            self.redo.clear();
            self.edit_base = None;
            self.preview_source = None;
            self.selected_widget.clear();
            self.font_stamp.clear();
        }
        self.owner.clone_from(&project.id);
        if !project.ui.screens.contains_key(&self.selected_screen) {
            self.selected_screen = project
                .ui
                .screens
                .keys()
                .next()
                .cloned()
                .unwrap_or_default();
        }
    }
    pub fn render(&mut self, ui: &mut egui::Ui, project: &mut Project, root: Option<&Path>) {
        self.sync(project);
        ui.heading("Saved UI Screens");
        ui.label("Save Project preserves these screens. Preview tests navigation; script events run in Play.");
        let mut action = 0;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.undo.is_empty() || self.edit_base.is_some(),
                    egui::Button::new("Undo UI"),
                )
                .clicked()
            {
                action = 1;
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo UI"))
                .clicked()
            {
                action = 2;
            }
            ui.checkbox(&mut self.preview_mode, "Preview");
            if ui.button("Restart preview").clicked() {
                self.preview_source = None;
            }
            if ui.button("Reload fonts").clicked() {
                self.font_stamp.clear();
            }
        });
        if action == 1 {
            self.undo(project);
            return;
        }
        if action == 2 {
            self.redo(project);
            return;
        }
        let before = project.ui.clone();
        self.document_controls(ui, &mut project.ui, &project.scenes);
        self.font_controls(ui, &mut project.ui, root);
        let mut delete = None;
        if let Some(screen) = project.ui.screens.get_mut(&self.selected_screen) {
            ui.horizontal(|ui| {
                ui.label("Screen name");
                ui.text_edit_singleline(&mut screen.name);
                ui.checkbox(&mut screen.pause_game, "Pause gameplay");
                ui.checkbox(&mut screen.modal, "Capture input");
                ui.checkbox(&mut screen.allow_cancel, "Back allowed");
            });
            ui.horizontal(|ui| {
                ui.label("Add:");
                for kind in [UiKind::Panel, UiKind::Label, UiKind::Button] {
                    if ui.button(format!("{kind:?}")).clicked() {
                        self.selected_widget = add_widget(screen, kind);
                    }
                }
                egui::ComboBox::from_id_salt("ui_default_focus")
                    .selected_text(format!("Default focus: {}", screen.default_focus))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut screen.default_focus, String::new(), "Automatic");
                        for w in screen.widgets.iter().filter(|w| w.kind == UiKind::Button) {
                            ui.selectable_value(&mut screen.default_focus, w.id.clone(), &w.id);
                        }
                    });
            });
            ui.horizontal_wrapped(|ui| {
                for w in &screen.widgets {
                    if ui
                        .selectable_label(
                            w.id == self.selected_widget,
                            format!("{} ({:?})", w.id, w.kind),
                        )
                        .clicked()
                    {
                        self.selected_widget = w.id.clone();
                    }
                }
            });
            if ui.button("Delete selected widget and children").clicked() {
                delete = Some(self.selected_widget.clone());
            }
        }
        if let Some(id) = delete {
            if let Some(screen) = project.ui.screens.get_mut(&self.selected_screen) {
                delete_widget(screen, &id);
                self.selected_widget.clear();
            }
        }
        self.inspect(ui, &mut project.ui);
        self.prepare_fonts(ui.ctx(), &project.ui, root);
        self.canvas(ui, &mut project.ui);
        if let Err(error) = project.ui.validate() {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if let Some(error) = &self.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if before != project.ui && self.edit_base.is_none() {
            self.edit_base = Some(before);
        }
        if !ui.input(|i| i.pointer.any_down()) && !ui.ctx().wants_keyboard_input() {
            self.finish_edit(project);
        }
        self.last_document = Some(project.ui.clone());
    }
    fn document_controls(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut UiDocument,
        scenes: &std::collections::HashMap<String, kiln_core::Scene>,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("ui_screen")
                .selected_text(&self.selected_screen)
                .show_ui(ui, |ui| {
                    for (id, s) in &doc.screens {
                        ui.selectable_value(
                            &mut self.selected_screen,
                            id.clone(),
                            format!("{} [{id}]", s.name),
                        );
                    }
                });
            ui.text_edit_singleline(&mut self.new_screen);
            if ui.button("Add screen").clicked() {
                match add_screen(doc, &self.new_screen) {
                    Ok(id) => {
                        self.selected_screen = id;
                        self.new_screen.clear();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            if ui.button("Delete screen").clicked() {
                delete_screen(doc, &self.selected_screen);
                self.selected_screen = doc.screens.keys().next().cloned().unwrap_or_default();
                self.selected_widget.clear();
            }
        });
        ui.horizontal(|ui| {
            screen_choice(ui, "Initial screen", &mut doc.initial_screen, &doc.screens);
            screen_choice(ui, "Pause screen", &mut doc.pause_screen, &doc.screens);
            ui.label("Reference W/H");
            for v in &mut doc.reference_size {
                ui.add(egui::DragValue::new(v).range(1.0..=16384.0));
            }
        });
        ui.collapsing("Initial screen per scene", |ui| {
            let mut ids: Vec<_> = scenes.keys().collect();
            ids.sort();
            for id in ids {
                ui.horizontal(|ui| {
                    ui.label(id);
                    let mut value = doc
                        .scene_screens
                        .get(id)
                        .unwrap_or(&doc.initial_screen)
                        .clone();
                    let before = value.clone();
                    screen_choice(ui, id, &mut value, &doc.screens);
                    if value != before {
                        doc.scene_screens.insert(id.clone(), value);
                    }
                });
            }
        });
    }
    fn canvas(&mut self, ui: &mut egui::Ui, doc: &mut UiDocument) {
        ui.horizontal(|ui| {
            ui.label("Preview size");
            for v in &mut self.canvas_size {
                ui.add(egui::DragValue::new(v).range(100.0..=3840.0));
            }
            ui.label("Snap");
            ui.add(egui::DragValue::new(&mut self.snap).range(0.0..=128.0));
        });
        let scale = (ui.available_width() / self.canvas_size[0]).clamp(0.1, 1.0);
        let size = Vec2::new(self.canvas_size[0] * scale, self.canvas_size[1] * scale);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let area = [rect.min.x, rect.min.y, rect.width(), rect.height()];
        let viewport = UiViewport::new(area, doc.reference_size);
        let mut source = doc.clone();
        source.initial_screen = self.selected_screen.clone();
        source.scene_screens.clear();
        let key = (source.clone(), self.selected_screen.clone());
        if self.preview_source.as_ref() != Some(&key) {
            self.preview = UiRuntime::new(&source, "").unwrap_or_default();
            self.preview_source = Some(key);
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if self.preview_mode {
            let mut input = ui.input(|i| {
                let keys: Vec<_> = i.keys_down.iter().map(|k| format!("{k:?}")).collect();
                let mut input = UiInput::from_keys(keys.iter().map(String::as_str));
                if i.modifiers.shift && input.next {
                    input.next = false;
                    input.previous = true;
                }
                input
            });
            if response.clicked() {
                response.request_focus();
            }
            if !response.has_focus() {
                input = UiInput::default();
            }
            ui.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            ..
                        } => input
                            .pointer_events
                            .push((viewport.logical_point([pos.x, pos.y]), *pressed)),
                        egui::Event::Key {
                            key,
                            pressed: true,
                            repeat: false,
                            modifiers,
                            ..
                        } if response.has_focus() => {
                            input
                                .pressed_keys
                                .push(if *key == egui::Key::Tab && modifiers.shift {
                                    "BackTab".into()
                                } else {
                                    format!("{key:?}")
                                })
                        }
                        _ => {}
                    }
                }
            });
            input.pointer = pointer.and_then(|p| viewport.logical_point([p.x, p.y]));
            input.pointer_down = ui.input(|i| i.pointer.primary_down());
            if let Err(error) = self.preview.update(input, viewport.logical_size) {
                self.error = Some(error);
            }
        }
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, color([0.08, 0.08, 0.10, 1.0]));
        for q in self.preview.draw(&self.fonts, viewport) {
            let i = q.instance;
            let r = Rect::from_center_size(
                Pos2::new(i.position[0], i.position[1]),
                Vec2::new(i.size[0], i.size[1]),
            );
            let color = color(i.color);
            if let Some(image) = self.font_images.get(&q.texture) {
                painter.image(
                    image.id(),
                    r,
                    Rect::from_min_max(
                        Pos2::new(i.uv_rect[0], i.uv_rect[1]),
                        Pos2::new(i.uv_rect[2], i.uv_rect[3]),
                    ),
                    color,
                );
            } else {
                painter.rect_filled(r, 0.0, color);
            }
        }
        if self.preview_mode {
            return;
        }
        let rows = doc
            .screens
            .get(&self.selected_screen)
            .map(|s| kiln_ui::authored::layout::resolve(s, viewport.logical_size))
            .unwrap_or_default();
        if response.clicked() || response.drag_started() {
            if let Some(point) = pointer.and_then(|p| viewport.logical_point([p.x, p.y])) {
                if let Some(screen) = doc.screens.get(&self.selected_screen) {
                    if let Some(row) = rows
                        .iter()
                        .rev()
                        .find(|row| row.visible && row.clip.contains_point(point[0], point[1]))
                    {
                        let w = &screen.widgets[row.index];
                        self.selected_widget = w.id.clone();
                        self.drag_base = Some(w.layout.offset);
                    }
                }
            }
        }
        if let Some(screen) = doc.screens.get_mut(&self.selected_screen) {
            if let Some(row) = rows
                .iter()
                .find(|r| screen.widgets[r.index].id == self.selected_widget)
            {
                let b = row.bounds;
                let r = Rect::from_min_size(
                    Pos2::new(
                        area[0] + b.x * viewport.scale,
                        area[1] + b.y * viewport.scale,
                    ),
                    Vec2::new(b.width, b.height) * viewport.scale,
                );
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0_f32, Color32::YELLOW));
                if response.dragged() {
                    if let Some(base) = self.drag_base {
                        let delta = response.drag_delta() / viewport.scale;
                        let w = &mut screen.widgets[row.index];
                        w.layout.offset = [base[0] + delta.x, base[1] + delta.y].map(|v| {
                            if self.snap > 0.0 {
                                (v / self.snap).round() * self.snap
                            } else {
                                v
                            }
                        });
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag_base = None;
        }
    }
}

fn color(value: [f32; 4]) -> Color32 {
    egui::Rgba::from_rgba_unmultiplied(value[0], value[1], value[2], value[3]).into()
}
fn screen_choice(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    screens: &std::collections::BTreeMap<String, UiScreen>,
) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(format!("{label}: {value}"))
        .show_ui(ui, |ui| {
            ui.selectable_value(value, String::new(), "None");
            for (id, s) in screens {
                ui.selectable_value(value, id.clone(), format!("{} [{id}]", s.name));
            }
        });
}
/// Create a stable screen ID; duplicate names receive a numeric suffix.
pub fn add_screen(doc: &mut UiDocument, name: &str) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Choose a screen name".into());
    }
    let base: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let mut id = base.clone();
    let mut n = 2;
    while doc.screens.contains_key(&id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    doc.screens.insert(
        id.clone(),
        UiScreen {
            name: name.trim().into(),
            ..Default::default()
        },
    );
    Ok(id)
}
pub fn add_widget(screen: &mut UiScreen, kind: UiKind) -> String {
    let id = format!("widget_{}", kiln_core::next_id());
    let mut w = UiWidget {
        id: id.clone(),
        kind,
        text: format!("{kind:?}"),
        ..Default::default()
    };
    if kind == UiKind::Panel {
        w.layout.size = [280.0, 160.0];
        w.color = [0.05, 0.07, 0.12, 0.95];
    }
    if kind == UiKind::Label {
        w.blocks_pointer = false;
    }
    screen.widgets.push(w);
    id
}
pub fn delete_widget(screen: &mut UiScreen, id: &str) {
    let mut removed = std::collections::HashSet::from([id.to_owned()]);
    loop {
        let count = removed.len();
        for w in &screen.widgets {
            if removed.contains(&w.parent) {
                removed.insert(w.id.clone());
            }
        }
        if count == removed.len() {
            break;
        }
    }
    screen.widgets.retain(|w| !removed.contains(&w.id));
    for w in &mut screen.widgets {
        w.navigation.retain(|_, target| !removed.contains(target));
    }
    if removed.contains(&screen.default_focus) {
        screen.default_focus.clear();
    }
}
pub fn delete_screen(doc: &mut UiDocument, id: &str) {
    doc.screens.remove(id);
    if doc.initial_screen == id {
        doc.initial_screen.clear();
    }
    if doc.pause_screen == id {
        doc.pause_screen.clear();
    }
    doc.scene_screens.retain(|_, target| target != id);
    for screen in doc.screens.values_mut() {
        for w in &mut screen.widgets {
            if matches!(&w.action,kiln_core::ui::UiAction::ShowScreen(target) if target==id) {
                w.action = kiln_core::ui::UiAction::Event;
            }
        }
    }
}
