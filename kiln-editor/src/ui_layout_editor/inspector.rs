use super::*;
use kiln_core::ui::UiAction;
impl UiLayoutEditorState {
    pub(super) fn inspect(&mut self, ui: &mut egui::Ui, doc: &mut UiDocument) {
        let font_ids: Vec<_> = doc.fonts.keys().cloned().collect();
        let screens: Vec<_> = doc.screens.keys().cloned().collect();
        let Some(screen) = doc.screens.get_mut(&self.selected_screen) else {
            return;
        };
        let ids: Vec<_> = screen.widgets.iter().map(|w| w.id.clone()).collect();
        let Some(index) = screen
            .widgets
            .iter()
            .position(|w| w.id == self.selected_widget)
        else {
            return;
        };
        let w = &mut screen.widgets[index];
        let old_parent = w.parent.clone();
        let mut reorder = 0;
        ui.collapsing(format!("Properties: {}", w.id), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut w.visible, "Visible");
                ui.checkbox(&mut w.enabled, "Enabled");
                ui.checkbox(&mut w.blocks_pointer, "Block pointer");
                if ui.button("Earlier").clicked() {
                    reorder = -1;
                }
                if ui.button("Later").clicked() {
                    reorder = 1;
                }
                egui::ComboBox::from_id_salt("parent")
                    .selected_text(format!("Parent: {}", w.parent))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut w.parent, String::new(), "Screen root");
                        for id in ids.iter().filter(|id| **id != w.id) {
                            ui.selectable_value(&mut w.parent, id.clone(), id);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Anchor preset:");
                for (name, anchor) in [
                    ("Top left", [0.0, 0.0]),
                    ("Top", [0.5, 0.0]),
                    ("Top right", [1.0, 0.0]),
                    ("Left", [0.0, 0.5]),
                    ("Center", [0.5, 0.5]),
                    ("Right", [1.0, 0.5]),
                    ("Bottom left", [0.0, 1.0]),
                    ("Bottom", [0.5, 1.0]),
                    ("Bottom right", [1.0, 1.0]),
                ] {
                    if ui.small_button(name).clicked() {
                        w.layout.anchor = anchor;
                        w.layout.pivot = anchor;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Anchor X/Y");
                for v in &mut w.layout.anchor {
                    ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0));
                }
                ui.label("Pivot X/Y");
                for v in &mut w.layout.pivot {
                    ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset X/Y");
                for v in &mut w.layout.offset {
                    ui.add(egui::DragValue::new(v).range(-32768.0..=32768.0));
                }
                ui.label("Size W/H");
                for v in &mut w.layout.size {
                    ui.add(egui::DragValue::new(v).range(-32768.0..=32768.0));
                }
                ui.checkbox(&mut w.layout.stretch[0], "Stretch width");
                ui.checkbox(&mut w.layout.stretch[1], "Stretch height");
            });
            ui.label("Stretched size adds to parent size; negative values create margins.");
            if w.kind != UiKind::Panel {
                ui.label("Text (newlines supported)");
                ui.text_edit_multiline(&mut w.text);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("ui_font")
                        .selected_text(format!("Font: {}", w.font))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut w.font, String::new(), "Built-in bitmap");
                            for id in &font_ids {
                                ui.selectable_value(&mut w.font, id.clone(), id);
                            }
                        });
                    ui.label("Font size");
                    ui.add(egui::DragValue::new(&mut w.font_size).range(1.0..=256.0));
                    ui.label("Alignment X/Y");
                    for v in &mut w.text_align {
                        ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0));
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Background");
                ui.color_edit_button_rgba_unmultiplied(&mut w.color);
                ui.label("Text");
                ui.color_edit_button_rgba_unmultiplied(&mut w.text_color);
                ui.label("Focus");
                ui.color_edit_button_rgba_unmultiplied(&mut w.focus_color);
            });
            if w.kind == UiKind::Button {
                ui.horizontal(|ui| {
                    ui.label("Script event (blank uses widget ID)");
                    ui.text_edit_singleline(&mut w.event);
                    let mut action = match w.action {
                        UiAction::Event => 0,
                        UiAction::ShowScreen(_) => 1,
                        UiAction::Back => 2,
                        UiAction::Close => 3,
                    };
                    egui::ComboBox::from_id_salt("ui_action")
                        .selected_text(["Event", "Show screen", "Back", "Close"][action])
                        .show_ui(ui, |ui| {
                            for (i, label) in
                                ["Event", "Show screen", "Back", "Close"].iter().enumerate()
                            {
                                ui.selectable_value(&mut action, i, *label);
                            }
                        });
                    match action {
                        0 => w.action = UiAction::Event,
                        2 => w.action = UiAction::Back,
                        3 => w.action = UiAction::Close,
                        _ => {
                            if !matches!(w.action, UiAction::ShowScreen(_)) {
                                w.action = UiAction::ShowScreen(
                                    screens.first().cloned().unwrap_or_default(),
                                );
                            }
                            if let UiAction::ShowScreen(target) = &mut w.action {
                                egui::ComboBox::from_id_salt("ui_target")
                                    .selected_text(target.clone())
                                    .show_ui(ui, |ui| {
                                        for id in &screens {
                                            ui.selectable_value(target, id.clone(), id);
                                        }
                                    });
                            }
                        }
                    }
                });
                ui.collapsing("Explicit focus links (automatic when empty)", |ui| {
                    for direction in ["up", "down", "left", "right", "next", "previous"] {
                        let mut target = w.navigation.get(direction).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt(direction)
                            .selected_text(format!("{direction}: {target}"))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut target, String::new(), "Automatic");
                                for id in &ids {
                                    ui.selectable_value(&mut target, id.clone(), id);
                                }
                            });
                        if target.is_empty() {
                            w.navigation.remove(direction);
                        } else {
                            w.navigation.insert(direction.into(), target);
                        }
                    }
                });
            }
        });
        if reorder < 0 && index > 0 {
            screen.widgets.swap(index, index - 1);
        } else if reorder > 0 && index + 1 < screen.widgets.len() {
            screen.widgets.swap(index, index + 1);
        }
        let index = screen
            .widgets
            .iter()
            .position(|w| w.id == self.selected_widget)
            .unwrap();
        if screen.widgets[index].parent != old_parent {
            if let Err(error) = doc.validate() {
                self.error = Some(error);
                doc.screens.get_mut(&self.selected_screen).unwrap().widgets[index].parent =
                    old_parent;
            }
        }
    }
}
