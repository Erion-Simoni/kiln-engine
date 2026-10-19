//! Particle renderer for 2D VFX.

use crate::sprite_renderer::SpriteInstance;
use kiln_vfx::particle::ParticleData;

/// Particle renderer using sprite batch infrastructure.
pub struct ParticleRenderer {
    /// Reuse sprite renderer for instanced rendering.
    /// Particles are rendered as colored quads (sprites without textures).
    sprite_instances: Vec<SpriteInstance>,
}

impl ParticleRenderer {
    /// Create a new particle renderer.
    pub fn new() -> Self {
        Self {
            sprite_instances: Vec::new(),
        }
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        self.sprite_instances.clear();
    }

    /// Queue particles for rendering.
    pub fn push_particles(&mut self, particles: &[ParticleData]) {
        for p in particles {
            // Skip dead particles
            if p.normalized_age >= 1.0 {
                continue;
            }

            let instance = SpriteInstance {
                position: p.position,
                size: [p.size, p.size], // Square particles
                color: p.color,
            };
            self.sprite_instances.push(instance);
        }
    }

    /// Get sprite instances for rendering.
    pub fn instances(&self) -> &[SpriteInstance] {
        &self.sprite_instances
    }

    /// Number of particles queued this frame.
    pub fn queued_count(&self) -> usize {
        self.sprite_instances.len()
    }
}

impl Default for ParticleRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_renderer_creation() {
        let renderer = ParticleRenderer::new();
        assert_eq!(renderer.queued_count(), 0);
    }

    #[test]
    fn particle_renderer_default() {
        let renderer = ParticleRenderer::default();
        assert_eq!(renderer.queued_count(), 0);
    }

    #[test]
    fn begin_frame_clears() {
        let mut renderer = ParticleRenderer::new();
        let particles = vec![ParticleData {
            position: [0.0; 2],
            velocity: [0.0; 2],
            color: [1.0; 4],
            size: 1.0,
            normalized_age: 0.0,
        }];
        renderer.push_particles(&particles);
        assert_eq!(renderer.queued_count(), 1);

        renderer.begin_frame();
        assert_eq!(renderer.queued_count(), 0);
    }

    #[test]
    fn push_particles() {
        let mut renderer = ParticleRenderer::new();
        let particles = vec![
            ParticleData {
                position: [0.0; 2],
                velocity: [0.0; 2],
                color: [1.0; 4],
                size: 1.0,
                normalized_age: 0.0,
            },
            ParticleData {
                position: [1.0; 2],
                velocity: [0.0; 2],
                color: [1.0; 4],
                size: 1.0,
                normalized_age: 0.0,
            },
        ];
        renderer.push_particles(&particles);
        assert_eq!(renderer.queued_count(), 2);
    }

    #[test]
    fn skip_dead_particles() {
        let mut renderer = ParticleRenderer::new();
        let particles = vec![
            ParticleData {
                position: [0.0; 2],
                velocity: [0.0; 2],
                color: [1.0; 4],
                size: 1.0,
                normalized_age: 0.0, // Alive
            },
            ParticleData {
                position: [1.0; 2],
                velocity: [0.0; 2],
                color: [1.0; 4],
                size: 1.0,
                normalized_age: 1.0, // Dead
            },
        ];
        renderer.push_particles(&particles);
        assert_eq!(renderer.queued_count(), 1);
    }
}
