//! Controller glyph system for gamepad button display.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Controller type for glyph selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControllerType {
    Xbox,
    PlayStation,
    Nintendo,
    Generic,
}

impl fmt::Display for ControllerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControllerType::Xbox => write!(f, "Xbox"),
            ControllerType::PlayStation => write!(f, "PlayStation"),
            ControllerType::Nintendo => write!(f, "Nintendo"),
            ControllerType::Generic => write!(f, "Generic"),
        }
    }
}

/// Gamepad button glyph mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerGlyph {
    /// A button (Xbox A, PlayStation Cross, Nintendo B)
    A,
    /// B button (Xbox B, PlayStation Circle, Nintendo A)
    B,
    /// X button (Xbox X, PlayStation Square, Nintendo Y)
    X,
    /// Y button (Xbox Y, PlayStation Triangle, Nintendo X)
    Y,
    /// Left bumper
    LeftBumper,
    /// Right bumper
    RightBumper,
    /// Left trigger
    LeftTrigger,
    /// Right trigger
    RightTrigger,
    /// Left stick click
    LeftStick,
    /// Right stick click
    RightStick,
    /// D-pad up
    DpadUp,
    /// D-pad down
    DpadDown,
    /// D-pad left
    DpadLeft,
    /// D-pad right
    DpadRight,
    /// Start button
    Start,
    /// Select/Back button
    Select,
    /// Home/Guide button
    Home,
}

impl ControllerGlyph {
    /// Get the unicode character for the glyph based on controller type.
    pub fn as_char(self, controller_type: ControllerType) -> char {
        match (self, controller_type) {
            // Xbox glyphs (using unicode as placeholders)
            (ControllerGlyph::A, ControllerType::Xbox) => '🅰',
            (ControllerGlyph::B, ControllerType::Xbox) => '🅱',
            (ControllerGlyph::X, ControllerType::Xbox) => '✕',
            (ControllerGlyph::Y, ControllerType::Xbox) => '🆎',
            // PlayStation glyphs
            (ControllerGlyph::A, ControllerType::PlayStation) => '✕',
            (ControllerGlyph::B, ControllerType::PlayStation) => '○',
            (ControllerGlyph::X, ControllerType::PlayStation) => '□',
            (ControllerGlyph::Y, ControllerType::PlayStation) => '△',
            // Nintendo glyphs
            (ControllerGlyph::A, ControllerType::Nintendo) => '🅰',
            (ControllerGlyph::B, ControllerType::Nintendo) => '🅱',
            (ControllerGlyph::X, ControllerType::Nintendo) => '✕',
            (ControllerGlyph::Y, ControllerType::Nintendo) => '🆎',
            // Fallback for all types
            (ControllerGlyph::LeftBumper, _) => '◀',
            (ControllerGlyph::RightBumper, _) => '▶',
            (ControllerGlyph::LeftTrigger, _) => '⏷',
            (ControllerGlyph::RightTrigger, _) => '⏷',
            (ControllerGlyph::LeftStick, _) => '◉',
            (ControllerGlyph::RightStick, _) => '◉',
            (ControllerGlyph::DpadUp, _) => '▲',
            (ControllerGlyph::DpadDown, _) => '▼',
            (ControllerGlyph::DpadLeft, _) => '◀',
            (ControllerGlyph::DpadRight, _) => '▶',
            (ControllerGlyph::Start, _) => '▶',
            (ControllerGlyph::Select, _) => '◀',
            (ControllerGlyph::Home, _) => '⌂',
            // Fallback for unmapped combinations
            (_, _) => '?',
        }
    }

    /// Get the label text for the glyph.
    pub fn as_label(self) -> &'static str {
        match self {
            ControllerGlyph::A => "A",
            ControllerGlyph::B => "B",
            ControllerGlyph::X => "X",
            ControllerGlyph::Y => "Y",
            ControllerGlyph::LeftBumper => "LB",
            ControllerGlyph::RightBumper => "RB",
            ControllerGlyph::LeftTrigger => "LT",
            ControllerGlyph::RightTrigger => "RT",
            ControllerGlyph::LeftStick => "L3",
            ControllerGlyph::RightStick => "R3",
            ControllerGlyph::DpadUp => "Up",
            ControllerGlyph::DpadDown => "Down",
            ControllerGlyph::DpadLeft => "Left",
            ControllerGlyph::DpadRight => "Right",
            ControllerGlyph::Start => "Start",
            ControllerGlyph::Select => "Select",
            ControllerGlyph::Home => "Home",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_type_display() {
        assert_eq!(ControllerType::Xbox.to_string(), "Xbox");
        assert_eq!(ControllerType::PlayStation.to_string(), "PlayStation");
    }

    #[test]
    fn test_glyph_as_char() {
        let a_glyph = ControllerGlyph::A.as_char(ControllerType::Xbox);
        assert_ne!(a_glyph, '?');

        let a_glyph_ps = ControllerGlyph::A.as_char(ControllerType::PlayStation);
        assert_ne!(a_glyph_ps, '?');
    }

    #[test]
    fn test_glyph_as_label() {
        assert_eq!(ControllerGlyph::A.as_label(), "A");
        assert_eq!(ControllerGlyph::LeftBumper.as_label(), "LB");
        assert_eq!(ControllerGlyph::Start.as_label(), "Start");
    }
}
