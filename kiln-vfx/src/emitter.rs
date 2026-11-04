//! Particle emitter configuration and emission shapes.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};

pub use kiln_core::vfx::{EmissionShape, EmitterConfig};

/// Active particle emitter.
#[derive(Debug)]
pub struct Emitter {
    rng: AtomicU64,
    config: EmitterConfig,
    /// Time accumulator for emission timing.
    time_since_last_emit: f32,
    /// Emitter position in world space.
    position: [f32; 2],
}

impl Emitter {
    fn random_f32(&self) -> f32 {
        let state = self.rng.fetch_add(1, Ordering::Relaxed);
        let x = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = x ^ (x >> 29);
        let x = x.wrapping_mul(0x6D2B79F5CE6A6B5B);
        let x = x ^ (x >> 32);
        x as f32 / u64::MAX as f32
    }
    fn rand_range(&self, min: f32, max: f32) -> f32 {
        if min >= max {
            min
        } else {
            min + self.random_f32() * (max - min)
        }
    }

    /// Create a new emitter with the given configuration.
    pub fn new(config: EmitterConfig) -> Self {
        Self {
            rng: AtomicU64::new(0x1234567890abcdef),
            config,
            time_since_last_emit: 0.0,
            position: [0.0; 2],
        }
    }

    /// Get the emitter configuration.
    pub fn config(&self) -> &EmitterConfig {
        &self.config
    }

    /// Get mutable access to the emitter configuration.
    pub fn config_mut(&mut self) -> &mut EmitterConfig {
        &mut self.config
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
        if !dt.is_finite() || dt <= 0.0 || self.config.validate().is_err() {
            return 0;
        }
        self.time_since_last_emit += dt;

        if self.config.rate <= 0.0 {
            return 0;
        }

        let emit_interval = 1.0 / self.config.rate;
        let count = (self.time_since_last_emit / emit_interval)
            .floor()
            .min(100_000.0) as u32;
        self.time_since_last_emit %= emit_interval;
        count
    }

    /// Generate a spawn position based on the emission shape.
    pub fn spawn_position(&self) -> [f32; 2] {
        match self.config.shape {
            EmissionShape::Point => self.position,
            EmissionShape::Circle { radius } => {
                let angle = self.random_f32() * 2.0 * PI;
                let r = radius * self.random_f32().sqrt();
                [
                    self.position[0] + r * angle.cos(),
                    self.position[1] + r * angle.sin(),
                ]
            }
            EmissionShape::Rectangle { width, height } => [
                self.position[0] + (self.random_f32() - 0.5) * width,
                self.position[1] + (self.random_f32() - 0.5) * height,
            ],
        }
    }

    /// Generate initial velocity based on speed and angle ranges.
    pub fn spawn_velocity(&self) -> [f32; 2] {
        let speed = self.rand_range(self.config.speed_range[0], self.config.speed_range[1]);
        let angle = self.rand_range(self.config.angle_range[0], self.config.angle_range[1]);
        [speed * angle.cos(), speed * angle.sin()]
    }

    /// Generate initial lifetime.
    pub fn spawn_lifetime(&self) -> f32 {
        self.rand_range(self.config.lifetime_range[0], self.config.lifetime_range[1])
    }

    /// Generate initial size.
    pub fn spawn_size(&self) -> f32 {
        self.rand_range(self.config.size_range[0], self.config.size_range[1])
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

impl Clone for Emitter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            time_since_last_emit: self.time_since_last_emit,
            position: self.position,
            rng: AtomicU64::new(self.rng.load(Ordering::Relaxed)),
        }
    }
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
