//! Kiln VFX crate — particle systems and 2D visual effects.

pub mod emitter;
pub mod gpu;
pub mod particle;
pub mod system;

pub use emitter::{Emitter, EmitterConfig, EmissionShape};
pub use gpu::{GpuParticleSystem, GpuParticleSystemConfig, MAX_GPU_PARTICLES};
pub use particle::{Particle, ParticleData};
pub use system::{ParticleSystem, ParticleSystemConfig};
