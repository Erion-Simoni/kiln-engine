//! Saved sprite geometry and particle authoring data.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpriteLayout {
    /// Texture-space pivot: top-left [0,0], center [0.5,0.5].
    pub pivot: [f32; 2],
    pub flip_x: bool,
    pub flip_y: bool,
    pub pixels_per_unit: f32,
}

impl Default for SpriteLayout {
    fn default() -> Self {
        Self {
            pivot: [0.5; 2],
            flip_x: false,
            flip_y: false,
            pixels_per_unit: 1.0,
        }
    }
}

impl SpriteLayout {
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .pivot
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            || !self.pixels_per_unit.is_finite()
            || self.pixels_per_unit <= 0.0
        {
            return Err("Pivot must be within 0–1 and pixel scale must be positive".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParticleEffect {
    pub emitters: Vec<crate::vfx::EmitterConfig>,
    pub max_particles: usize,
    pub playing: bool,
    pub layer: f32,
}

impl Default for ParticleEffect {
    fn default() -> Self {
        Self {
            emitters: vec![Default::default()],
            max_particles: 1000,
            playing: true,
            layer: 0.0,
        }
    }
}

impl ParticleEffect {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_particles == 0
            || self.max_particles > 100_000
            || self.emitters.len() > 64
            || !self.layer.is_finite()
        {
            return Err(
                "Effect needs 1–100000 particles, at most 64 emitters, and a finite layer".into(),
            );
        }
        for emitter in &self.emitters {
            emitter.validate()?;
        }
        Ok(())
    }
}
