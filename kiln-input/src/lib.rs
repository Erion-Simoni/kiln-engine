//! Kiln input mapping and action system.

pub mod action;
pub mod binding;
pub mod controller;
#[cfg(feature = "gamepad")]
pub mod gamepad;

pub use action::{Action, ActionMap};
pub use binding::{Binding, BindingType};
pub use controller::{ControllerGlyph, ControllerType};

#[cfg(feature = "gamepad")]
pub use gamepad::{GamepadEvent, GamepadInfo, GamepadManager};
