//! Prefab system for reusable entity templates

use crate::next_id;
use crate::project::{Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A prefab is a reusable template for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    pub id: String,
    pub name: String,
    pub root_entity: Entity,
}

/// Override values for a specific prefab instance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrefabOverride {
    pub entity_id: String,
    pub overrides: HashMap<String, ComponentOverride>,
}

/// Override for a specific component property
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComponentOverride {
    /// Per-instance material and animation clip: absent inherits, false clears.
    SpriteResources {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "texture_value"
        )]
        material: Option<Option<String>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "texture_value"
        )]
        animation_clip: Option<Option<String>>,
    },
    /// Per-instance dimensions and draw layer; missing values inherit.
    SpriteGeometry {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layer: Option<f32>,
    },
    Transform {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rotation: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale: Option<f32>,
    },
    Sprite {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<[f32; 4]>,
        /// Inherit when absent, set a path, or explicitly clear (`false` in saved data).
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "texture_value"
        )]
        texture: Option<Option<String>>,
    },
}

// TOML has no null: false is an explicit clear, omitted means inherit.
mod texture_value {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<Option<String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(Some(path)) => serializer.serialize_str(path),
            Some(None) => serializer.serialize_bool(false),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<String>>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Path(String),
            Clear(bool),
            Inherit(()),
        }
        match Value::deserialize(deserializer)? {
            Value::Path(path) => Ok(Some(Some(path))),
            Value::Clear(false) => Ok(Some(None)),
            Value::Clear(true) => Err(serde::de::Error::custom(
                "Resource override must be a string or false to clear",
            )),
            Value::Inherit(()) => Ok(None),
        }
    }
}

/// Tracks which prefab an entity instance came from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    pub prefab_id: String,
    pub entity_id: String,
    pub overrides: PrefabOverride,
}

impl Prefab {
    pub fn new(name: String, root_entity: Entity) -> Self {
        Self {
            id: next_id().to_string(),
            name,
            root_entity,
        }
    }

