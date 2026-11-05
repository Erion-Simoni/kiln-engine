use super::*;
use kiln_core::ui::UiFont;
use std::io::Write;
/// Validate a font and import its resource without overwriting a different file.
pub fn import_font(
    doc: &mut UiDocument,
    root: &Path,
    source: &Path,
    id: &str,
) -> Result<(), String> {
    let id = id.trim();
    if id.is_empty() || doc.fonts.contains_key(id) {
        return Err("Choose a unique font ID".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let path = if source.is_absolute() {
        source.to_owned()
    } else {
        root.join(source)
    }
    .canonicalize()
    .map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    kiln_ui::authored::fonts::build_atlas(&bytes, [])?;
    let relative = if let Ok(path) = path.strip_prefix(&root) {
        path.to_owned()
    } else {
        let folder = root.join("assets/fonts");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let folder = folder.canonicalize().map_err(|e| e.to_string())?;
        if !folder.starts_with(&root) {
            return Err("Font folder escapes the project".into());
        }
        let target = folder.join(path.file_name().ok_or("Invalid font filename")?);
        if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Font destination is a symbolic link".into());
        }
        if target.exists() {
            if std::fs::read(&target).map_err(|e| e.to_string())? != bytes {
                return Err("A different font file has that filename".into());
            }
        } else {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            if let Err(e) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
                let _ = std::fs::remove_file(&target);
                return Err(e.to_string());
            }
        }
        target
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_owned()
    };
    doc.fonts.insert(
        id.into(),
        UiFont {
            path: relative.to_string_lossy().replace('\\', "/"),
            extra_chars: String::new(),
        },
    );
    Ok(())
}
impl UiLayoutEditorState {
    pub(super) fn font_controls(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut UiDocument,
        root: Option<&Path>,
    ) {
        ui.collapsing("Fonts", |ui| {
            ui.horizontal(|ui| {
                ui.label("Font ID");
                ui.text_edit_singleline(&mut self.font_id);
                ui.label("File path");
                ui.text_edit_singleline(&mut self.font_source);
                if ui.button("Import TTF/OTF").clicked() {
                    self.error = root
                        .ok_or("Save the project first".into())
                        .and_then(|root| {
                            import_font(doc, root, Path::new(&self.font_source), &self.font_id)
                        })
                        .err();
                }
            });
            let mut remove = None;
            for (id, font) in &mut doc.fonts {
                ui.horizontal(|ui| {
                    ui.label(id);
                    ui.label(&font.path);
                    ui.label("Extra characters");
                    ui.text_edit_singleline(&mut font.extra_chars);
                    if ui.button("Remove").clicked() {
                        remove = Some(id.clone());
                    }
                });
            }
            if let Some(id) = remove {
                doc.fonts.remove(&id);
                for s in doc.screens.values_mut() {
                    for w in &mut s.widgets {
                        if w.font == id {
                            w.font.clear();
                        }
                    }
                }
            }
        });
    }
    pub(super) fn prepare_fonts(
        &mut self,
        ctx: &egui::Context,
        doc: &UiDocument,
        root: Option<&Path>,
    ) {
        let extras: String = doc
            .screens
            .values()
            .flat_map(|s| {
                s.widgets
                    .iter()
                    .flat_map(|w| w.text.chars().filter(|c| !c.is_ascii()))
            })
            .collect();
        let stamp = format!("{root:?}{:?}{extras}", doc.fonts);
        if stamp == self.font_stamp {
            return;
        }
        self.font_stamp = stamp;
        self.font_images.clear();
        let mut index = 1;
        let (fonts, errors) = UiFonts::load(
            doc,
            |path| {
                std::fs::read(root.ok_or("No project folder")?.join(path))
                    .map_err(|e| e.to_string())
            },
            |id, size, rgba| {
                let key = kiln_render::TextureId(index);
                index += 1;
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [size[0] as usize, size[1] as usize],
                    rgba,
                );
                self.font_images.insert(
                    key,
                    ctx.load_texture(format!("ui-font-{id}"), image, egui::TextureOptions::LINEAR),
                );
                key
            },
        );
        self.fonts = fonts;
        if !errors.is_empty() {
            self.error = Some(errors.join("\n"));
        }
    }
}
