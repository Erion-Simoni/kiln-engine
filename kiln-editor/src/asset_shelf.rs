//! Folder rail and bounded thumbnail shelf for existing project assets.

use crate::assets::{AssetBrowser, AssetKind, SpriteAssetPayload};
use crate::editor_shell::AssetCategory;
use egui::{Color32, TextureHandle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Default)]
pub(crate) struct Thumbnails {
    root: PathBuf,
    entries: HashMap<String, Thumbnail>,
}

struct Thumbnail {
    modified: Option<SystemTime>,
    texture: Option<TextureHandle>,
}

impl Thumbnails {
    fn texture(
        &mut self,
        ctx: &egui::Context,
        browser: &AssetBrowser,
        path: &str,
    ) -> Option<egui::TextureId> {
        let root = browser.root()?;
        if self.root != root {
            self.entries.clear();
            self.root = root.to_owned();
        }
        let full = root.join(path);
        let modified = full.metadata().ok().and_then(|m| m.modified().ok());
        if !self.entries.contains_key(path) || self.entries[path].modified != modified {
            // Cap resident thumbnails to 128 x 64 x 64 RGBA (~2 MiB).
            if self.entries.len() >= 128 && !self.entries.contains_key(path) {
                // Do not free textures already referenced by this frame's paint
                // jobs. Further resources retain their ordinary file cards.
                return None;
            }
            let image = std::fs::read(full)
                .ok()
                .and_then(|bytes| kiln_assets::decode_png(&bytes).ok());
            let texture = image.map(|image| {
                let width = image.width.min(64) as usize;
                let height = image.height.min(64) as usize;
                let mut pixels = Vec::with_capacity(width * height * 4);
                for y in 0..height {
                    for x in 0..width {
                        let source_x = x * image.width as usize / width;
                        let source_y = y * image.height as usize / height;
                        let index = (source_y * image.width as usize + source_x) * 4;
                        pixels.extend_from_slice(&image.pixels[index..index + 4]);
                    }
                }
                ctx.load_texture(
                    format!("asset-thumb:{}:{path}", root.display()),
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels),
                    egui::TextureOptions::NEAREST,
                )
            });
            self.entries
                .insert(path.into(), Thumbnail { modified, texture });
        }
        self.entries[path].texture.as_ref().map(TextureHandle::id)
    }
}

pub(crate) enum ShelfAction {
    SelectImage(String),
    OpenScene(String),
    OpenScript(String),
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    browser: &mut AssetBrowser,
    category: &mut AssetCategory,
    thumbnails: &mut Thumbnails,
    scenes: &[String],
) -> Option<ShelfAction> {
    let mut action = None;
    let available = ui.available_size();
    let shelf_height = available.y.clamp(110.0, 180.0);
    ui.allocate_ui_with_layout(
        egui::vec2(available.x, shelf_height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(164.0, available.y.min(140.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (name, value) in [
                            ("Assets", AssetCategory::All),
                            ("   Textures", AssetCategory::Textures),
                            ("   Scenes", AssetCategory::Scenes),
                            ("   Scripts", AssetCategory::Scripts),
                            ("   Audio", AssetCategory::Audio),
                        ] {
                            if ui
                                .add_sized(
                                    [156.0, 19.0],
                                    egui::Button::new(name)
                                        .selected(*category == value)
                                        .frame(false),
                                )
                                .clicked()
                            {
                                *category = value;
                            }
                        }
                        ui.add_enabled(false, egui::Label::new("   Models                   3D"));
                        ui.add_enabled(false, egui::Label::new("   Materials                3D"));
                    },
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("asset-shelf")
                    .max_height(shelf_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(12.0, 8.0);
                            if matches!(category, AssetCategory::All | AssetCategory::Scenes) {
                                for scene in scenes {
                                    let response = card(ui, scene, None, "SCENE", false);
                                    if response.double_clicked() {
                                        action = Some(ShelfAction::OpenScene(scene.clone()));
                                    }
                                    response.on_hover_text("Double-click to open scene");
                                }
                            }
                            let entries: Vec<_> = browser
                                .filtered_entries()
                                .into_iter()
                                .filter(|entry| category.matches(&entry.path))
                                .cloned()
                                .collect();
                            for entry in entries {
                                let texture = if entry.kind == AssetKind::Image {
                                    thumbnails.texture(ui.ctx(), browser, &entry.path)
                                } else {
                                    None
                                };
                                let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
                                let kind = if entry.kind == AssetKind::Image {
                                    "IMAGE"
                                } else if entry.path.ends_with(".ks") {
                                    "SCRIPT"
                                } else {
                                    "FILE"
                                };
                                let response = card(
                                    ui,
                                    name,
                                    texture,
                                    kind,
                                    browser.preview_source() == Some(&entry.path),
                                );
                                if response.double_clicked() && entry.path.ends_with(".ks") {
                                    action = Some(ShelfAction::OpenScript(entry.path.clone()));
                                }
                                if response.clicked() && entry.kind == AssetKind::Image {
                                    action = Some(ShelfAction::SelectImage(entry.path.clone()));
                                }
                                if response.drag_started() && entry.kind == AssetKind::Image {
                                    browser.set_drag_source(Some(&entry.path));
                                    response.dnd_set_drag_payload(SpriteAssetPayload {
                                        path: entry.path.clone(),
                                    });
                                }
                                response.on_hover_text(&entry.path);
                            }
                        });
                        if browser.entries().is_empty() && scenes.is_empty() {
                            ui.weak("Open a project to browse its assets.");
                        }
                    });
            });
        },
    );
    action
}

