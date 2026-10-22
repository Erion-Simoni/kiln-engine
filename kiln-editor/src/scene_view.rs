//! The editor's scene viewport.
//!
//! The editor used to paint scene sprites with `egui`, which meant the editor
//! and the player drew the same scene differently: the editor had no GPU
//! textures and it flipped the vertical axis. The viewport now renders through
//! the engine's own sprite renderer, so both hosts show the same pixels.
//!
//! The scene is drawn into the swap chain inside the scene-view panel. The
//! panel's rectangle is known in `egui` points, so it is converted to physical
//! pixels and letterboxed against the world's aspect ratio before it becomes a
//! viewport.

use std::collections::HashMap;
use std::path::Path;

use kiln_core::Project;
use kiln_render::{camera::Camera, sprite_renderer::SpriteInstance, SpriteRenderer, TextureId};
use wgpu::{Device, Queue};

/// A rectangle on the swap chain, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRect {
    /// Left edge, in pixels from the left of the swap chain.
    pub x: u32,
    /// Top edge, in pixels from the top of the swap chain.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ViewportRect {
    /// Whether the rectangle covers no pixels.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// The letterboxed sub-rectangle of `panel` that has the aspect of the world.
///
/// The world region is centred inside the panel, so a panel that is wider than
/// the world gets bars on the left and the right, and a taller panel gets bars
/// on the top and the bottom. An empty panel or an empty world gives an empty
/// rectangle.
pub fn letterbox(panel_w: u32, panel_h: u32, world_w: f32, world_h: f32) -> ViewportRect {
    if panel_w == 0 || panel_h == 0 || world_w <= 0.0 || world_h <= 0.0 {
        return ViewportRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
    }

    let panel_aspect = panel_w as f32 / panel_h as f32;
    let world_aspect = world_w / world_h;

    let (width, height) = if world_aspect > panel_aspect {
        // The world is wider than the panel: it fills the width.
        (panel_w, (panel_w as f32 / world_aspect).round() as u32)
    } else {
        // The world is taller than the panel: it fills the height.
        ((panel_h as f32 * world_aspect).round() as u32, panel_h)
    };

    let width = width.min(panel_w);
    let height = height.min(panel_h);

    ViewportRect {
        x: (panel_w - width) / 2,
        y: (panel_h - height) / 2,
        width,
        height,
    }
}

/// The camera that shows the world region at the origin.
///
/// This is the same camera the standalone player uses, so a scene looks the
/// same in both hosts: the world origin sits at the bottom left of the view
/// and world Y grows upward.
pub fn world_camera(world_w: f32, world_h: f32) -> Camera {
    Camera::new(world_w, world_h)
}

/// The camera for the screen-space overlay drawn over the scene.
///
/// The overlay is measured in pixels, so the viewport size is the pixel size
/// of the viewport and the origin is its top left corner.
pub fn overlay_camera(viewport: ViewportRect) -> Camera {
    Camera::new_top_left(viewport.width as f32, viewport.height as f32)
}

/// Convert a panel rectangle from `egui` points to physical pixels.
///
/// `egui` measures in points and the swap chain in pixels, so the scale
/// factor has to be applied before the rectangle can be used as a viewport.
pub fn rect_to_pixels(rect: egui::Rect, pixels_per_point: f32) -> ViewportRect {
    let scale = pixels_per_point.max(0.0);
    ViewportRect {
        x: (rect.min.x * scale).round().max(0.0) as u32,
        y: (rect.min.y * scale).round().max(0.0) as u32,
        width: (rect.width() * scale).round().max(0.0) as u32,
        height: (rect.height() * scale).round().max(0.0) as u32,
    }
}

/// A project texture that has been uploaded to the GPU.
#[derive(Debug, Clone, Copy)]
pub struct SceneTexture {
    /// The texture handle used by the sprite renderer.
    pub id: TextureId,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

/// The textures of a project, keyed by their asset path.
#[derive(Debug, Clone, Default)]
pub struct SceneTextures {
    textures: HashMap<String, SceneTexture>,
}

impl SceneTextures {
    /// Look up a texture by its asset path.
    pub fn get(&self, path: &str) -> Option<&SceneTexture> {
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

    /// Decode and upload every texture the project refers to.
    ///
    /// The files are read from `root`, the project directory. Textures that
    /// are missing or cannot be decoded are reported in the returned list and
    /// the entity falls back to the white texture.
    pub fn upload_for_project(
        renderer: &mut SpriteRenderer,
        device: &Device,
        queue: &Queue,
        project: &Project,
        root: &Path,
    ) -> (Self, Vec<String>) {
        let mut library = SceneTextures::default();
        let mut errors = Vec::new();

        for path in texture_paths(project) {
            let full = root.join(&path);
            let bytes = match std::fs::read(&full) {
                Ok(bytes) => bytes,
                Err(e) => {
                    errors.push(format!("texture asset '{path}' not found: {e}"));
                    continue;
                }
            };
            let image = match kiln_assets::decode_png(&bytes) {
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
                SceneTexture {
                    id,
                    width: image.width,
                    height: image.height,
                },
            );
        }

        (library, errors)
    }
}

/// Collect every distinct texture path the project's scenes and tilesets use.
pub fn texture_paths(project: &Project) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    let mut push = |path: &str| {
        if !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    };

    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let kiln_core::project::Component::Sprite {
                    texture: Some(path),
                    ..
                } = component
                {
                    push(path);
                }
            }
        }
    }
    for tileset in project.tilesets.values() {
        push(&tileset.texture);
    }

    paths
}

