//! Prefab system for reusable entity templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::project::{Entity, Component};

/// A prefab is a reusable template for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    pub id: String,
    pub name: String,
    pub root_entity: Entity,
}

/// Override values for a specific prefab instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabOverride {
    pub entity_id: String,
    pub overrides: HashMap<String, ComponentOverride>,
}

/// Override for a specific component property
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComponentOverride {
    Transform { x: Option<f32>, y: Option<f32>, rotation: Option<f32>, scale: Option<f32> },
    Sprite { color: Option<[f32; 4]> },
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
            id: uuid::Uuid::new_v4().to_string(),
            name,
            root_entity,
        }
    }

    /// Apply overrides to an entity from this prefab
    pub fn apply_overrides(&self, entity: &mut Entity, overrides: &PrefabOverride) {
        for (_component_type, override_value) in &overrides.overrides {
            for component in &mut entity.components {
                match (component, override_value) {
                    (Component::Transform { x, y, rotation, scale }, ComponentOverride::Transform { x: ox, y: oy, rotation: orot, scale: oscale }) => {
                        if let Some(val) = ox { *x = *val; }
                        if let Some(val) = oy { *y = *val; }
                        if let Some(val) = orot { *rotation = *val; }
                        if let Some(val) = oscale { *scale = *val; }
                    }
                    (Component::Sprite { color }, ComponentOverride::Sprite { color: ocolor }) => {
                        if let Some(val) = ocolor { *color = *val; }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Create an entity instance from this prefab
    pub fn instantiate(&self, overrides: Option<PrefabOverride>) -> Entity {
        let mut entity = self.root_entity.clone();
        entity.id = uuid::Uuid::new_v4().to_string(); // New instance ID

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
                Component::Transform { x: 0.0, y: 0.0, rotation: 0.0, scale: 1.0 },
                Component::Sprite { color: [1.0, 0.0, 0.0, 1.0] },
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
            components: vec![
                Component::Transform { x: 10.0, y: 20.0, rotation: 0.0, scale: 1.0 },
            ],
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
            components: vec![
                Component::Transform { x: 0.0, y: 0.0, rotation: 0.0, scale: 1.0 },
            ],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        let mut overrides = PrefabOverride::new("entity1".to_string());
        overrides.add_override(
            "transform".to_string(),
            ComponentOverride::Transform { x: Some(50.0), y: Some(100.0), rotation: None, scale: None },
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
            components: vec![
                Component::Sprite { color: [1.0, 0.0, 0.0, 1.0] },
            ],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        let mut overrides = PrefabOverride::new("entity1".to_string());
        overrides.add_override(
            "sprite".to_string(),
            ComponentOverride::Sprite { color: Some([0.0, 1.0, 0.0, 1.0]) },
        );

        let instance = prefab.instantiate(Some(overrides));

        if let Component::Sprite { color } = instance.components[0] {
            assert_eq!(color, [0.0, 1.0, 0.0, 1.0]);
        } else {
            panic!("Expected Sprite component");
        }
    }

    #[test]
    fn prefab_serialization() {
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![
                Component::Transform { x: 0.0, y: 0.0, rotation: 0.0, scale: 1.0 },
            ],
        };

        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        // Serialize to RON
        let ron_str = ron::to_string(&prefab).expect("serialize to RON");
        let loaded: Prefab = ron::from_str(&ron_str).expect("deserialize from RON");

        assert_eq!(loaded.name, "TestPrefab");
        assert_eq!(loaded.root_entity.name, "TestEntity");
    }
}
