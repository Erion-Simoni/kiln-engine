//! Project file format

use crate::clip::Clip;
use crate::input::ActionMap;
use crate::next_id;
use crate::prefab::{Prefab, PrefabOverride};
use crate::tilemap::{TileMap, TileSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub icon: String,
    pub features: Vec<String>,
    #[serde(default)]
    pub window: WindowSettings,
    #[serde(default)]
    pub mobile: MobileSettings,
    #[serde(default)]
    pub scenes: HashMap<String, Scene>,
    /// Gameplay camera configuration keyed by scene ID.
    #[serde(default)]
    pub cameras: HashMap<String, crate::camera::CameraSettings>,
    /// Tilemaps of the project, keyed by their id.
    #[serde(default)]
    pub tilemaps: HashMap<String, TileMap>,
    /// Saved terrain palettes indexed by cardinal-neighbor occupancy.
    #[serde(default)]
    pub terrains: HashMap<String, crate::tilemap::TerrainSet>,
    /// Tilesets of the project, keyed by their id.
    #[serde(default)]
    pub tilesets: HashMap<String, TileSet>,
    /// Animation clips of the project, keyed by their id.
    #[serde(default)]
    pub animation_clips: HashMap<String, Clip>,
    /// Shader programs of the project, keyed by their id.
    #[serde(default)]
    pub shaders: HashMap<String, Shader>,
    /// 2D sprite materials of the project, keyed by their id.
    #[serde(default)]
    pub materials: HashMap<String, SpriteMaterial>,
    /// Audio assets of the project, keyed by their name.
    #[serde(default)]
    pub audio_assets: Vec<AudioAsset>,
    /// Saved author defaults for master and named mixer buses.
    #[serde(default)]
    pub audio_mixer: crate::audio::AudioMixerSettings,
    /// Saved in-game screens and UI font resources.
    #[serde(default)]
    pub ui: crate::ui::UiDocument,
    /// SDF font assets of the project, keyed by their id.
    #[serde(default)]
    pub sdf_fonts: HashMap<String, SdfFont>,
    /// Reusable entity templates keyed by stable prefab id.
    #[serde(default)]
    pub prefabs: HashMap<String, Prefab>,
    /// Stable asset IDs keyed by project-relative asset path.
    #[serde(default)]
    pub asset_ids: HashMap<String, String>,
    /// Import options keyed by stable asset ID.
    #[serde(default)]
    pub asset_import_settings: HashMap<String, AssetImportSettings>,
    /// Input action map for rebindable controls.
    #[serde(default)]
    pub action_map: ActionMap,
}

/// Per-asset texture import options stored in the project manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetImportSettings {
    /// Use nearest-neighbor sampling for pixel art.
    pub nearest_filter: bool,
    /// Generate mipmaps when supported by the renderer.
    pub generate_mipmaps: bool,
    /// Treat the image as a pixel-art sprite.
    pub is_sprite: bool,
}

impl Default for AssetImportSettings {
    fn default() -> Self {
        Self {
            nearest_filter: false,
            generate_mipmaps: true,
            is_sprite: false,
        }
    }
}

impl Project {
    /// Effective nearest-sampling/mipmap settings; legacy projects retain their
    /// original nearest/no-mip appearance until import options are assigned.
    pub fn texture_options(&self, path: &str) -> [bool; 2] {
        self.asset_ids
            .get(path)
            .and_then(|id| self.asset_import_settings.get(id))
            .map(|settings| [settings.nearest_filter, settings.generate_mipmaps])
            .unwrap_or([true, false])
    }
}

/// An audio asset reference in the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioAsset {
    /// Unique audio identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Asset path of the audio file.
    pub path: String,
    /// Whether this is music (looped, single instance) or sound effect.
    #[serde(default)]
    pub is_music: bool,
    /// Volume (0.0 to 1.0).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Empty selects Music or SFX according to is_music.
    #[serde(default)]
    pub bus: String,
    /// Repeat on completion. Missing legacy values retain looping music.
    #[serde(default = "default_audio_loop")]
    pub looped: bool,
}

fn default_audio_loop() -> bool {
    true
}

impl Default for AudioAsset {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            path: String::new(),
            is_music: false,
            volume: 1.0,
            bus: String::new(),
            looped: true,
        }
    }
}

