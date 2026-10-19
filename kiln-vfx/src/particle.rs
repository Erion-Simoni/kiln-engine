//! Particle data structure.

use bytemuck::{Pod, Zeroable};

/// Per-particle data for CPU simulation.
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    /// World-space position.
    pub position: [f32; 2],
    /// Velocity vector.
    pub velocity: [f32; 2],
    /// RGBA color.
    pub color: [f32; 4],
    /// Size in world units.
    pub size: f32,
    /// Total lifetime in seconds.
    pub lifetime: f32,
    /// Current age in seconds.
    pub age: f32,
}

impl Particle {
    /// Create a new particle at the origin.
    pub fn new() -> Self {
        Self {
            position: [0.0; 2],
            velocity: [0.0; 2],
            color: [1.0; 4],
            size: 1.0,
            lifetime: 1.0,
            age: 0.0,
        }
    }

    /// Check if the particle is still alive.
    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }

    /// Get normalized lifetime (0.0 at birth, 1.0 at death).
    pub fn normalized_age(&self) -> f32 {
        if self.lifetime <= 0.0 {
            return 1.0;
        }
        (self.age / self.lifetime).min(1.0)
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU particle data (matches shader layout).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParticleData {
    /// World-space position.
    pub position: [f32; 2],
    /// Velocity vector.
    pub velocity: [f32; 2],
    /// RGBA color.
    pub color: [f32; 4],
    /// Size in world units.
    pub size: f32,
    /// Normalized lifetime (0.0 at birth, 1.0 at death).
    pub normalized_age: f32,
}

impl ParticleData {
    /// Byte stride of the particle layout.
    pub const STRIDE: u64 = std::mem::size_of::<ParticleData>() as u64;

    /// Convert from CPU particle.
    pub fn from_particle(p: &Particle) -> Self {
        Self {
            position: p.position,
            velocity: p.velocity,
            color: p.color,
            size: p.size,
            normalized_age: p.normalized_age(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_size() {
        // 2 + 2 + 4 + 1 + 1 + 1 = 11 f32s = 44 bytes
        assert_eq!(std::mem::size_of::<Particle>(), 44);
    }

    #[test]
    fn particle_data_size() {
        // 2 + 2 + 4 + 1 + 1 = 10 f32s = 40 bytes
        assert_eq!(std::mem::size_of::<ParticleData>(), 40);
    }

    #[test]
    fn particle_lifecycle() {
        let mut p = Particle::new();
        p.lifetime = 2.0;

        assert!(p.is_alive());
        assert_eq!(p.normalized_age(), 0.0);

        p.age = 1.0;
        assert!(p.is_alive());
        assert_eq!(p.normalized_age(), 0.5);

        p.age = 2.0;
        assert!(!p.is_alive());
        assert_eq!(p.normalized_age(), 1.0);
    }

    #[test]
    fn particle_data_conversion() {
        let p = Particle {
            position: [1.0, 2.0],
            velocity: [3.0, 4.0],
            color: [0.5, 0.6, 0.7, 0.8],
            size: 5.0,
            lifetime: 2.0,
            age: 1.0,
        };

        let data = ParticleData::from_particle(&p);
        assert_eq!(data.position, p.position);
        assert_eq!(data.velocity, p.velocity);
        assert_eq!(data.color, p.color);
        assert_eq!(data.size, p.size);
        assert_eq!(data.normalized_age, 0.5);
    }
}
