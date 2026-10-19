//! Input binding types for keyboard and gamepad.

use serde::{Deserialize, Serialize};
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
    pub fn from_key_string(key: &str) -> Self {
        Self::Key(key.to_string())
    }

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
            BindingType::Key(key) => write!(f, "{:?}", key),
            BindingType::GamepadButton(btn) => write!(f, "GamepadBtn({})", btn),
            BindingType::GamepadAxis { axis, threshold } => {
                write!(f, "GamepadAxis({}, {})", axis, threshold)
            }
        }
    }
}

/// Input binding with binding type and modifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub binding_type: BindingType,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Binding {
    pub fn new(binding_type: BindingType) -> Self {
        Self {
            binding_type,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_new() {
        let binding = Binding::new(BindingType::Key("KeyA".to_string()));
        assert!(!binding.shift);
        assert!(!binding.ctrl);
        assert!(!binding.alt);
    }

    #[test]
    fn test_binding_with_modifiers() {
        let binding = Binding::new(BindingType::Key("KeyS".to_string()))
            .with_shift()
            .with_ctrl();
        assert!(binding.shift);
        assert!(binding.ctrl);
        assert!(!binding.alt);
    }

    #[test]
    fn test_binding_type_display() {
        let key_binding = BindingType::Key("Space".to_string());
        assert!(key_binding.to_string().contains("Space"));

        let btn_binding = BindingType::GamepadButton(0);
        assert!(btn_binding.to_string().contains("GamepadBtn"));

        let axis_binding = BindingType::GamepadAxis { axis: 0, threshold: 0.5 };
        assert!(axis_binding.to_string().contains("GamepadAxis"));
    }

    #[test]
    fn test_key_string_conversion() {
        let binding_type = BindingType::from_key_string("Space");
        assert_eq!(binding_type, BindingType::Key("Space".to_string()));

        let converted = binding_type.as_key_string();
        assert_eq!(converted, Some("Space"));
    }
}
