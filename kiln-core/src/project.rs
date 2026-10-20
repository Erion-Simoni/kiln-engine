//! Project file format

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Transform {
        x: f32,
        y: f32,
        rotation: f32,
        scale: f32,
    },
    Sprite {
        color: [f32; 4],
        /// Asset path of the sprite texture (e.g. `"sprites/hero.png"`).
        ///
        /// `None` means the sprite is drawn with the built-in white texel and
        /// tinted by `color`.
        #[serde(default)]
        texture: Option<String>,
    },
}

impl Project {
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
            id: uuid::Uuid::new_v4().to_string(),
            name: "Untitled Project".to_string(),
            version: "0.1.0".to_string(),
            publisher: String::new(),
            icon: String::new(),
            features: vec![],
            window: WindowSettings::default(),
            mobile: MobileSettings::default(),
            scenes: HashMap::new(),
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