/// The grid lines of the scene view, as world-space quads.
///
/// The lines sit on the world grid, so they line up with the world however the
/// viewport is scaled.
pub fn grid_lines(
    world_w: f32,
    world_h: f32,
    spacing: f32,
    color: [f32; 4],
) -> Vec<SpriteInstance> {
    let mut lines = Vec::new();
    if spacing <= 0.0 || world_w <= 0.0 || world_h <= 0.0 {
        return lines;
    }

    let thickness = (spacing / 100.0).max(1.0);
    let mut x = 0.0;
    while x <= world_w {
        lines.push(SpriteInstance::colored(
            [x, world_h / 2.0],
            [thickness, world_h],
            color,
        ));
        x += spacing;
    }
    let mut y = 0.0;
    while y <= world_h {
        lines.push(SpriteInstance::colored(
            [world_w / 2.0, y],
            [world_w, thickness],
            color,
        ));
        y += spacing;
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::project::{Component, Entity, Scene};

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
            script: None,
        };
        project.scenes.insert("main".to_string(), scene);
        project
    }

    #[test]
    fn letterbox_of_an_empty_panel_is_empty() {
        assert!(letterbox(0, 100, 640.0, 480.0).is_empty());
        assert!(letterbox(100, 100, 0.0, 480.0).is_empty());
    }

    #[test]
    fn letterbox_fills_the_panel_when_the_aspects_match() {
        let rect = letterbox(640, 480, 640.0, 480.0);
        assert_eq!(
            rect,
            ViewportRect {
                x: 0,
                y: 0,
                width: 640,
                height: 480
            }
        );
    }

    #[test]
    fn a_wide_panel_gets_side_bars() {
        // The world is 4:3, the panel is 16:9, so the world fills the height
        // and is centred horizontally.
        let rect = letterbox(1600, 900, 640.0, 480.0);
        assert_eq!(rect.height, 900);
        assert_eq!(rect.width, 1200);
        assert_eq!(rect.x, 200);
        assert_eq!(rect.y, 0);
    }

    #[test]
    fn a_tall_panel_gets_top_and_bottom_bars() {
        let rect = letterbox(640, 900, 640.0, 480.0);
        assert_eq!(rect.width, 640);
        assert_eq!(rect.height, 480);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 210);
    }

    #[test]
    fn the_world_camera_matches_the_player_camera() {
        // The same world region must project the same way in both hosts.
        let editor = world_camera(640.0, 480.0);
        let player = Camera::new(640.0, 480.0);
        assert_eq!(editor.projection_matrix(), player.projection_matrix());
    }

    #[test]
    fn the_overlay_camera_starts_at_the_top_left() {
        let camera = overlay_camera(ViewportRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        });
        let ndc = camera.projection_matrix() * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        // Screen space: the origin is at the left edge and at the top edge.
        assert!(ndc.x < -0.9, "the origin is at the left edge: {}", ndc.x);
        assert!(ndc.y > 0.9, "the origin is at the top edge: {}", ndc.y);
    }

    #[test]
    fn a_panel_rect_is_scaled_to_pixels() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 50.0));
        let pixels = rect_to_pixels(rect, 2.0);
        assert_eq!(
            pixels,
            ViewportRect {
                x: 20,
                y: 40,
                width: 200,
                height: 100
            }
        );
    }

    #[test]
    fn texture_paths_are_collected_from_scenes_and_tilesets() {
        let mut project = project_with_texture("sprites/hero.png");
        project.tilesets.insert(
            "terrain".to_string(),
            kiln_core::TileSet {
                texture: "tiles/terrain.png".to_string(),
                tile_size: [16, 16],
                columns: 2,
                rows: 2,
                solid: Vec::new(),
            },
        );
        let paths = texture_paths(&project);
        assert!(paths.contains(&"sprites/hero.png".to_string()));
        assert!(paths.contains(&"tiles/terrain.png".to_string()));
    }

    #[test]
    fn a_project_without_textures_has_no_paths() {
        assert!(texture_paths(&Project::default()).is_empty());
    }

    #[test]
    fn the_grid_covers_the_world() {
        let lines = grid_lines(200.0, 100.0, 50.0, [1.0; 4]);
        // Five vertical lines (0, 50, 100, 150, 200) and three horizontal ones.
        assert_eq!(lines.len(), 8);
        for line in &lines {
            assert!(line.position[0] >= 0.0 && line.position[0] <= 200.0);
            assert!(line.position[1] >= 0.0 && line.position[1] <= 100.0);
        }
    }

    #[test]
    fn an_empty_grid_has_no_lines() {
        assert!(grid_lines(200.0, 100.0, 0.0, [1.0; 4]).is_empty());
    }
}
