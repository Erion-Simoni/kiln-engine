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
}

impl Sprite {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            size: [32.0, 32.0],
            uv_rect: FULL_UV,
            texture: TextureId::WHITE,
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
    current_clip: Option<String>,
    elapsed: f32,
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            current_clip: None,
            elapsed: 0.0,
        }
    }

    /// Add a clip to the player.
    pub fn add_clip(&mut self, clip: Clip) {
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
        self.elapsed += dt;
    }

    /// Get the current frame's UV rect (pixel coordinates), if a clip is playing.
    pub fn current_uv_rect(&self) -> Option<[u32; 4]> {
        let clip_name = self.current_clip.as_ref()?;
        let clip = self.clips.get(clip_name)?;
        let (frame_idx, _) = clip.frame_at_time(self.elapsed).ok()?;
        let frame = &clip.frames[frame_idx];
        Some(frame.rect)
    }
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}
