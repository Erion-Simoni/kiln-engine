//! Particle system for CPU simulation.

use crate::emitter::Emitter;
use crate::particle::Particle;

/// Particle system configuration.
#[derive(Debug, Clone, Copy)]
pub struct ParticleSystemConfig {
    /// Maximum number of particles in the system.
    pub max_particles: usize,
}

impl Default for ParticleSystemConfig {
    fn default() -> Self {
        Self {
            max_particles: 10_000,
        }
    }
}

/// CPU-based particle system.
pub struct ParticleSystem {
    config: ParticleSystemConfig,
    emitters: Vec<Emitter>,
    particles: Vec<Particle>,
}

impl ParticleSystem {
    /// Create a new particle system with the given configuration.
    pub fn new(config: ParticleSystemConfig) -> Self {
        Self {
            config,
            emitters: Vec::new(),
            particles: Vec::with_capacity(config.max_particles),
        }
    }

    /// Add an emitter to the system.
    pub fn add_emitter(&mut self, emitter: Emitter) {
        self.emitters.push(emitter);
    }

    /// Get the number of active particles.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get the particles (read-only).
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Get the internal particles vector (for testing).
    #[cfg(test)]
    pub fn particles_internal(&self) -> &Vec<Particle> {
        &self.particles
    }

    /// Update the particle system for one frame.
    pub fn update(&mut self, dt: f32) {
        // Update emitters and spawn new particles
        for emitter in &mut self.emitters {
            let count = emitter.update(dt);

            for _ in 0..count {
                if self.particles.len() >= self.config.max_particles {
                    break;
                }

                let mut particle = Particle::new();
                particle.position = emitter.spawn_position();
                particle.velocity = emitter.spawn_velocity();
                particle.lifetime = emitter.spawn_lifetime();
                particle.size = emitter.spawn_size();
                particle.color = emitter.spawn_color();
                self.particles.push(particle);
            }
        }

        // Update existing particles
        for particle in &mut self.particles {
            particle.age += dt;

            // Apply gravity
            let gravity = self.emitters.first().map_or([0.0, 0.0], |e| e.gravity());
            particle.velocity[0] += gravity[0] * dt;
            particle.velocity[1] += gravity[1] * dt;

            // Update position
            particle.position[0] += particle.velocity[0] * dt;
            particle.position[1] += particle.velocity[1] * dt;

            // Fade out if enabled
            if self.emitters.first().map_or(false, |e| e.fade_out()) {
                let alpha = 1.0 - particle.normalized_age();
                particle.color[3] = alpha;
            }
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());
    }

    /// Clear all particles and emitters.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emitters.clear();
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new(ParticleSystemConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::EmitterConfig;

    #[test]
    fn system_creation() {
        let config = ParticleSystemConfig {
            max_particles: 100,
        };
        let system = ParticleSystem::new(config);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn system_default() {
        let system = ParticleSystem::default();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.particles_internal().capacity(), 10_000);
    }

    #[test]
    fn add_emitter() {
        let mut system = ParticleSystem::default();
        let config = EmitterConfig::default();
        let emitter = Emitter::new(config);
        system.add_emitter(emitter);
        assert_eq!(system.emitters.len(), 1);
    }

    #[test]
    fn particle_spawning() {
        let mut system = ParticleSystem::default();
        let config = EmitterConfig {
            rate: 10.0,
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);
        emitter.set_position([0.0, 0.0]);
        system.add_emitter(emitter);

        system.update(0.1);
        assert_eq!(system.particle_count(), 1);

        system.update(0.1);
        assert_eq!(system.particle_count(), 2);
    }

    #[test]
    fn particle_death() {
        let mut system = ParticleSystem::default();
        let config = EmitterConfig {
            rate: 10.0,
            lifetime_range: [0.15, 0.15],
            ..Default::default()
        };
        let emitter = Emitter::new(config);
        system.add_emitter(emitter);

        system.update(0.1);
        assert_eq!(system.particle_count(), 1);

        system.update(0.1);
        // Old particle should die (age 0.2 > lifetime 0.15), new one spawns
        assert_eq!(system.particle_count(), 1);
    }

    #[test]
    fn max_particles_cap() {
        let config = ParticleSystemConfig {
            max_particles: 5,
        };
        let mut system = ParticleSystem::new(config);
        let emitter_config = EmitterConfig {
            rate: 100.0,
            ..Default::default()
        };
        system.add_emitter(Emitter::new(emitter_config));

        system.update(1.0);
        assert!(system.particle_count() <= 5);
    }

    #[test]
    fn particle_physics() {
        let mut system = ParticleSystem::default();
        let config = EmitterConfig {
            rate: 10.0,
            gravity: [0.0, -10.0],
            speed_range: [0.0, 0.0], // No initial velocity
            angle_range: [0.0, 0.0],
            ..Default::default()
        };
        let mut emitter = Emitter::new(config);
        emitter.set_position([0.0, 0.0]);
        system.add_emitter(emitter);

        system.update(0.1);
        let particle = &system.particles_internal()[0];

        // Particle should have moved due to gravity
        assert!(particle.position[1] < 0.0);
    }

    #[test]
    fn fade_out() {
        let mut system = ParticleSystem::default();
        let config = EmitterConfig {
            rate: 10.0,
            fade_out: true,
            lifetime_range: [1.0, 1.0],
            ..Default::default()
        };
        system.add_emitter(Emitter::new(config));

        system.update(0.1);
        let particle = &system.particles_internal()[0];

        // Alpha should be less than 1.0 after 0.1 seconds
        assert!(particle.color[3] < 1.0);
    }

    #[test]
    fn clear() {
        let mut system = ParticleSystem::default();
        system.add_emitter(Emitter::new(EmitterConfig::default()));
        system.update(1.0);

        assert!(system.particle_count() > 0);

        system.clear();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.emitters.len(), 0);
    }
}
