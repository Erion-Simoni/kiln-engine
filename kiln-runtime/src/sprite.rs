//! Sprite component for 2D rendering

use kiln_render::{TextureId, FULL_UV};
use kiln_vfx::{Emitter, EmitterConfig, ParticleSystem};

/// CPU-side sprite drawn via [`kiln_render::SpriteRenderer`].
#[derive(Debug, Clone, hecs::Bundle)]
pub struct Sprite {
    pub color: [f32; 4],
    pub size: [f32; 2],
    /// Normalised UV rect into [`Self::texture`].
    pub uv_rect: [f32; 4],
    /// GPU texture handle (`TextureId::WHITE` = solid colour tint).
    pub texture: TextureId,
    /// Sorting layer — sprites on lower layers are drawn first.
    /// Preserves painter order for transparent sprites across textures.
    pub layer: f32,
    /// Optional material reference for custom shader rendering.
    pub material: Option<String>,
    /// Optional name of an animation clip from `animation_clips` to play
    /// on this sprite.
    pub animation_clip: Option<String>,
    /// Horizontal flip: when true, the sprite is flipped horizontally.
    pub flip_x: bool,
    /// Vertical flip: when true, the sprite is flipped vertically.
    pub flip_y: bool,
    /// Pivot point in local space (0.0 = left/top, 0.5 = center, 1.0 = right/bottom).
    /// Defaults to (0.5, 0.5) center pivot.
    pub pivot: [f32; 2],
    /// Texture pixels represented by one world unit.
    pub pixels_per_unit: f32,
}

impl Sprite {
    /// Bounds of the actual pivoted/rotated draw quad, used for framing/picking.
    pub fn world_bounds(&self, pose: kiln_core::Transform) -> [f32; 4] {
        let draw = self.draw_instance(pose);
        let (sin, cos) = pose.rotation.sin_cos();
        let ex = (cos.abs() * draw.size[0].abs() + sin.abs() * draw.size[1].abs()) * 0.5;
        let ey = (sin.abs() * draw.size[0].abs() + cos.abs() * draw.size[1].abs()) * 0.5;
        [
            draw.position[0] - ex,
            draw.position[1] - ey,
            draw.position[0] + ex,
            draw.position[1] + ey,
        ]
    }
    /// Draw geometry shared by editor/player, including pivot, pixel scale and flips.
    pub fn draw_instance(&self, pose: kiln_core::Transform) -> kiln_render::SpriteInstance {
        let size = [
            self.size[0] / self.pixels_per_unit,
            self.size[1] / self.pixels_per_unit,
        ];
        let local = kiln_core::Vec2::new(
            (0.5 - self.pivot[0]) * size[0],
            (self.pivot[1] - 0.5) * size[1],
        );
        let center =
            pose.position + (local * pose.scale).rotate(kiln_core::Vec2::from_angle(pose.rotation));
        let mut uv = self.uv_rect;
        if self.flip_x {
            uv.swap(0, 2);
        }
        if self.flip_y {
            uv.swap(1, 3);
        }
        kiln_render::SpriteInstance {
            position: center.to_array(),
            size,
            color: self.color,
            uv_rect: uv,
            material_id: 0,
            rotation: 0.0,
        }
        .with_transform(pose.rotation, pose.scale.to_array())
    }
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            size: [32.0, 32.0],
            uv_rect: FULL_UV,
            texture: TextureId::WHITE,
            layer: 0.0,
            material: None,
            animation_clip: None,
            flip_x: false,
            flip_y: false,
            pivot: [0.5, 0.5],
            pixels_per_unit: 1.0,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width, height];
        self
    }

    pub fn with_texture(mut self, texture: TextureId) -> Self {
        self.texture = texture;
        self
    }

    pub fn with_uv(mut self, uv_rect: [f32; 4]) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Set the sorting layer (default 0.0).
    pub fn with_layer(mut self, layer: f32) -> Self {
        self.layer = layer;
        self
    }

    /// Set the animation clip to play on this sprite.
    pub fn with_animation_clip(mut self, clip_name: impl Into<String>) -> Self {
        self.animation_clip = Some(clip_name.into());
        self
    }

    /// Set horizontal flipping (flip horizontally).
    pub fn with_flip_x(mut self, flip: bool) -> Self {
        self.flip_x = flip;
        self
    }

    /// Set vertical flipping (flip vertically).
    pub fn with_flip_y(mut self, flip: bool) -> Self {
        self.flip_y = flip;
        self
    }

    /// Set the pivot point for the sprite (0.0 = left/top, 0.5 = center, 1.0 = right/bottom).
    pub fn with_pivot(mut self, x: f32, y: f32) -> Self {
        self.pivot = [x, y];
        self
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

pub use crate::animation_player::AnimationPlayer;

/// Particle system component for 2D visual effects (smoke, fire, magic, etc.).
///
/// Drives a CPU particle system and can be rendered as sprite instances.
pub struct ParticleSystemComponent {
    system: ParticleSystem,
    pub layer: f32,
}

impl std::fmt::Debug for ParticleSystemComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParticleSystemComponent")
            .field("particle_count", &self.system.particle_count())
            .finish()
    }
}

impl Clone for ParticleSystemComponent {
    fn clone(&self) -> Self {
        // Preserve emitter configuration, random state and live particles.
        Self {
            system: self.system.clone(),
            layer: self.layer,
        }
    }
}

impl ParticleSystemComponent {
    /// Create a new particle system component.
    pub fn new() -> Self {
        Self {
            system: ParticleSystem::new(Default::default()),
            layer: 0.0,
        }
    }

    pub fn from_effect(effect: &kiln_core::visual::ParticleEffect) -> Result<Self, String> {
        effect.validate()?;
        let mut system = ParticleSystem::new(kiln_vfx::ParticleSystemConfig {
            max_particles: effect.max_particles,
        });
        system.emitting = effect.playing;
        for config in &effect.emitters {
            system.add_emitter(Emitter::new(config.clone()));
        }
        Ok(Self {
            system,
            layer: effect.layer,
        })
    }

    /// Add an emitter to the particle system.
    pub fn add_emitter(&mut self, config: EmitterConfig) {
        self.system.add_emitter(Emitter::new(config));
    }

    /// Get mutable access to the particle system.
    pub fn system_mut(&mut self) -> &mut ParticleSystem {
        &mut self.system
    }

    /// Get immutable access to the particle system.
    pub fn system(&self) -> &ParticleSystem {
        &self.system
    }

    /// Particle count.
    pub fn particle_count(&self) -> usize {
        self.system.particle_count()
    }
}

impl Default for ParticleSystemComponent {
    fn default() -> Self {
        Self::new()
    }
}

/// System to update animations and apply UV rects to sprites.
/// Reads AnimationPlayer and Sprite, writes Sprite (uv_rect).
pub fn update_animations(world: &mut hecs::World) {
    update_animations_dt(world, 1.0 / 60.0);
}

pub fn update_animations_dt(world: &mut hecs::World, dt: f32) {
    for (anim, sprite) in world.query_mut::<(&mut AnimationPlayer, &mut Sprite)>() {
        anim.update(dt);
        anim.apply_to_sprite(sprite);
    }
}

/// Emit particles from particle system components for one frame.
pub fn update_particle_systems(world: &mut hecs::World, dt: f32) {
    for (ps, pose) in world.query_mut::<(
        &mut ParticleSystemComponent,
        Option<&crate::GlobalTransform>,
    )>() {
        if let Some(pose) = pose {
            for emitter in &mut ps.system.emitters {
                emitter.set_position(pose.position.to_array());
            }
        }
        ps.system_mut().update(dt);
    }
}
