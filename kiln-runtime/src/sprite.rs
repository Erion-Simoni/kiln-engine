//! Sprite component for 2D rendering

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub color: [f32; 4],
    pub size: [f32; 2],
}

impl Sprite {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            size: [32.0, 32.0],
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}
