//! Project file format

use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    #[serde(default)]
    pub scenes: HashMap<String, Scene>,
}

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
            features: vec![],
            scenes: HashMap::new(),
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
    }

    #[test]
    fn default_has_no_scenes() {
        let proj = Project::default();
        assert!(proj.scenes.is_empty());
    }
}

