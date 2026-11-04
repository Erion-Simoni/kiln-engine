//! Cached decoded project images for animation, sprite and tilemap authoring.
use std::{collections::HashMap, path::Path, time::SystemTime};

#[derive(Clone)]
pub struct PreviewImage {
    pub texture: egui::TextureHandle,
    pub size: [u32; 2],
}
#[derive(Default)]
pub struct PreviewImages {
    entries: HashMap<String, (Option<SystemTime>, PreviewImage)>,
}
impl std::fmt::Debug for PreviewImages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewImages")
            .field("images", &self.entries.len())
            .finish()
    }
}
impl PreviewImages {
    pub fn load(
        &mut self,
        ctx: &egui::Context,
        root: Option<&Path>,
        path: &str,
    ) -> Result<PreviewImage, String> {
        let root = root.ok_or("Open a saved project to preview textures")?;
        if path.is_empty()
            || Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("Choose a project-relative image path".into());
        }
        let full = root.join(path);
        let modified = std::fs::metadata(&full)
            .map_err(|e| format!("{path}: {e}"))?
            .modified()
            .ok();
        let key = full.to_string_lossy().to_string();
        if let Some((previous, image)) = self.entries.get(&key) {
            if previous == &modified {
                return Ok(image.clone());
            }
        }
        let image = kiln_assets::decode_png(&std::fs::read(&full).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let size = [image.width, image.height];
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [size[0] as usize, size[1] as usize],
            &image.pixels,
        );
        let preview = PreviewImage {
            texture: ctx.load_texture(&key, color, egui::TextureOptions::NEAREST),
            size,
        };
        self.entries.insert(key, (modified, preview.clone()));
        Ok(preview)
    }
}
