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
    /// Camera scroll multiplier per axis: one is ordinary world scrolling.
    pub parallax: [f32; 2],
    /// Camera world origin where parallax introduces no render offset.
    pub parallax_origin: [f32; 2],
}

impl Default for SpriteLayout {
    fn default() -> Self {
        Self {
            pivot: [0.5; 2],
            flip_x: false,
            flip_y: false,
            pixels_per_unit: 1.0,
            parallax: [1.0; 2],
            parallax_origin: [0.0; 2],
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
        if !self
            .parallax
            .iter()
            .all(|v| v.is_finite() && (-8.0..=8.0).contains(v))
            || !self.parallax_origin.iter().all(|v| v.is_finite())
        {
            return Err(
                "Parallax factors must be finite in -8..8 and anchors must be finite".into(),
            );
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

#[cfg(test)]
mod parallax_tests {
    use super::*;
    #[test]
    fn old_layout_defaults_and_authored_parallax_roundtrips_and_validates() {
        let old: SpriteLayout = toml::from_str("pixels_per_unit=2.0").unwrap();
        assert_eq!(old.parallax, [1.0; 2]);
        assert_eq!(old.parallax_origin, [0.0; 2]);
        let layout = SpriteLayout {
            parallax: [0.5, -1.0],
            parallax_origin: [100.0, 50.0],
            ..old
        };
        assert_eq!(
            toml::from_str::<SpriteLayout>(&toml::to_string(&layout).unwrap()).unwrap(),
            layout
        );
        layout.validate().unwrap();
        for factor in [f32::NAN, f32::INFINITY, 8.01, -8.01] {
            assert!(SpriteLayout {
                parallax: [factor, 1.0],
                ..layout
            }
            .validate()
            .is_err());
        }
        assert!(SpriteLayout {
            parallax_origin: [f32::INFINITY, 0.0],
            ..layout
        }
        .validate()
        .is_err());
    }
}
