//! Gamepad manager with hot-plug detection.

use crate::controller::ControllerType;
use gilrs::{EventType, GamepadId, Gilrs};
use std::collections::HashMap;

/// Gamepad device info.
#[derive(Debug, Clone)]
pub struct GamepadInfo {
    pub id: GamepadId,
    pub name: String,
    pub controller_type: ControllerType,
}

/// Gamepad manager for handling input and hot-plug events.
pub struct GamepadManager {
    gilrs: Gilrs,
    connected_gamepads: HashMap<GamepadId, GamepadInfo>,
}

impl GamepadManager {
    /// Create a new gamepad manager.
    pub fn new() -> Result<Self, Box<gilrs::Error>> {
        let gilrs = Gilrs::new().map_err(Box::new)?;
        let mut connected_gamepads = HashMap::new();

        // Initialize with already connected gamepads
        for (id, gamepad) in gilrs.gamepads() {
            let info = GamepadInfo {
                id,
                name: gamepad.name().to_string(),
                controller_type: detect_controller_type(gamepad.name()),
            };
            connected_gamepads.insert(id, info);
        }

        Ok(Self {
            gilrs,
            connected_gamepads,
        })
    }

    /// Process gamepad events and return hot-plug events.
    pub fn update(&mut self) -> Vec<GamepadEvent> {
        let mut events = Vec::new();

        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    let gamepad = self.gilrs.gamepad(event.id);
                    let info = GamepadInfo {
                        id: event.id,
                        name: gamepad.name().to_string(),
                        controller_type: detect_controller_type(gamepad.name()),
                    };
                    self.connected_gamepads.insert(event.id, info.clone());
                    events.push(GamepadEvent::Connected(info));
                }
                EventType::Disconnected => {
                    if let Some(info) = self.connected_gamepads.remove(&event.id) {
                        events.push(GamepadEvent::Disconnected(info));
                    }
                }
                _ => {}
            }
        }

        events
    }

    /// Current held tokens. Rebuild each poll so disconnect/release never latches input.
    /// Buttons use the existing runtime indices; semantic UI tokens avoid OS scan codes.
    pub fn pressed_tokens(&self) -> Vec<String> {
        use gilrs::{Axis, Button};
        let mut out = Vec::new();
        let buttons = [
            Button::South,
            Button::East,
            Button::North,
            Button::West,
            Button::LeftTrigger2,
            Button::RightTrigger2,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::LeftThumb,
            Button::RightThumb,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
            Button::Start,
            Button::Select,
            Button::Mode,
        ];
        for (_, pad) in self.gilrs.gamepads().filter(|(_, pad)| pad.is_connected()) {
            for (index, button) in buttons.into_iter().enumerate() {
                if pad.is_pressed(button) {
                    out.push(format!("GamepadBtn{index}"));
                }
            }
            for (button, key) in [
                (Button::DPadUp, "GamepadUp"),
                (Button::DPadDown, "GamepadDown"),
                (Button::DPadLeft, "GamepadLeft"),
                (Button::DPadRight, "GamepadRight"),
                (Button::South, "GamepadAccept"),
                (Button::East, "GamepadCancel"),
                (Button::Start, "GamepadPause"),
            ] {
                if pad.is_pressed(button) {
                    out.push(key.into());
                }
            }
            for (index, axis) in [
                Axis::LeftStickX,
                Axis::LeftStickY,
                Axis::RightStickX,
                Axis::RightStickY,
                Axis::LeftZ,
                Axis::RightZ,
            ]
            .into_iter()
            .enumerate()
            {
                let value = pad.value(axis);
                if value.abs() > 0.1 {
                    out.push(format!("GamepadAxis{index}:{value:.2}"));
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Get all connected gamepads.
    pub fn connected_gamepads(&self) -> Vec<GamepadInfo> {
        self.connected_gamepads.values().cloned().collect()
    }

    /// Check if a gamepad is connected.
    pub fn is_connected(&self, id: GamepadId) -> bool {
        self.connected_gamepads.contains_key(&id)
    }

    /// Get the gilrs instance for direct gamepad state access.
    pub fn gilrs(&mut self) -> &mut Gilrs {
        &mut self.gilrs
    }
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize gamepad manager")
    }
}

/// Gamepad hot-plug event.
#[derive(Debug, Clone)]
pub enum GamepadEvent {
    Connected(GamepadInfo),
    Disconnected(GamepadInfo),
}

/// Detect controller type from gamepad name.
fn detect_controller_type(name: &str) -> ControllerType {
    let name_lower = name.to_lowercase();

    if name_lower.contains("xbox") || name_lower.contains("microsoft") {
        ControllerType::Xbox
    } else if name_lower.contains("playstation")
        || name_lower.contains("dualshock")
        || name_lower.contains("dualsense")
    {
        ControllerType::PlayStation
    } else if name_lower.contains("nintendo")
        || name_lower.contains("switch")
        || name_lower.contains("pro controller")
    {
        ControllerType::Nintendo
    } else {
        ControllerType::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_type_detection() {
        assert_eq!(
            detect_controller_type("Xbox Controller"),
            ControllerType::Xbox
        );
        assert_eq!(
            detect_controller_type("DualSense"),
            ControllerType::PlayStation
        );
        assert_eq!(
            detect_controller_type("Pro Controller"),
            ControllerType::Nintendo
        );
        assert_eq!(
            detect_controller_type("Generic Gamepad"),
            ControllerType::Generic
        );
    }
}
