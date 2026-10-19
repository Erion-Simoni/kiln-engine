//! Action map system for rebindable input mappings.

use crate::binding::Binding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Action identifier (e.g., "jump", "fire", "interact").
pub type ActionId = String;

/// Action that can be triggered by input bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: ActionId,
    pub name: String,
    pub bindings: Vec<Binding>,
}

impl Action {
    pub fn new(id: impl Into<ActionId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            bindings: Vec::new(),
        }
    }

    pub fn with_binding(mut self, binding: Binding) -> Self {
        self.bindings.push(binding);
        self
    }

    pub fn add_binding(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    pub fn remove_binding(&mut self, binding: &Binding) {
        self.bindings.retain(|b| b != binding);
    }
}

/// Action map containing all actions for a game or context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMap {
    pub actions: HashMap<ActionId, Action>,
}

impl ActionMap {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    pub fn add_action(&mut self, action: Action) {
        self.actions.insert(action.id.clone(), action);
    }

    pub fn get_action(&self, id: &str) -> Option<&Action> {
        self.actions.get(id)
    }

    pub fn get_action_mut(&mut self, id: &str) -> Option<&mut Action> {
        self.actions.get_mut(id)
    }

    pub fn remove_action(&mut self, id: &str) -> Option<Action> {
        self.actions.remove(id)
    }

    pub fn actions(&self) -> impl Iterator<Item = &Action> {
        self.actions.values()
    }

    /// Serialize action map to JSON.
    pub fn to_json(&self) -> Result<String, ActionMapError> {
        serde_json::to_string_pretty(self).map_err(ActionMapError::Serialization)
    }

    /// Deserialize action map from JSON.
    pub fn from_json(json: &str) -> Result<Self, ActionMapError> {
        serde_json::from_str(json).map_err(ActionMapError::Serialization)
    }
}

impl Default for ActionMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Action map error types.
#[derive(Error, Debug)]
pub enum ActionMapError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Action not found: {0}")]
    ActionNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::{Binding, BindingType};

    #[test]
    fn test_action_new() {
        let action = Action::new("jump", "Jump");
        assert_eq!(action.id, "jump");
        assert_eq!(action.name, "Jump");
        assert!(action.bindings.is_empty());
    }

    #[test]
    fn test_action_with_binding() {
        let binding = Binding::new(BindingType::Key("Space".to_string()));
        let action = Action::new("jump", "Jump").with_binding(binding);
        assert_eq!(action.bindings.len(), 1);
    }

    #[test]
    fn test_action_add_remove_binding() {
        let binding = Binding::new(BindingType::Key("Space".to_string()));
        let mut action = Action::new("jump", "Jump");
        action.add_binding(binding.clone());
        assert_eq!(action.bindings.len(), 1);
        action.remove_binding(&binding);
        assert!(action.bindings.is_empty());
    }

    #[test]
    fn test_action_map_add_get() {
        let mut map = ActionMap::new();
        let action = Action::new("jump", "Jump");
        map.add_action(action);

        let retrieved = map.get_action("jump");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Jump");
    }

    #[test]
    fn test_action_map_remove() {
        let mut map = ActionMap::new();
        let action = Action::new("jump", "Jump");
        map.add_action(action);

        let removed = map.remove_action("jump");
        assert!(removed.is_some());
        assert!(map.get_action("jump").is_none());
    }

    #[test]
    fn test_action_map_serialization() {
        let mut map = ActionMap::new();
        let binding = Binding::new(BindingType::Key("Space".to_string()));
        let action = Action::new("jump", "Jump").with_binding(binding);
        map.add_action(action);

        let json = map.to_json().unwrap();
        assert!(json.contains("jump"));
        assert!(json.contains("Jump"));

        let deserialized = ActionMap::from_json(&json).unwrap();
        assert_eq!(deserialized.actions.len(), 1);
    }
}
