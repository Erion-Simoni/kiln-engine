//! Input handling

use winit::event::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    KeyCode(winit::keyboard::KeyCode),
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub key: Option<Key>,
    pub pressed: bool,
    pub mouse_pos: Option<(f32, f32)>,
    pub mouse_button: Option<MouseButton>,
    pub mouse_pressed: bool,
    pub scroll: Option<f32>,
}

#[derive(Debug, Default)]
pub struct InputState {
    pub keys: Vec<Key>,
    pub mouse_pos: (f32, f32),
    pub mouse_buttons: Vec<MouseButton>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_key_pressed(&self, key: winit::keyboard::KeyCode) -> bool {
        self.keys
            .iter()
            .any(|k| matches!(k, Key::KeyCode(kc) if kc == &key))
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.contains(&button)
    }
}
