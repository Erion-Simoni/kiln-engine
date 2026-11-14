//! Native editor chrome matching the supplied Kiln design.

use crate::preview_3d::Preview3d;
use crate::transform_gizmo::GizmoMode;
use egui::{Color32, Stroke};

/// Transient workspace presentation; never serialized into a game project.
pub struct EditorShell {
    pub(crate) selection_origin: Option<egui::Pos2>,
    pub(crate) entity_clipboard: Option<String>,
    pub(crate) script_editor: crate::script_editor::ScriptEditor,
    pub(crate) quick_actions: crate::quick_actions::QuickActions,
    pub(crate) preview_3d: bool,
    pub(crate) grid: bool,
    pub(crate) gizmos: bool,
    pub(crate) preview: Preview3d,
    pub(crate) asset_category: AssetCategory,
    pub(crate) thumbnails: crate::asset_shelf::Thumbnails,
}

impl Default for EditorShell {
    fn default() -> Self {
        Self {
            entity_clipboard: None,
            selection_origin: None,
            script_editor: crate::script_editor::ScriptEditor::default(),
            quick_actions: crate::quick_actions::QuickActions::default(),
            preview_3d: false,
            grid: true,
            gizmos: true,
            preview: Preview3d::default(),
            asset_category: AssetCategory::All,
            thumbnails: crate::asset_shelf::Thumbnails::default(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AssetCategory {
    #[default]
    All,
    Textures,
    Scenes,
    Scripts,
    Audio,
}

impl AssetCategory {
    pub(crate) fn matches(self, path: &str) -> bool {
        let path = path.to_ascii_lowercase();
        match self {
            Self::All => true,
            Self::Textures => path.ends_with(".png"),
            Self::Scenes => path.ends_with(".scene") || path.ends_with(".toml"),
            Self::Scripts => path.ends_with(".ks") || path.ends_with(".kiln"),
            Self::Audio => [".wav", ".ogg", ".mp3", ".flac"]
                .iter()
                .any(|ext| path.ends_with(ext)),
        }
    }
}

/// Install the compact charcoal palette without changing OS scaling.
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    for (text, size) in [
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Heading, 16.0),
        (egui::TextStyle::Small, 11.0),
    ] {
        style
            .text_styles
            .insert(text, egui::FontId::proportional(size));
    }
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.interact_size.y = 24.0;
    style.spacing.indent = 14.0;
    style.spacing.window_margin = egui::Margin::same(8.0);
    let visuals = &mut style.visuals;
    visuals.override_text_color = Some(Color32::from_gray(212));
    visuals.panel_fill = Color32::from_rgb(35, 38, 44);
    visuals.window_fill = Color32::from_rgb(35, 38, 44);
    visuals.faint_bg_color = Color32::from_gray(38);
    visuals.extreme_bg_color = Color32::from_gray(27);
    visuals.code_bg_color = Color32::from_gray(27);
    visuals.window_rounding = 6.0.into();
    visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_gray(23));
    visuals.selection.bg_fill = Color32::from_rgb(47, 100, 155);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.rounding = 4.0.into();
        widget.bg_stroke = Stroke::new(1.0_f32, Color32::from_gray(23));
    }
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(47, 51, 59);
    visuals.widgets.inactive.bg_fill = Color32::from_gray(27);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(61, 70, 84);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(47, 100, 155);
    ctx.set_style(style);
}

#[derive(Default)]
pub(crate) struct ToolbarActions {
    pub toggle_play: bool,
    pub pause: bool,
    pub step: bool,
    pub reset_layout: bool,
    pub show_scene: bool,
}

