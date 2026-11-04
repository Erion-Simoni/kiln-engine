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

    /// Replace the displayed definitions with those stored in the project.
    pub fn replace_prefabs(&mut self, prefabs: HashMap<String, Prefab>) {
        self.prefabs = prefabs;
        if self
            .selected_prefab
            .as_ref()
            .is_some_and(|id| !self.prefabs.contains_key(id))
        {
            self.selected_prefab = None;
        }
    }

    /// Clone the definitions for project serialization.
    pub fn prefabs(&self) -> &HashMap<String, Prefab> {
        &self.prefabs
    }

    pub fn get_prefab(&self, id: &str) -> Option<&Prefab> {
        self.prefabs.get(id)
    }

    pub fn get_prefab_mut(&mut self, id: &str) -> Option<&mut Prefab> {
        self.prefabs.get_mut(id)
    }

    /// The prefab that is selected, if there is one.
    pub fn selected_prefab(&self) -> Option<&Prefab> {
        self.selected_prefab
            .as_ref()
            .and_then(|id| self.prefabs.get(id))
    }

    /// The id of the selected prefab, if there is one.
    pub fn selected_prefab_id(&self) -> Option<&str> {
        self.selected_prefab.as_deref()
    }

    /// Select a prefab, so it can be edited or instantiated.
    pub fn select_prefab(&mut self, id: &str) {
        if self.prefabs.contains_key(id) {
            self.selected_prefab = Some(id.to_string());
        }
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
    fn replacing_project_prefabs_preserves_selection_only_when_it_still_exists() {
        let mut editor = PrefabEditorState::new();
        let entity = Entity {
            id: "enemy".to_string(),
            name: "Enemy".to_string(),
            components: vec![],
        };
        let prefab = Prefab::new("Enemy".to_string(), entity);
        let id = prefab.id.clone();
        editor.add_prefab(prefab.clone());
        editor.select_prefab(&id);
        editor.replace_prefabs(HashMap::from([(id.clone(), prefab.clone())]));
        assert_eq!(editor.selected_prefab_id(), Some(id.as_str()));
        editor.replace_prefabs(HashMap::new());
        assert_eq!(editor.selected_prefab_id(), None);
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
