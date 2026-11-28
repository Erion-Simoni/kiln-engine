//! Saved atlas recipe authoring beside the Sprite Editor.
use kiln_core::{atlas::AtlasRecipe, Project};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct AtlasEditor {
    pub open: bool,
    root: Option<PathBuf>,
    id: String,
    recipe: AtlasRecipe,
    label: String,
    source: String,
    message: String,
    built: bool,
}
impl Default for AtlasEditor {
    fn default() -> Self {
        Self {
            open: false,
            root: None,
            id: "atlas".into(),
            recipe: Default::default(),
            label: "hero".into(),
            source: "art/hero.png".into(),
            message: String::new(),
            built: false,
        }
    }
}
impl AtlasEditor {
    pub fn take_built(&mut self) -> bool {
        std::mem::take(&mut self.built)
    }
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        project: &mut Project,
        root: Option<&Path>,
        enabled: bool,
    ) {
        if !self.open {
            return;
        }
        if self.root.as_deref() != root {
            *self = Self {
                open: true,
                root: root.map(Path::to_path_buf),
                ..Default::default()
            };
        }
        let mut open = self.open;
        egui::Window::new("Texture Atlas").open(&mut open).enabled(enabled)
            .collapsible(false).resizable(false).vscroll(true).max_height(650.0).default_pos(egui::pos2(450.0, 160.0)).default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Build a sheet and one static clip per source label.");
                let mut ids: Vec<_> = project.atlases.keys().cloned().collect(); ids.sort();
                egui::ComboBox::from_id_salt("saved_atlas_recipe").selected_text("Load saved recipe").show_ui(ui, |ui| {
                    for id in ids {
                        if ui.selectable_label(false, &id).clicked() {
                            self.recipe = project.atlases[&id].clone(); self.id = id; self.message.clear();
                        }
                    }
                });
                ui.horizontal(|ui| { ui.label("Atlas name"); ui.text_edit_singleline(&mut self.id); });
                ui.horizontal(|ui| { ui.label("Output PNG"); ui.text_edit_singleline(&mut self.recipe.output); });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.recipe.size).range(1..=4096).prefix("Size "));
                    ui.add(egui::DragValue::new(&mut self.recipe.padding).range(0..=64).prefix("Padding "));
                });
                ui.separator();
                ui.label("Sources (stable labels keep generated clip names)");
                let mut remove = None;
                for (label, path) in &mut self.recipe.sources {
                    ui.horizontal(|ui| {
                        ui.label(label); ui.text_edit_singleline(path);
                        if ui.button("Remove").clicked() { remove = Some(label.clone()); }
                    });
                }
                if let Some(label) = remove { self.recipe.sources.remove(&label); }
                ui.horizontal(|ui| { ui.label("Label"); ui.text_edit_singleline(&mut self.label); });
                ui.horizontal(|ui| { ui.label("PNG path"); ui.text_edit_singleline(&mut self.source); });
                if ui.button("Add Source").clicked() {
                    if self.recipe.sources.contains_key(&self.label) {
                        self.message = "That label already exists; edit its path above or choose another label".into();
                    } else { self.recipe.sources.insert(self.label.clone(), self.source.clone()); }
                }
                ui.separator();
                if ui.button("Build / Rebuild Atlas").clicked() {
                    self.message = match root.ok_or("Open a project first".into()).and_then(|root| {
                        kiln_assets::atlas_project::build(project, root, &self.id, &self.recipe)
                    }) {
                        Ok(()) => {
                            self.recipe = project.atlases[&self.id].clone(); self.built = true;
                            format!("Built {} clips. Save Project to keep this recipe.", self.recipe.sources.len())
                        },
                        Err(error) => error,
                    };
                }
                ui.label(&self.message);
                for label in self.recipe.built_regions.keys() {
                    ui.monospace(format!("Clip: {}", kiln_core::atlas::clip_name(&self.id, label)));
                }
                ui.small("Choose generated clips in the sprite Inspector. Duplicate a generated clip before editing it independently.");
            });
        self.open = open;
    }
}