impl AudioAsset {
    /// Effective routing for legacy and newly authored assets.
    pub fn bus_name(&self) -> &str {
        if self.bus.is_empty() {
            if self.is_music {
                "Music"
            } else {
                "SFX"
            }
        } else {
            &self.bus
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    #[serde(default = "default_window_width")]
    pub width: u32,
    #[serde(default = "default_window_height")]
    pub height: u32,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub borderless: bool,
    #[serde(default)]
    pub vsync: bool,
    #[serde(default = "default_high_dpi")]
    pub high_dpi: bool,
    #[serde(default)]
    pub resizable: bool,
    #[serde(default)]
    pub monitor: u32,
}

fn default_window_width() -> u32 {
    1280
}
fn default_window_height() -> u32 {
    720
}
fn default_high_dpi() -> bool {
    true
}
fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MobileSettings {
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub ios: IosSettings,
    #[serde(default)]
    pub android: AndroidSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Portrait,
    #[default]
    Landscape,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosSettings {
    #[serde(default)]
    pub bundle_identifier: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub privacy_descriptions: PrivacyDescriptions,
    #[serde(default)]
    pub launch_screen: LaunchScreen,
    #[serde(default)]
    pub supports_notch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrivacyDescriptions {
    #[serde(default)]
    pub camera: String,
    #[serde(default)]
    pub microphone: String,
    #[serde(default)]
    pub photo_library: String,
    #[serde(default)]
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchScreen {
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub background_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidSettings {
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub target_sdk: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub nested_scenes: Vec<NestedScene>,
    /// Path of the scene's gameplay script, relative to the project root.
    ///
    /// The script is a KilnScript source file that the runtime executes once
    /// per frame. Scenes without a script are static.
    #[serde(default)]
    pub script: Option<String>,
}

/// Reference to another scene that is loaded as part of this scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedScene {
    pub scene_id: String,
    #[serde(default)]
    pub additive: bool,
    #[serde(default)]
    pub offset: SceneOffset,
}

/// Offset for nested scene loading
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneOffset {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

impl Default for SceneOffset {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    /// Sprite drawing geometry, independent of the referenced texture.
    SpriteLayout {
        settings: crate::visual::SpriteLayout,
    },
    /// Saved CPU particle effect, simulated by the shared runtime.
    Particles {
        effect: crate::visual::ParticleEffect,
    },

    /// Authored physics body and its collision shape.
    Physics {
        body: crate::physics_data::RigidBody2D,
        collider: crate::physics_data::Collider2D,
    },
    /// Authored physical joint between stable entity source IDs.
    Joint { joint: crate::physics_data::Joint2D },
    /// Entity-attached gameplay program with per-instance numeric properties.
    Script {
        /// Project-relative script resource path.
        path: String,
        /// Inspector-exposed initial property values.
        #[serde(default)]
        properties: HashMap<String, f64>,
    },
    Transform {
        x: f32,
        y: f32,
        rotation: f32,
        scale: f32,
    },
    Sprite {
        color: [f32; 4],
        /// Authored width in world units. Older projects default to 32.
        #[serde(default = "default_sprite_dimension")]
        width: f32,
        /// Authored height in world units. Older projects default to 32.
        #[serde(default = "default_sprite_dimension")]
        height: f32,
        /// Asset path of the sprite texture (e.g. `"sprites/hero.png"`).
        ///
        /// `None` means the sprite is drawn with the built-in white texel and
        /// tinted by `color`.
        #[serde(default)]
        texture: Option<String>,
        /// Sorting layer — sprites on lower layers are drawn first.
        /// Preserves painter order for transparent sprites across textures.
        #[serde(default)]
        layer: f32,
        /// Optional material reference for custom shader rendering.
        #[serde(default)]
        material: Option<String>,
        /// Optional name of an animation clip from `animation_clips` to play
        /// on this sprite.
        #[serde(default)]
        animation_clip: Option<String>,
    },
    /// A tilemap instance placed in the scene.
    Tilemap {
        /// Id of the tilemap in [`Project::tilemaps`].
        map: String,
    },
    /// The gameplay role of the entity in a scripted scene.
    ///
    /// Roles let a scene's script find entities by purpose instead of by
    /// index, e.g. `"player"` or `"coin"`.
    Role {
        /// Role name, e.g. `"player"` or `"coin"`.
        role: String,
    },
    /// Link a scene entity to a reusable prefab definition.
    PrefabInstance {
        /// Stable id of the definition in [`Project::prefabs`].
        prefab_id: String,
        /// Per-instance property overrides.
        #[serde(default)]
        overrides: PrefabOverride,
    },
    /// Parent-child relationship for entity hierarchy.
    Parent {
        /// Parent entity's source id.
        parent: String,
    },
}

fn default_sprite_dimension() -> f32 {
    32.0
}

impl Project {
    /// The id of the scene a project starts in.
    ///
    /// This is the first scene by id, so the player and the editor start in
    /// the same place.
    pub fn default_scene_id(&self) -> Option<String> {
        let mut ids: Vec<&String> = self.scenes.keys().collect();
        ids.sort();
        ids.first().map(|id| (*id).clone())
    }
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        toml::from_str(toml_str).map_err(|e| e.to_string())
    }

    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: next_id().to_string(),
            name: "Untitled Project".to_string(),
            version: "0.1.0".to_string(),
            publisher: String::new(),
            icon: String::new(),
            features: vec![],
            window: WindowSettings::default(),
            mobile: MobileSettings::default(),
            scenes: HashMap::new(),
            cameras: HashMap::new(),
            tilemaps: HashMap::new(),
            terrains: HashMap::new(),
            tilesets: HashMap::new(),
            animation_clips: HashMap::new(),
            shaders: HashMap::new(),
            materials: HashMap::new(),
            audio_assets: vec![],
            audio_mixer: Default::default(),
            ui: Default::default(),
            sdf_fonts: HashMap::new(),
            prefabs: HashMap::new(),
            asset_ids: HashMap::new(),
            asset_import_settings: HashMap::new(),
            action_map: ActionMap::new(),
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: default_window_width(),
            height: default_window_height(),
            fullscreen: false,
            borderless: false,
            vsync: true,
            high_dpi: default_high_dpi(),
            resizable: true,
            monitor: 0,
        }
    }
}

impl Default for IosSettings {
    fn default() -> Self {
        Self {
            bundle_identifier: "com.example.game".to_string(),
            permissions: vec![],
            privacy_descriptions: PrivacyDescriptions::default(),
            launch_screen: LaunchScreen::default(),
            supports_notch: true,
        }
    }
}

impl Default for LaunchScreen {
    fn default() -> Self {
        Self {
            image: String::new(),
            background_color: "#000000".to_string(),
        }
    }
}

impl Default for AndroidSettings {
    fn default() -> Self {
        Self {
            package_name: "com.example.game".to_string(),
            permissions: vec![],
            target_sdk: 33, // Android 13
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_toml() {
        let proj = Project {
            name: "TestGame".to_string(),
            ..Project::default()
        };
        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");
        assert_eq!(loaded.name, "TestGame");
        assert_eq!(loaded.version, "0.1.0");
        assert_eq!(loaded.window.width, 1280);
        assert_eq!(loaded.window.height, 720);
        assert_eq!(loaded.mobile.orientation, Orientation::Landscape);
    }

    #[test]
    fn default_has_no_scenes() {
        let proj = Project::default();
        assert!(proj.scenes.is_empty());
    }

    #[test]
    fn window_settings_round_trip() {
        let mut proj = Project::default();
        proj.window.width = 1920;
        proj.window.height = 1080;
        proj.window.fullscreen = true;
        proj.window.vsync = false;
        proj.window.monitor = 1;

        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        assert_eq!(loaded.window.width, 1920);
        assert_eq!(loaded.window.height, 1080);
        assert!(loaded.window.fullscreen);
        assert!(!loaded.window.vsync);
        assert_eq!(loaded.window.monitor, 1);
    }

    #[test]
    fn packaging_fields_round_trip() {
        let proj = Project {
            publisher: "MyStudio".to_string(),
            icon: "icon.png".to_string(),
            ..Project::default()
        };
        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        assert_eq!(loaded.publisher, "MyStudio");
        assert_eq!(loaded.icon, "icon.png");
    }

    #[test]
    fn prefab_definitions_round_trip_in_project_manifest() {
        let entity = Entity {
            id: "enemy-template".into(),
            name: "Enemy".into(),
            components: vec![Component::Sprite {
                color: [0.8, 0.2, 0.1, 1.0],
                width: 48.0,
                height: 48.0,
                texture: Some("art/enemy.png".into()),
                layer: 3.0,
                material: None,
                animation_clip: Some("walk".into()),
            }],
        };
        let prefab = Prefab {
            id: "prefab_enemy".into(),
            name: "Enemy".into(),
            root_entity: entity,
        };
        let mut project = Project::default();
        project.prefabs.insert(prefab.id.clone(), prefab);

        let toml = project.to_toml().unwrap();
        let loaded = Project::from_toml(&toml).unwrap();
        assert_eq!(loaded.prefabs["prefab_enemy"].root_entity.name, "Enemy");
        assert!(matches!(
            &loaded.prefabs["prefab_enemy"].root_entity.components[0],
            Component::Sprite {
                texture: Some(path),
                animation_clip: Some(clip),
                width: 48.0,
                ..
            } if path == "art/enemy.png" && clip == "walk"
        ));
    }

    #[test]
    fn stable_asset_ids_and_import_settings_round_trip() {
        let mut project = Project::default();
        project
            .asset_ids
            .insert("art/enemy.png".into(), "asset_stable_42".into());
        project.asset_import_settings.insert(
            "asset_stable_42".into(),
            AssetImportSettings {
                nearest_filter: true,
                generate_mipmaps: false,
                is_sprite: true,
            },
        );
        let loaded = Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert_eq!(loaded.asset_ids["art/enemy.png"], "asset_stable_42");
        assert!(loaded.asset_import_settings["asset_stable_42"].nearest_filter);
    }

    #[test]
    fn older_project_manifests_default_to_an_empty_prefab_library() {
        let legacy = r#"
id = "legacy"
name = "Legacy"
version = "0.1.0"
features = []
"#;
        let loaded = Project::from_toml(legacy).unwrap();
        assert!(loaded.prefabs.is_empty());
        assert!(loaded.asset_ids.is_empty());
        assert!(loaded.asset_import_settings.is_empty());
    }

    #[test]
    fn legacy_sprite_dimensions_default_to_thirty_two() {
        let sprite: Component =
            serde_json::from_str(r#"{"type":"Sprite","color":[1.0,1.0,1.0,1.0],"texture":null}"#)
                .unwrap();
        assert!(matches!(
            sprite,
            Component::Sprite {
                width: 32.0,
                height: 32.0,
                ..
            }
        ));
    }

    #[test]
    fn nested_scene_round_trip() {
        let scene = Scene {
            id: "scene1".to_string(),
            name: "MainScene".to_string(),
            entities: vec![],
            nested_scenes: vec![NestedScene {
                scene_id: "scene2".to_string(),
                additive: true,
                offset: SceneOffset { x: 100.0, y: 200.0 },
            }],
            script: None,
        };

        let mut proj = Project::default();
        proj.scenes.insert("scene1".to_string(), scene);

        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        let loaded_scene = loaded.scenes.get("scene1").unwrap();
        assert_eq!(loaded_scene.nested_scenes.len(), 1);
        assert_eq!(loaded_scene.nested_scenes[0].scene_id, "scene2");
        assert!(loaded_scene.nested_scenes[0].additive);
        assert_eq!(loaded_scene.nested_scenes[0].offset.x, 100.0);
        assert_eq!(loaded_scene.nested_scenes[0].offset.y, 200.0);
    }

    #[test]
    fn scene_offset_default() {
        let offset = SceneOffset::default();
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
    }

    #[test]
    fn project_from_ron() {
        let ron_str = r#"
        (
            id: "test-project",
            name: "Test Project",
            version: "0.1.0",
            publisher: "",
            icon: "",
            features: [],
            window: (
                width: 1280,
                height: 720,
                fullscreen: false,
                borderless: false,
                vsync: true,
                high_dpi: true,
                resizable: true,
                monitor: 0,
            ),
            mobile: (
                orientation: landscape,
                ios: (
                    bundle_identifier: "com.test.game",
                    permissions: [],
                    privacy_descriptions: (
                        camera: "",
                        microphone: "",
                        photo_library: "",
                        location: "",
                    ),
                    launch_screen: (
                        image: "",
                        background_color: "black",
                    ),
                    supports_notch: true,
                ),
                android: (
                    package_name: "com.test.game",
                    permissions: [],
                    target_sdk: 33,
                ),
            ),
            scenes: {},
        )
        "#;

        let project: Project = ron::from_str(ron_str).expect("parse RON");
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.window.width, 1280);
        assert_eq!(project.mobile.orientation, Orientation::Landscape);
        assert_eq!(project.mobile.ios.bundle_identifier, "com.test.game");
        assert_eq!(project.mobile.android.package_name, "com.test.game");
    }

    #[test]
    fn mobile_settings_round_trip() {
        let mut proj = Project::default();
        proj.mobile.orientation = Orientation::Portrait;
        proj.mobile.ios.bundle_identifier = "com.example.mygame".to_string();
        proj.mobile
            .ios
            .permissions
            .push("NSCameraUsageDescription".to_string());
        proj.mobile.android.package_name = "com.example.mygame".to_string();
        proj.mobile.android.target_sdk = 34;

        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        assert_eq!(loaded.mobile.orientation, Orientation::Portrait);
        assert_eq!(loaded.mobile.ios.bundle_identifier, "com.example.mygame");
        assert_eq!(loaded.mobile.ios.permissions.len(), 1);
        assert_eq!(loaded.mobile.android.package_name, "com.example.mygame");
        assert_eq!(loaded.mobile.android.target_sdk, 34);
    }

    #[test]
    fn mobile_settings_defaults() {
        let mobile = MobileSettings::default();
        assert_eq!(mobile.orientation, Orientation::Landscape);
        assert_eq!(mobile.ios.bundle_identifier, "com.example.game");
        assert_eq!(mobile.android.package_name, "com.example.game");
        assert_eq!(mobile.android.target_sdk, 33);
    }
}

/// A WGSL shader program for custom sprite rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shader {
    /// Unique shader identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// WGSL source code for the fragment shader.
    /// The shader receives position, size, color, uv_rect as inputs
    /// and outputs a color.
    pub source: String,
    /// Entry point function name.
    #[serde(default)]
    pub entry_point: String,
    /// Shader parameters exposed in the editor.
    #[serde(default)]
    pub parameters: HashMap<String, MaterialParam>,
}

/// A 2D sprite material referencing a [`Shader`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpriteMaterial {
    /// Unique material identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Id of the shader in [`Project::shaders`].
    #[serde(default)]
    pub shader: String,
    /// Parameter overrides.
    #[serde(default)]
    pub parameters: HashMap<String, MaterialParam>,
}

/// Editor alias for a 2D sprite material.
pub type Material = SpriteMaterial;

/// A shader/material parameter value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialParam {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
    Texture(String),
}

/// Signed Distance Field font asset for high-quality text rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdfFont {
    /// Unique font identifier.
    pub id: String,
    /// Human-readable font name.
    pub name: String,
    /// Asset path to the SDF font texture atlas.
    pub texture_path: String,
    /// Atlas texture width in pixels.
    pub atlas_width: u32,
    /// Atlas texture height in pixels.
    pub atlas_height: u32,
    /// Glyph metrics: character code -> glyph metrics.
    #[serde(default)]
    pub glyphs: HashMap<char, GlyphMetrics>,
    /// Base line height in pixels.
    pub line_height: f32,
    /// Space character advance width in pixels.
    pub space_advance: f32,
    /// Tab width in spaces.
    #[serde(default)]
    pub tab_width: u32,
}

/// Glyph metrics for an SDF font glyph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphMetrics {
    /// Character code.
    pub codepoint: char,
    /// Glyph position in the atlas texture (x, y).
    pub offset: [f32; 2],
    /// Glyph size in pixels (width, height).
    pub size: [f32; 2],
    /// Glyph bearing (offset from origin to left edge).
    pub bearing: [f32; 2],
    /// Advance width for the next character.
    pub advance: f32,
}
impl Default for GlyphMetrics {
    fn default() -> Self {
        GlyphMetrics {
            codepoint: ' ',
            offset: [0.0, 0.0],
            size: [0.0, 0.0],
            bearing: [0.0, 0.0],
            advance: 0.0,
        }
    }
}