fn card(
    ui: &mut egui::Ui,
    name: &str,
    texture: Option<egui::TextureId>,
    kind: &str,
    selected: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(82.0, 88.0), egui::Sense::click_and_drag());
    let painter = ui.painter();
    let image = egui::Rect::from_min_size(rect.min + egui::vec2(12.0, 4.0), egui::vec2(56.0, 56.0));
    if selected || response.hovered() {
        painter.rect_filled(
            rect,
            2.0,
            if selected {
                Color32::from_rgb(42, 77, 115)
            } else {
                Color32::from_gray(52)
            },
        );
    }
    painter.rect_filled(image, 0.0, Color32::from_gray(29));
    if let Some(texture) = texture {
        painter.image(
            texture,
            image.shrink(3.0),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_stroke(
            image.shrink(12.0),
            2.0,
            egui::Stroke::new(1.0_f32, Color32::from_gray(92)),
        );
        painter.text(
            image.center(),
            egui::Align2::CENTER_CENTER,
            kind,
            egui::FontId::proportional(9.0),
            Color32::from_gray(160),
        );
    }
    let font = egui::FontId::proportional(11.0);
    let galley = painter.layout(name.into(), font, Color32::from_gray(190), 80.0);
    painter.galley(
        egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 65.0),
        galley,
        Color32::from_gray(190),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_budget_preserves_textures_referenced_by_the_current_frame() {
        let root = tempfile::tempdir().unwrap();
        let png = kiln_assets::encode_png_rgba(1, 1, &[20, 80, 160, 255]).unwrap();
        for index in 0..129 {
            std::fs::write(root.path().join(format!("{index}.png")), &png).unwrap();
        }
        let mut browser = AssetBrowser::new();
        browser.scan(root.path());
        let ctx = egui::Context::default();
        let mut thumbnails = Thumbnails::default();
        ctx.begin_pass(egui::RawInput::default());
        let first = thumbnails.texture(&ctx, &browser, "0.png").unwrap();
        for index in 1..128 {
            assert!(thumbnails
                .texture(&ctx, &browser, &format!("{index}.png"))
                .is_some());
        }
        assert!(thumbnails.texture(&ctx, &browser, "128.png").is_none());
        assert_eq!(thumbnails.texture(&ctx, &browser, "0.png"), Some(first));
        assert_eq!(thumbnails.entries.len(), 128);
        assert!(ctx.end_pass().textures_delta.free.is_empty());
    }
}
