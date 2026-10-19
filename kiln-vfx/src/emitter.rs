//! Particle emitter configuration and emission shapes.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Emission shape for particle spawning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EmissionShape {
    /// Single point.
    Point,
    /// Circle with radius.
    Circle { radius: f32 },
    /// Rectangle with width and height.
    Rectangle { width: f32, height: f32 },
}

impl Default for EmissionShape {
    fn default() -> Self {
        Self::Point
    }
}

/// Particle emitter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Active particle emitter.
#[derive(Debug, Clone)]
pub struct Emitter {
    config: EmitterConfig,
    /// Time accumulator for emission timing.
    time_since_last_emit: f32,
    /// Emitter position in world space.
    position: [f32; 2],
}

impl Emitter {
    /// Create a new emitter with the given configuration.
    pub fn new(config: EmitterConfig) -> Self {
        Self {
            config,
            time_since_last_emit: 0.0,
            position: [0.0; 2],
        }

    }

    /// Get the emitter configuration.
    pub fn config(&self) -> &EmitterConfig {
        &self.config
    }

    /// Get the emitter position.
    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    /// Set the emitter position.
    pub fn set_position(&mut self, position: [f32; 2]) {
        self.position = position;
    }

    /// Update the emitter and return the number of particles to emit this frame.
    pub fn update(&mut self, dt: f32) -> u32 {
        self.time_since_last_emit += dt;

        if self.config.rate <= 0.0 {
            return 0;
        }

        let emit_interval = 1.0 / self.config.rate;
        let mut count = 0;

        while self.time_since_last_emit >= emit_interval {
            self.time_since_last_emit -= emit_interval;
            count += 1;
        }

        count
    }

    /// Generate a spawn position based on the emission shape.
    pub fn spawn_position(&self) -> [f32; 2] {
        match self.config.shape {
            EmissionShape::Point => self.position,
            EmissionShape::Circle { radius } => {
                let angle = rand::random::<f32>() * 2.0 * PI;
                let r = radius * rand::random::<f32>().sqrt();
                [
                    self.position[0] + r * angle.cos(),
                    self.position[1] + r * angle.sin(),
                ]
            }
            EmissionShape::Rectangle { width, height } => [
                self.position[0] + (rand::random::<f32>() - 0.5) * width,
                self.position[1] + (rand::random::<f32>() - 0.5) * height,
            ],
        }
    }

    /// Generate initial velocity based on speed and angle ranges.
    pub fn spawn_velocity(&self) -> [f32; 2] {
        let speed = rand_range(self.config.speed_range[0], self.config.speed_range[1]);
        let angle = rand_range(self.config.angle_range[0], self.config.angle_range[1]);
        [speed * angle.cos(), speed * angle.sin()]
    }

    /// Generate initial lifetime.
    pub fn spawn_lifetime(&self) -> f32 {
        rand_range(self.config.lifetime_range[0], self.config.lifetime_range[1])
    }

    /// Generate initial size.
    pub fn spawn_size(&self) -> f32 {
        rand_range(self.config.size_range[0], self.config.size_range[1])
    }

    /// Get initial color.
    pub fn spawn_color(&self) -> [f32; 4] {
        self.config.color
    }

    /// Get gravity.
    pub fn gravity(&self) -> [f32; 2] {
        self.config.gravity
    }

    /// Check if particles should fade out.
    pub fn fade_out(&self) -> bool {
        self.config.fade_out
    }
}

/// Helper: random value in range [min, max].
fn rand_range(min: f32, max: f32) -> f32 {
    if min >= max {
        return min;
    }
    min + rand::random::<f32>() * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitter_config_default() {
        let config = EmitterConfig::default();
        assert_eq!(config.shape, EmissionShape::Point);
        assert_eq!(config.rate, 10.0);
        assert_eq!(config.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn emitter_rate() {
        let config = EmitterConfig {
            rate: 10.0,
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);

        // Should emit 10 particles per second
        let count = emitter.update(0.1);
        assert_eq!(count, 1);

        let count = emitter.update(0.1);
        assert_eq!(count, 1);
    }

    #[test]
    fn emitter_position() {
        let emitter = Emitter::new(EmitterConfig::default());
        assert_eq!(emitter.position(), [0.0; 2]);

        let mut emitter = Emitter::new(EmitterConfig::default());
        emitter.set_position([5.0, 10.0]);
        assert_eq!(emitter.position(), [5.0, 10.0]);
    }

    #[test]
    fn emission_shape_point() {
        let config = EmitterConfig {
            shape: EmissionShape::Point,
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);
        emitter.set_position([1.0, 2.0]);

        let pos = emitter.spawn_position();
        assert_eq!(pos, [1.0, 2.0]);
    }

    #[test]
    fn emission_shape_circle() {
        let config = EmitterConfig {
            shape: EmissionShape::Circle { radius: 10.0 },
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);
        emitter.set_position([0.0, 0.0]);

        let pos = emitter.spawn_position();
        // Check that position is within circle
        let dist = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
        assert!(dist <= 10.0);
    }

    #[test]
    fn emission_shape_rectangle() {
        let config = EmitterConfig {
            shape: EmissionShape::Rectangle {
                width: 10.0,
                height: 20.0,
            },
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);
        emitter.set_position([0.0, 0.0]);

        let pos = emitter.spawn_position();
        // Check that position is within rectangle
        assert!(pos[0] >= -5.0 && pos[0] <= 5.0);
        assert!(pos[1] >= -10.0 && pos[1] <= 10.0);
    }
}
