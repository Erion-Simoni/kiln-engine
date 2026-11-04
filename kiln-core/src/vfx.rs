//! Persisted particle emitter configuration.
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Emission shape for particle spawning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum EmissionShape {
    /// Single point.
    #[default]
    Point,
    /// Circle with radius.
    Circle { radius: f32 },
    /// Rectangle with width and height.
    Rectangle { width: f32, height: f32 },
}

/// Particle emitter configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitterConfig {
    /// Emission shape.
    pub shape: EmissionShape,
    /// Particles per second.
    pub rate: f32,
    /// Initial speed range (min, max).
    pub speed_range: [f32; 2],
    /// Initial angle range in radians (min, max).
    pub angle_range: [f32; 2],
    /// Initial lifetime range in seconds (min, max).
    pub lifetime_range: [f32; 2],
    /// Initial size range (min, max).
    pub size_range: [f32; 2],
    /// Initial color (R, G, B, A).
    pub color: [f32; 4],
    /// Gravity acceleration (x, y).
    pub gravity: [f32; 2],
    /// Whether particles should fade out over lifetime.
    pub fade_out: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            shape: EmissionShape::Point,
            rate: 10.0,
            speed_range: [10.0, 20.0],
            angle_range: [0.0, 2.0 * PI],
            lifetime_range: [1.0, 2.0],
            size_range: [0.5, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            gravity: [0.0, -9.8],
            fade_out: true,
        }
    }
}

impl EmitterConfig {
    /// Reject malformed saved/edited configurations before simulation.
    pub fn validate(&self) -> Result<(), String> {
        if !self.rate.is_finite()
            || !(0.0..=100_000.0).contains(&self.rate)
            || !self
                .color
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            || !self.gravity.iter().all(|v| v.is_finite())
        {
            return Err("Particle rate, color and gravity must be valid".into());
        }
        for range in [
            self.speed_range,
            self.angle_range,
            self.lifetime_range,
            self.size_range,
        ] {
            if !range.iter().all(|v| v.is_finite()) || range[0] > range[1] {
                return Err("Particle ranges must be finite and ordered".into());
            }
        }
        if self.lifetime_range[0] <= 0.0 || self.size_range[0] <= 0.0 {
            return Err("Particle lifetime and size must be positive".into());
        }
        let valid = match self.shape {
            EmissionShape::Point => true,
            EmissionShape::Circle { radius } => radius.is_finite() && radius >= 0.0,
            EmissionShape::Rectangle { width, height } => {
                width.is_finite() && height.is_finite() && width >= 0.0 && height >= 0.0
            }
        };
        if !valid {
            return Err("Emission shape dimensions must be finite and nonnegative".into());
        }
        Ok(())
    }
}
