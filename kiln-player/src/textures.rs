//! Decoding image assets and uploading them as GPU textures.
//!
//! The player keeps every decoded texture in a [`TextureLibrary`], keyed by
//! the asset path used in project data (for example `sprites/hero.png`).
//! [`TextureLibrary::get`] resolves a path to the uploaded [`TextureId`]
//! plus the pixel size of the image, which the spawner uses as the sprite
//! size in world units.

use kiln_assets::{decode_png, Image};
use kiln_core::project::Component;
use kiln_core::Project;
use kiln_render::{SpriteRenderer, TextureId};
use std::collections::HashMap;
use wgpu::{Device, Queue};

/// A texture that has been uploaded to the GPU.
#[derive(Debug, Clone, Copy)]
pub struct GpuTexture {
    /// Handle used by the sprite renderer.
    pub id: TextureId,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

/// Textures uploaded for a project, keyed by asset path.
#[derive(Debug, Default)]
pub struct TextureLibrary {
    textures: HashMap<String, GpuTexture>,
}

impl TextureLibrary {
    /// Look up a texture by asset path.
    pub fn get(&self, path: &str) -> Option<&GpuTexture> {
        self.textures.get(path)
    }

    /// Number of uploaded textures.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Whether no texture has been uploaded.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Decode and upload every texture referenced by the project.
    ///
    /// Returns the library together with a human readable list of failures
    /// (missing asset, invalid PNG); the player logs those and keeps going
    /// with the white fallback texture.
    pub fn upload_for_project(
        renderer: &mut SpriteRenderer,
        device: &Device,
        queue: &Queue,
        project: &Project,
        assets: &HashMap<String, Vec<u8>>,
    ) -> (Self, Vec<String>) {
        let mut library = TextureLibrary::default();
        let mut errors = Vec::new();

        for path in texture_paths(project) {
            let Some(bytes) = find_asset(assets, &path) else {
                errors.push(format!("texture asset '{path}' not found"));
                continue;
            };

            let image: Image = match decode_png(bytes) {
                Ok(image) => image,
                Err(e) => {
                    errors.push(format!("texture asset '{path}' failed to decode: {e}"));
                    continue;
                }
            };

            let id = renderer.create_texture_rgba(
                device,
                queue,
                &path,
                image.width,
                image.height,
                &image.pixels,
            );

            library.textures.insert(
                path,
                GpuTexture {
                    id,
                    width: image.width,
                    height: image.height,
                },
            );
        }

        (library, errors)
    }
}

/// Collect every distinct texture path referenced by the project scenes.
pub fn texture_paths(project: &Project) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();

    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let Component::Sprite {
                    texture: Some(path),
                    ..
                } = component
                {
                    if !paths.iter().any(|p| p == path) {
                        paths.push(path.clone());
                    }
                }
            }
        }
    }

    paths.sort();
    paths
}

/// Find the bytes of an asset, falling back to a match by file name.
///
/// Archives store entries by file name only, so `sprites/hero.png` may be
/// stored as `hero.png`.
fn find_asset<'a>(assets: &'a HashMap<String, Vec<u8>>, path: &str) -> Option<&'a [u8]> {
    if let Some(bytes) = assets.get(path) {
        return Some(bytes);
    }

    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    assets
        .iter()
        .find(|(name, _)| {
            name.rsplit(['/', '\\'])
                .next()
                .map(|n| n == file_name)
                .unwrap_or(false)
        })
        .map(|(_, bytes)| bytes.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::{Entity, Scene};

    fn project_with_texture(path: &str) -> Project {
        let mut project = Project::default();
        let scene = Scene {
            id: "main".to_string(),
            name: "Main".to_string(),
            entities: vec![Entity {
                id: "e1".to_string(),
                name: "Player".to_string(),
                components: vec![Component::Sprite {
                    color: [1.0, 1.0, 1.0, 1.0],
                    texture: Some(path.to_string()),
                }],
            }],
            nested_scenes: Vec::new(),
        };
        project.scenes.insert("main".to_string(), scene);
        project
    }

    #[test]
    fn collects_texture_paths() {
        let project = project_with_texture("sprites/hero.png");
        assert_eq!(
            texture_paths(&project),
            vec!["sprites/hero.png".to_string()]
        );
    }

    #[test]
    fn project_without_textures_has_no_paths() {
        let project = Project::default();
        assert!(texture_paths(&project).is_empty());
    }

    #[test]
    fn finds_asset_by_exact_path() {
        let mut assets = HashMap::new();
        assets.insert("sprites/hero.png".to_string(), vec![1, 2, 3]);
        assert_eq!(
            find_asset(&assets, "sprites/hero.png"),
            Some(&[1u8, 2, 3][..])
        );
    }

    #[test]
    fn finds_asset_by_file_name_fallback() {
        let mut assets = HashMap::new();
        assets.insert("hero.png".to_string(), vec![7]);
        assert_eq!(find_asset(&assets, "sprites/hero.png"), Some(&[7u8][..]));
    }

    #[test]
    fn missing_asset_returns_none() {
        let assets = HashMap::new();
        assert!(find_asset(&assets, "missing.png").is_none());
    }
}
