//! Sprite component for 2D rendering

use kiln_anim::Clip;
use kiln_render::{TextureId, FULL_UV};
use std::collections::HashMap;

/// CPU-side sprite drawn via [`kiln_render::SpriteRenderer`].
#[derive(Debug, Clone, Copy)]
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
}

impl Sprite {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            size: [32.0, 32.0],
            uv_rect: FULL_UV,
            texture: TextureId::WHITE,
            layer: 0.0,
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
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

/// Animation player that drives sprite animation over time.
///
/// Tracks a set of named animation clips and advances the current frame
/// based on elapsed time. The current frame's UV rect can be read and
/// applied to a sprite's `uv_rect` each frame.
#[derive(Debug)]
pub struct AnimationPlayer {
    clips: HashMap<String, Clip>,
    /// Texture dimensions for each clip (width, height) - used to convert
    /// pixel UV rects to normalized UV rects.
    clip_textures: HashMap<String, (u32, u32)>,
    current_clip: Option<String>,
    elapsed: f32,
    /// Playback speed multiplier (1.0 = normal speed).
    speed: f32,
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            clip_textures: HashMap::new(),
            current_clip: None,
            elapsed: 0.0,
            speed: 1.0,
        }
    }

    /// Add a clip to the player.
    pub fn add_clip(&mut self, clip: Clip) {
        // Store texture dimensions from the first frame
        if let Some(first_frame) = clip.frames.first() {
            self.clip_textures.insert(
                clip.name.clone(),
                (first_frame.rect[2], first_frame.rect[3]),
            );
        }
        self.clips.insert(clip.name.clone(), clip);
    }

    /// Start playing the named clip from the beginning.
    pub fn play(&mut self, clip_name: &str) {
        self.current_clip = Some(clip_name.to_string());
        self.elapsed = 0.0;
    }

    /// Stop the current clip.
    pub fn stop(&mut self) {
        self.current_clip = None;
        self.elapsed = 0.0;
    }

    /// Advance the animation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.elapsed += dt * self.speed;
    }

    /// Set the playback speed multiplier (1.0 = normal speed).
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.0);
    }

    /// Get the current frame's normalized UV rect (0.0 to 1.0), if a clip is playing.
    /// Returns [u0, v0, u1, v1] in normalized texture coordinates.
    pub fn current_uv_rect(&self) -> Option<[f32; 4]> {
        let clip_name = self.current_clip.as_ref()?;
        let clip = self.clips.get(clip_name)?;
        let (frame_idx, _) = clip.frame_at_time(self.elapsed).ok()?;
        let frame = &clip.frames[frame_idx];
        
        // Get texture dimensions for this clip
        let (tex_w, tex_h) = self.clip_textures.get(clip_name)?;
        let tex_w = *tex_w as f32;
        let tex_h = *tex_h as f32;
        
        let [x, y, w, h] = frame.rect;
        Some([
            x as f32 / tex_w,
            y as f32 / tex_h,
            (x + w) as f32 / tex_w,
            (y + h) as f32 / tex_h,
        ])
    }
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// System to update animations and apply UV rects to sprites.
/// Reads AnimationPlayer and Sprite, writes Sprite (uv_rect).
pub fn update_animations(world: &mut hecs::World) {
    for (_, (anim, sprite)) in world.query_mut::<(&mut AnimationPlayer, &mut Sprite)>().iter() {
        anim.update(1.0 / 60.0); // Fixed timestep
        if let Some(uv_rect) = anim.current_uv_rect() {
            sprite.uv_rect = uv_rect;
        }
    }
}
