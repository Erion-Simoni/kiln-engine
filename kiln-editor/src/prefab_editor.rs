//! Prefab editor panel for creating and managing prefabs

use kiln_core::{Entity, Prefab, PrefabOverride};
use std::collections::HashMap;

/// Prefab editor state
#[derive(Debug, Clone, Default)]
pub struct PrefabEditorState {
    prefabs: HashMap<String, Prefab>,
    selected_prefab: Option<String>,
}

impl PrefabEditorState {
    pub fn new() -> Self {
        Self {
            prefabs: HashMap::new(),
            selected_prefab: None,
        }
    }

    pub fn add_prefab(&mut self, prefab: Prefab) {
        self.prefabs.insert(prefab.id.clone(), prefab);
    }

    pub fn get_prefab(&self, id: &str) -> Option<&Prefab> {
        self.prefabs.get(id)
    }

    pub fn get_prefab_mut(&mut self, id: &str) -> Option<&mut Prefab> {
        self.prefabs.get_mut(id)
    }

    pub fn remove_prefab(&mut self, id: &str) {
        self.prefabs.remove(id);
        if self.selected_prefab.as_ref() == Some(&id.to_string()) {
            self.selected_prefab = None;
        }
    }

    pub fn instantiate_prefab(
        &self,
        prefab_id: &str,
        overrides: Option<PrefabOverride>,
    ) -> Option<Entity> {
        let prefab = self.prefabs.get(prefab_id)?;
        Some(prefab.instantiate(overrides))
    }

    pub fn prefab_list(&self) -> Vec<&Prefab> {
        self.prefabs.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_editor_add_and_get() {
        let mut editor = PrefabEditorState::new();
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![],
        };
        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        editor.add_prefab(prefab.clone());
        assert_eq!(editor.prefab_list().len(), 1);
        assert!(editor.get_prefab(&prefab.id).is_some());
    }

    #[test]
    fn prefab_editor_remove() {
        let mut editor = PrefabEditorState::new();
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![],
        };
        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        editor.add_prefab(prefab.clone());
        editor.remove_prefab(&prefab.id);
        assert_eq!(editor.prefab_list().len(), 0);
    }

    #[test]
    fn prefab_editor_instantiate() {
        let mut editor = PrefabEditorState::new();
        let entity = Entity {
            id: "entity1".to_string(),
            name: "TestEntity".to_string(),
            components: vec![],
        };
        let prefab = Prefab::new("TestPrefab".to_string(), entity);

        editor.add_prefab(prefab.clone());
        let instance = editor.instantiate_prefab(&prefab.id, None);

        assert!(instance.is_some());
        assert_ne!(instance.unwrap().id, "entity1");
    }
}
