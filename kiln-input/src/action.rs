//! Action map system for rebindable input mappings.

pub use kiln_core::{Action, ActionMap};

use serde_json;
use thiserror::Error;

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
}
