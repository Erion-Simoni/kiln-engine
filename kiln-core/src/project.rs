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
}

fn default_window_width() -> u32 { 1280 }
fn default_window_height() -> u32 { 720 }
fn default_high_dpi() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub entities: Vec<Entity>,
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
    Transform { x: f32, y: f32, rotation: f32, scale: f32 },
    Sprite { color: [f32; 4] },
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_toml() {
        let mut proj = Project::default();
        proj.name = "TestGame".to_string();
        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");
        assert_eq!(loaded.name, "TestGame");
        assert_eq!(loaded.version, "0.1.0");
        assert_eq!(loaded.window.width, 1280);
        assert_eq!(loaded.window.height, 720);
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

        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        assert_eq!(loaded.window.width, 1920);
        assert_eq!(loaded.window.height, 1080);
        assert_eq!(loaded.window.fullscreen, true);
        assert_eq!(loaded.window.vsync, false);
    }

    #[test]
    fn packaging_fields_round_trip() {
        let mut proj = Project::default();
        proj.publisher = "MyStudio".to_string();
        proj.icon = "icon.png".to_string();

        let toml = proj.to_toml().expect("serialize");
        let loaded = Project::from_toml(&toml).expect("deserialize");

        assert_eq!(loaded.publisher, "MyStudio");
        assert_eq!(loaded.icon, "icon.png");
    }
}