    /// Apply overrides to an entity from this prefab
    pub fn apply_overrides(&self, entity: &mut Entity, overrides: &PrefabOverride) {
        for override_value in overrides.overrides.values() {
            for component in &mut entity.components {
                match (component, override_value) {
                    (
                        Component::Transform {
                            x,
                            y,
                            rotation,
                            scale,
                        },
                        ComponentOverride::Transform {
                            x: ox,
                            y: oy,
                            rotation: orot,
                            scale: oscale,
                        },
                    ) => {
                        if let Some(val) = ox {
                            *x = *val;
                        }
                        if let Some(val) = oy {
                            *y = *val;
                        }
                        if let Some(val) = orot {
                            *rotation = *val;
                        }
                        if let Some(val) = oscale {
                            *scale = *val;
                        }
                    }
                    (
                        Component::Sprite {
                            width,
                            height,
                            layer,
                            ..
                        },
                        ComponentOverride::SpriteGeometry {
                            width: ow,
                            height: oh,
                            layer: ol,
                        },
                    ) => {
                        if let Some(value) = ow {
                            *width = *value;
                        }
                        if let Some(value) = oh {
                            *height = *value;
                        }
                        if let Some(value) = ol {
                            *layer = *value;
                        }
                    }
                    (
                        Component::Sprite {
                            material,
                            animation_clip,
                            ..
                        },
                        ComponentOverride::SpriteResources {
                            material: om,
                            animation_clip: oc,
                        },
                    ) => {
                        if let Some(value) = om {
                            *material = value.clone();
                        }
                        if let Some(value) = oc {
                            *animation_clip = value.clone();
                        }
                    }
                    (
                        Component::Sprite { color, texture, .. },
                        ComponentOverride::Sprite {
                            color: color_override,
                            texture: texture_override,
                        },
                    ) => {
                        if let Some(val) = color_override {
                            *color = *val;
                        }
                        if let Some(val) = texture_override {
                            *texture = val.clone();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Create an entity instance from this prefab
    pub fn instantiate(&self, overrides: Option<PrefabOverride>) -> Entity {
        let mut entity = self.root_entity.clone();
        entity.id = next_id().to_string();

        if let Some(overrides) = overrides {
            self.apply_overrides(&mut entity, &overrides);
        }

        entity
    }
}

impl PrefabOverride {
    pub fn new(entity_id: String) -> Self {
        Self {
            entity_id,
            overrides: HashMap::new(),
        }
    }

    pub fn add_override(&mut self, component_type: String, override_value: ComponentOverride) {
        self.overrides.insert(component_type, override_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_creation() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![
                Component::Transform {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Sprite {
                    color: [1.0, 0.0, 0.0, 1.0],
                    width: 32.0,
                    height: 32.0,
                    texture: None,
                    layer: 0.0,
                    material: None,
                    animation_clip: None,
                },
            ],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);
        assert_eq!(prefab.name, "TestPrefab");
        assert_eq!(prefab.root_entity.name, "TestEntity");
    }

    #[test]
    fn prefab_instantiate_without_overrides() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![Component::Transform {
                x: 10.0,
                y: 20.0,
                rotation: 0.0,
                scale: 1.0,
            }],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);
        let instance = prefab.instantiate(None);

        assert_ne!(instance.id, "entity1"); // New ID
        assert_eq!(instance.name, "TestEntity");
        assert_eq!(instance.components.len(), 1);
    }

    #[test]
    fn prefab_instantiate_with_overrides() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        let mut overrides = PrefabOverride::new("entity1".to_string());
        overrides.add_override(
            "transform".to_string(),
            ComponentOverride::Transform {
                x: Some(50.0),
                y: Some(100.0),
                rotation: None,
                scale: None,
            },
        );

        let instance = prefab.instantiate(Some(overrides));

        if let Component::Transform { x, y, .. } = instance.components[0] {
            assert_eq!(x, 50.0);
            assert_eq!(y, 100.0);
        } else {
            panic!("Expected Transform component");
        }
    }

    #[test]
    fn prefab_override_sprite_color() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![Component::Sprite {
                layer: 0.0,
                width: 32.0,
                height: 32.0,
                material: None,
                animation_clip: None,
                color: [1.0, 0.0, 0.0, 1.0],
                texture: None,
            }],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        let mut overrides = PrefabOverride::new("entity1".to_string());
        overrides.add_override(
            "sprite".to_string(),
            ComponentOverride::Sprite {
                color: Some([0.0, 1.0, 0.0, 1.0]),
                texture: None,
            },
        );

        let instance = prefab.instantiate(Some(overrides));

        if let Component::Sprite { color, .. } = instance.components[0] {
            assert_eq!(color, [0.0, 1.0, 0.0, 1.0]);
        } else {
            panic!("Expected Sprite component");
        }
    }

    #[test]
    fn sprite_override_can_clear_an_inherited_texture() {
        let entity = Entity {
            id: "template".into(),
            name: "Textured".into(),
            components: vec![Component::Sprite {
                color: [1.0; 4],
                width: 32.0,
                height: 32.0,
                texture: Some("art/texture.png".into()),
                layer: 0.0,
                material: None,
                animation_clip: None,
            }],
        };
        let prefab = Prefab::new("Textured".into(), entity);
        let mut overrides = PrefabOverride::new("instance".into());
        overrides.add_override(
            "sprite".into(),
            ComponentOverride::Sprite {
                color: None,
                texture: Some(None),
            },
        );
        let instance = prefab.instantiate(Some(overrides));
        assert!(matches!(
            instance.components.first(),
            Some(Component::Sprite { texture: None, .. })
        ));
    }

    #[test]
    fn legacy_texture_override_string_still_deserializes() {
        let legacy = r#"{"type":"Sprite","color":null,"texture":"art/old.png"}"#;
        let override_value: ComponentOverride = serde_json::from_str(legacy).unwrap();
        assert!(
            matches!(override_value, ComponentOverride::Sprite { texture: Some(Some(path)), .. } if path == "art/old.png")
        );
    }

    #[test]
    fn prefab_serialization() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        // Serialize to RON
        let ron_str = ron::to_string(&prefab).expect("serialize to RON");
        let loaded: Prefab = ron::from_str(&ron_str).expect("deserialize from RON");

        assert_eq!(loaded.name, "TestPrefab");
        assert_eq!(loaded.root_entity.name, "TestEntity");
    }

    #[test]
    fn prefab_ron_merge_friendly() {
        // Test that RON format is merge-friendly by simulating a git merge scenario
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![
                Component::Transform {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Sprite {
                    color: [1.0, 0.0, 0.0, 1.0],
                    width: 32.0,
                    height: 32.0,
                    texture: None,
                    layer: 0.0,
                    material: None,
                    animation_clip: None,
                },
            ],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        // Serialize to RON
        let ron_str = ron::to_string(&prefab).expect("serialize to RON");

        // Verify RON is human-readable and structured
        assert!(ron_str.contains("name"));
        assert!(ron_str.contains("id"));
        assert!(ron_str.contains("root_entity"));

        // Verify we can round-trip
        let loaded: Prefab = ron::from_str(&ron_str).expect("deserialize from RON");
        assert_eq!(loaded.name, prefab.name);
        assert_eq!(loaded.root_entity.name, prefab.root_entity.name);
    }
}

#[cfg(test)]
mod texture_persistence_tests {
    use super::*;

    #[test]
    fn texture_override_preserves_inherit_set_and_clear_in_project_and_recovery_formats() {
        for texture in [Some(None), Some(Some("art/hero.png".into())), None] {
            let value = ComponentOverride::Sprite {
                color: None,
                texture: texture.clone(),
            };
            for format in ["json", "toml"] {
                let decoded = if format == "json" {
                    serde_json::from_str::<ComponentOverride>(
                        &serde_json::to_string(&value).unwrap(),
                    )
                    .unwrap()
                } else {
                    toml::from_str::<ComponentOverride>(&toml::to_string(&value).unwrap()).unwrap()
                };
                let ComponentOverride::Sprite {
                    texture: actual, ..
                } = decoded
                else {
                    unreachable!()
                };
                assert_eq!(
                    actual, texture,
                    "Texture inherit, set and clear must remain distinct after persistence"
                );
            }
        }
    }
    #[test]
    fn legacy_null_inherits_and_invalid_texture_override_types_are_rejected() {
        let legacy: ComponentOverride =
            serde_json::from_str(r#"{"type":"Sprite","color":null,"texture":null}"#).unwrap();
        assert!(matches!(
            legacy,
            ComponentOverride::Sprite { texture: None, .. }
        ));
        for value in ["true", "42", "[]", "{}"] {
            let source = format!(r#"{{"type":"Sprite","texture":{value}}}"#);
            assert!(serde_json::from_str::<ComponentOverride>(&source).is_err());
        }
        assert!(toml::from_str::<ComponentOverride>("type = 'Sprite'\ntexture = true").is_err());
    }
    #[test]
    fn sprite_resources_preserve_all_three_states_and_reject_invalid_values() {
        for value in [None, Some(None), Some(Some("named-resource".into()))] {
            let original = ComponentOverride::SpriteResources {
                material: value.clone(),
                animation_clip: value.clone(),
            };
            for decoded in [
                serde_json::from_str::<ComponentOverride>(
                    &serde_json::to_string(&original).unwrap(),
                )
                .unwrap(),
                toml::from_str::<ComponentOverride>(&toml::to_string(&original).unwrap()).unwrap(),
            ] {
                let ComponentOverride::SpriteResources {
                    material,
                    animation_clip,
                } = decoded
                else {
                    unreachable!()
                };
                assert_eq!(material, value);
                assert_eq!(animation_clip, value);
            }
        }
        for field in ["material", "animation_clip"] {
            for value in ["true", "42", "[]", "{}"] {
                let source = format!(r#"{{"type":"SpriteResources","{field}":{value}}}"#);
                assert!(serde_json::from_str::<ComponentOverride>(&source).is_err());
            }
        }
    }
}