impl EditorShell {
    pub(crate) fn toolbar(
        &mut self,
        ui: &mut egui::Ui,
        mode: &mut GizmoMode,
        snap: &mut bool,
        snap_size: &mut f32,
        playing: bool,
        paused: bool,
    ) -> ToolbarActions {
        let mut actions = ToolbarActions::default();
        let full = ui.available_rect_before_wrap();
        let center = full.center().x;
        ui.horizontal_centered(|ui| {
            for (key, label, tool) in [
                ("Q", "Select", GizmoMode::None),
                ("W", "Move", GizmoMode::Translate),
                ("E", "Rotate", GizmoMode::Rotate),
                ("R", "Scale", GizmoMode::Scale),
            ] {
                if ui
                    .add_enabled(
                        !self.preview_3d && !playing,
                        egui::Button::new(if full.width() >= 1200.0 {
                            format!("{key}  {label}")
                        } else {
                            key.to_owned()
                        })
                        .selected(*mode == tool)
                        .min_size(egui::vec2(28.0, 24.0)),
                    )
                    .on_hover_text(format!("{label} ({key})"))
                    .clicked()
                {
                    *mode = tool;
                }
            }
            ui.separator();
            if full.width() >= 1000.0 {
                ui.add_enabled(false, egui::Button::new("World"))
                    .on_disabled_hover_text(
                        "Viewport gizmos use world space. Edit local transforms in the Inspector.",
                    );
            }
            if ui
                .add_enabled(
                    !self.preview_3d && !playing,
                    egui::Button::new("Snap").selected(*snap).frame(false),
                )
                .clicked()
            {
                *snap = !*snap;
            }
            ui.add_enabled(
                !self.preview_3d && !playing,
                egui::DragValue::new(snap_size)
                    .range(1.0..=1024.0)
                    .speed(1.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                actions.reset_layout = ui
                    .button(if full.width() >= 1000.0 {
                        "Layout: Default"
                    } else {
                        "Layout"
                    })
                    .on_hover_text("Restore the default workspace layout")
                    .clicked();
                if ui
                    .add_enabled(!playing, egui::Button::new("3D").selected(self.preview_3d))
                    .on_hover_text("Open the interactive 3D visual preview (session only)")
                    .clicked()
                {
                    self.set_preview(true);
                    actions.show_scene = true;
                }
                if ui.selectable_label(!self.preview_3d, "2D").clicked() {
                    self.set_preview(false);
                    actions.show_scene = true;
                }
            });
        });
        let rect = egui::Rect::from_center_size(
            egui::pos2(center, full.center().y),
            egui::vec2(116.0, 26.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                let response = ui.add_sized(
                    [32.0, 24.0],
                    egui::Button::new(if playing { "■" } else { "▶" }).selected(playing),
                );
                actions.toggle_play = response
                    .on_hover_text(if playing {
                        "Stop 2D game"
                    } else {
                        "Play active 2D scene"
                    })
                    .clicked();
                actions.pause = ui
                    .add_enabled(
                        playing,
                        egui::Button::new("II")
                            .min_size(egui::vec2(32.0, 24.0))
                            .selected(paused),
                    )
                    .on_hover_text("Pause / resume game")
                    .clicked();
                actions.step = ui
                    .add_enabled(
                        playing,
                        egui::Button::new("|>").min_size(egui::vec2(32.0, 24.0)),
                    )
                    .on_hover_text("Advance one simulation frame")
                    .clicked();
            });
        });
        actions
    }

    pub(crate) fn set_preview(&mut self, enabled: bool) {
        if self.preview_3d != enabled {
            self.preview_3d = enabled;
            log::info!(
                "Viewport mode: {}",
                if enabled {
                    "3D visual preview"
                } else {
                    "2D scene"
                }
            );
        }
    }

    pub(crate) fn future_components(ui: &mut egui::Ui) {
        ui.add_space(12.0);
        for name in ["Mesh Renderer", "Material", "Light"] {
            ui.add_enabled_ui(false, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("> {name}"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small("3D · preview only");
                    });
                });
            });
            ui.separator();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_transport_receives_real_pointer_clicks() {
        for width in [640.0, 800.0, 1024.0, 1440.0] {
            let ctx = egui::Context::default();
            apply_theme(&ctx);
            let mut shell = EditorShell::default();
            let mut mode = GizmoMode::Translate;
            let mut snap = false;
            let mut size = 8.0;
            let mut clicked = false;
            for events in [
                vec![],
                vec![egui::Event::PointerMoved(egui::pos2(
                    width / 2.0 - 43.0,
                    44.0,
                ))],
                vec![egui::Event::PointerButton {
                    pos: egui::pos2(width / 2.0 - 43.0, 44.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
                vec![egui::Event::PointerButton {
                    pos: egui::pos2(width / 2.0 - 43.0, 44.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ] {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::TopBottomPanel::top("menu")
                            .exact_height(24.0)
                            .show(ctx, |_| {});
                        egui::TopBottomPanel::top("tools")
                            .exact_height(38.0)
                            .show(ctx, |ui| {
                                clicked |= shell
                                    .toolbar(ui, &mut mode, &mut snap, &mut size, false, false)
                                    .toggle_play;
                            });
                    },
                );
            }
            assert!(
                clicked,
                "Centered Play must not be obscured at width {width}"
            );
        }
    }

    #[test]
    fn asset_categories_accept_case_and_exclude_unrelated_resources() {
        assert!(AssetCategory::Textures.matches("Textures/Player.PNG"));
        assert!(!AssetCategory::Textures.matches("scripts/player.kiln"));
        assert!(AssetCategory::Scripts.matches("scripts/player.kiln"));
        assert!(AssetCategory::Audio.matches("music/theme.ogg"));
        assert!(AssetCategory::All.matches("anything"));
    }
}
