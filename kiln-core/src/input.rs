//! Input action map stored on a project.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Input binding type (keyboard or gamepad).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingType {
    /// Keyboard key binding (serialized as string).
    Key(String),
    /// Gamepad button binding (SDL button code).
    GamepadButton(u8),
    /// Gamepad axis binding (SDL axis code with threshold).
    GamepadAxis { axis: u8, threshold: f32 },
}

impl BindingType {
    /// Build a keyboard binding from a key name.
    pub fn from_key_string(key: &str) -> Self {
        Self::Key(key.to_string())
    }

    /// The keyboard key name, if this is a key binding.
    pub fn as_key_string(&self) -> Option<&str> {
        match self {
            BindingType::Key(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for BindingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindingType::Key(key) => write!(f, "{key}"),
            BindingType::GamepadButton(btn) => write!(f, "GamepadBtn({btn})"),
            BindingType::GamepadAxis { axis, threshold } => {
                write!(f, "GamepadAxis({axis}, {threshold})")
            }
        }
    }
}

/// Input binding with binding type and modifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub binding_type: BindingType,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
}

impl Binding {
    /// A binding with no modifiers.
    pub fn new(binding_type: BindingType) -> Self {
        Self {
            binding_type,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    /// Enable the shift modifier.
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Enable the ctrl modifier.
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Enable the alt modifier.
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

/// Action identifier (e.g., "jump", "fire", "interact").
pub type ActionId = String;

/// Action that can be triggered by input bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: ActionId,
    pub name: String,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

impl Action {
    /// A new action with no bindings.
    pub fn new(id: impl Into<ActionId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            bindings: Vec::new(),
        }
    }

    /// Append a binding and return self.
    pub fn with_binding(mut self, binding: Binding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Append a binding.
    pub fn add_binding(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    /// Remove a matching binding.
    pub fn remove_binding(&mut self, binding: &Binding) {
        self.bindings.retain(|b| b != binding);
    }
}

/// Action map containing all actions for a game or context.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionMap {
    #[serde(default)]
    pub actions: HashMap<ActionId, Action>,
}

impl ActionMap {
    /// An empty action map.
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    /// Insert or replace an action.
    pub fn add_action(&mut self, action: Action) {
        self.actions.insert(action.id.clone(), action);
    }

    /// Look up an action by id.
    pub fn get_action(&self, id: &str) -> Option<&Action> {
        self.actions.get(id)
    }

    /// Look up an action by id, mutably.
    pub fn get_action_mut(&mut self, id: &str) -> Option<&mut Action> {
        self.actions.get_mut(id)
    }

    /// Remove an action by id.
    pub fn remove_action(&mut self, id: &str) -> Option<Action> {
        self.actions.remove(id)
    }

    /// Iterate all actions.
    pub fn actions(&self) -> impl Iterator<Item = &Action> {
        self.actions.values()
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
