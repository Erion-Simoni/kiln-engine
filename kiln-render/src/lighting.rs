//! 2D lighting system.
//!
//! Provides point lights with color, intensity, and distance attenuation.
//! Lights are culled to only affect sprites within their radius.

use crate::{SpriteInstance, SpriteRenderer, TextureId};
use wgpu::{Device, Queue};

/// A 2D point light.
#[derive(Debug, Clone, Copy)]
pub struct Light {
    /// Position in world space.
    pub position: [f32; 2],
    /// Light color (RGB).
    pub color: [f32; 3],
    /// Light intensity (0.0 = off, 1.0 = normal, >1.0 = overbright).
    pub intensity: f32,
    /// Radius of the light's influence in world units.
    pub radius: f32,
    /// Whether the light is enabled.
    pub enabled: bool,
}

impl Light {
    /// Create a new point light.
    pub fn new(position: [f32; 2], color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
            enabled: true,
        }
    }

    /// Check if this light affects the given position.
    pub fn affects_point(&self, point: [f32; 2]) -> bool {
        if !self.enabled {
            return false;
        }
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dist_sq = dx * dx + dy * dy;
        dist_sq <= self.radius * self.radius
    }

    /// Compute the light contribution at a given point.
    /// Returns the light color with intensity attenuated by distance.
    pub fn contribution_at(&self, point: [f32; 2]) -> [f32; 3] {
        if !self.enabled {
            return [0.0, 0.0, 0.0];
        }
        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist >= self.radius {
            return [0.0, 0.0, 0.0];
        }
        let attenuation = 1.0 - (dist / self.radius);
        let attenuation = attenuation * attenuation; // Quadratic falloff
        [
            self.color[0] * self.intensity * attenuation,
            self.color[1] * self.intensity * attenuation,
            self.color[2] * self.intensity * attenuation,
        ]
    }
}

/// A collection of 2D lights for a scene.
#[derive(Debug, Clone, Default)]
pub struct Lighting {
    lights: Vec<Light>,
}

impl Lighting {
    /// Create a new empty lighting system.
    pub fn new() -> Self {
        Self { lights: Vec::new() }
    }

    /// Add a light to the lighting system.
    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    /// Get all lights.
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Get mutable access to all lights.
    pub fn lights_mut(&mut self) -> &mut Vec<Light> {
        &mut self.lights
    }

    /// Get the number of lights.
    pub fn len(&self) -> usize {
        self.lights.len()
    }

    /// Check if the lighting system is empty.
    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }

    /// Render lights as additive sprite batches.
    ///
    /// Each light is rendered as a soft additive sprite centered on its position,
    /// with size based on its radius and color/intensity baked into the sprite color.
    pub fn render(&self, renderer: &mut SpriteRenderer) {
        for light in &self.lights {
            if !light.enabled {
                continue;
            }
            // Compute sprite size from light radius
            let size = [light.radius * 2.0, light.radius * 2.0];
            let color = [
                light.color[0] * light.intensity,
                light.color[1] * light.intensity,
                light.color[2] * light.intensity,
                0.5, // Alpha for soft blending
            ];
            let sprite = SpriteInstance::colored(light.position, size, color);
            renderer.push_textured(TextureId::WHITE, sprite);
        }
    }
}
