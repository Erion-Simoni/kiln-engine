//! Transform component for 2D entities

use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Transform {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
        }
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}
