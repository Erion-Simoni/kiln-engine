//! Shared clip playback, crossed frame events and non-looping transitions.
use kiln_core::{Clip, Frame, FrameEvent};
use kiln_render::TextureId;
use std::collections::HashMap;

/// Logical clip playback; GPU bindings and pending frame events are transient.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimationSnapshot {
    pub clips: std::collections::BTreeMap<String, Clip>,
    pub current: Option<String>,
    pub elapsed: f64,
    pub speed: f32,
    pub started: bool,
}

#[derive(Debug)]
pub struct AnimationPlayer {
    clips: HashMap<String, Clip>,
    textures: HashMap<String, (TextureId, [u32; 2])>,
    current: Option<String>,
    elapsed: f64,
    speed: f32,
    started: bool,
    events: Vec<FrameEvent>,
}

impl AnimationPlayer {
    pub fn snapshot(&self) -> Result<AnimationSnapshot, String> {
        let saved = AnimationSnapshot {
            clips: self
                .clips
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            current: self.current.clone(),
            elapsed: self.elapsed,
            speed: self.speed,
            started: self.started,
        };
        // Verify that a newly written snapshot can be restored before replacing a slot.
        Self::restore_snapshot(&saved, |path| {
            self.clips.iter().find_map(|(id, clip)| {
                (clip.texture == path)
                    .then(|| self.textures.get(id).copied())
                    .flatten()
            })
        })?;
        Ok(saved)
    }

    pub fn restore_snapshot(
        saved: &AnimationSnapshot,
        mut resolve: impl FnMut(&str) -> Option<(TextureId, [u32; 2])>,
    ) -> Result<Self, String> {
        if saved.clips.len() > 256
            || !saved.elapsed.is_finite()
            || saved.elapsed < 0.0
            || !saved.speed.is_finite()
            || saved.speed < 0.0
        {
            return Err("Invalid saved animation state".into());
        }
        let mut player = Self::new();
        for (id, clip) in &saved.clips {
            if id != &clip.name || clip.frames.len() > 8192 {
                return Err("Invalid saved animation clip".into());
            }
            let (texture, size) = resolve(&clip.texture).ok_or("Missing animation texture")?;
            clip.validate(size)?;
            player.add_bound_clip(clip.clone(), texture, size);
        }
        if saved
            .current
            .as_ref()
            .is_some_and(|id| !player.clips.contains_key(id))
        {
            return Err("Missing saved animation clip".into());
        }
        player.current.clone_from(&saved.current);
        player.elapsed = saved.elapsed;
        player.speed = saved.speed;
        player.started = saved.started;
        Ok(player)
    }
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            textures: HashMap::new(),
            current: None,
            elapsed: 0.0,
            speed: 1.0,
            started: false,
            events: Vec::new(),
        }
    }
    /// Unbound clips infer atlas extents; native hosts bind decoded dimensions.
    pub fn add_clip(&mut self, clip: Clip) {
        let size = clip.frames.iter().fold([1, 1], |size, f| {
            [
                size[0].max(f.rect[0].saturating_add(f.rect[2])),
                size[1].max(f.rect[1].saturating_add(f.rect[3])),
            ]
        });
        self.add_bound_clip(clip, TextureId::WHITE, size);
    }
    pub fn add_bound_clip(&mut self, clip: Clip, texture: TextureId, size: [u32; 2]) {
        self.textures.insert(clip.name.clone(), (texture, size));
        self.clips.insert(clip.name.clone(), clip);
    }
    /// Switching state starts a clip; requesting the current state preserves time.
    pub fn play(&mut self, name: &str) {
        if self.current.as_deref() != Some(name) && self.clips.contains_key(name) {
            self.current = Some(name.into());
            self.elapsed = 0.0;
            self.started = false;
        }
    }
    pub fn stop(&mut self) {
        self.current = None;
        self.elapsed = 0.0;
        self.started = false;
        self.events.clear();
    }
    pub fn current_clip(&self) -> Option<&str> {
        self.current.as_deref()
    }
    pub fn elapsed(&self) -> f32 {
        self.elapsed as f32
    }
    /// Scrub without emitting historical events, used by the authoring preview.
    pub fn seek(&mut self, time: f32) {
        if !time.is_finite() {
            return;
        }
        if let Some(clip) = self.current.as_ref().and_then(|name| self.clips.get(name)) {
            let duration = clip.total_duration();
            self.elapsed = if duration > 0.0 {
                (if clip.loops {
                    time.max(0.0).rem_euclid(duration)
                } else {
                    time.clamp(0.0, duration)
                }) as f64
            } else {
                0.0
            };
            self.started = true;
            self.events.clear();
        }
    }
    pub fn set_speed(&mut self, speed: f32) {
        if speed.is_finite() {
            self.speed = speed.max(0.0);
        }
    }
    pub fn get_clip_mut(&mut self, name: &str) -> Option<&mut Clip> {
        self.clips.get_mut(name)
    }
    pub fn clips_mut(&mut self) -> &mut HashMap<String, Clip> {
        &mut self.clips
    }
    pub fn take_events(&mut self) -> Vec<FrameEvent> {
        std::mem::take(&mut self.events)
    }
    pub fn frame(&self) -> Option<&Frame> {
        let clip = self.clips.get(self.current.as_ref()?)?;
        Some(&clip.frames[clip.frame_at_time(self.elapsed as f32).ok()?.0])
    }
    pub fn current_uv_rect(&self) -> Option<[f32; 4]> {
        let frame = self.frame()?;
        let (_, size) = self.textures.get(self.current.as_ref()?)?;
        if size.contains(&0) {
            return None;
        }
        let [x, y, w, h] = frame.rect;
        Some([
            x as f32 / size[0] as f32,
            y as f32 / size[1] as f32,
            x.saturating_add(w) as f32 / size[0] as f32,
            y.saturating_add(h) as f32 / size[1] as f32,
        ])
    }
    pub fn apply_to_sprite(&self, sprite: &mut crate::Sprite) {
        if let Some(uv) = self.current_uv_rect() {
            sprite.uv_rect = uv;
            if let Some((texture, _)) = self
                .current
                .as_ref()
                .and_then(|name| self.textures.get(name))
            {
                sprite.texture = *texture;
            }
            if let Some(frame) = self.frame() {
                sprite.pivot = frame.pivot;
            }
        }
    }
    /// Consume real elapsed time, reporting every crossed frame once.
    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
        let mut remaining = dt as f64 * self.speed as f64;
        for _ in 0..4096 {
            let Some(clip) = self.current.as_ref().and_then(|name| self.clips.get(name)) else {
                return;
            };
            let total = clip.total_duration() as f64;
            if !total.is_finite() || total <= 0.0 || clip.frames.is_empty() {
                return;
            }
            if !self.started {
                self.events.extend(clip.frames[0].events.clone());
                self.started = true;
            }
            let end = (self.elapsed + remaining).min(total);
            let mut start = 0.0;
            for frame in &clip.frames {
                if start > self.elapsed && start <= end {
                    self.events.extend(frame.events.clone());
                }
                start += frame.duration as f64;
            }
            remaining -= (end - self.elapsed).max(0.0);
            self.elapsed = end;
            if end < total {
                return;
            }
            if clip.loops {
                self.elapsed = 0.0;
                self.started = false;
            } else if let Some(next) = clip
                .next
                .clone()
                .filter(|next| self.clips.contains_key(next))
            {
                self.current = Some(next);
                self.elapsed = 0.0;
                self.started = false;
            } else {
                return;
            }
            if remaining <= 0.0 {
                return;
            }
        }
    }
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip(name: &str, texture: &str, loops: bool) -> Clip {
        let mut clip = Clip::new(name.into(), texture.into());
        clip.loops = loops;
        for index in 0..2 {
            clip.add_frame(Frame {
                rect: [index * 16, 0, 16, 16],
                pivot: [0.25, 1.0],
                duration: 0.125,
                events: vec![FrameEvent {
                    name: format!("frame{index}"),
                    payload: Some(index.to_string()),
                }],
            });
        }
        clip
    }
    #[test]
    fn actual_sheet_uvs_events_and_transitions_share_one_clock() {
        let mut player = AnimationPlayer::new();
        let mut walk = clip("walk", "walk.png", false);
        walk.next = Some("idle".into());
        let mut idle = clip("idle", "idle.png", true);
        for frame in &mut idle.frames {
            frame.events.clear();
        }
        player.add_bound_clip(walk, TextureId(7), [64, 16]);
        player.add_bound_clip(idle, TextureId(8), [32, 32]);
        player.play("walk");
        player.update(0.125);
        assert_eq!(player.current_uv_rect(), Some([0.25, 0.0, 0.5, 1.0]));
        assert_eq!(
            player
                .take_events()
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            ["frame0", "frame1"]
        );
        player.play("walk");
        assert_eq!(
            player.elapsed(),
            0.125,
            "state requests must not reset each tick"
        );
        player.update(0.1875);
        assert_eq!(player.current_clip(), Some("idle"));
        assert_eq!(player.elapsed(), 0.0625);
        let mut sprite = crate::Sprite::default();
        player.apply_to_sprite(&mut sprite);
        assert_eq!(sprite.texture, TextureId(8));
        assert_eq!(sprite.uv_rect, [0.0, 0.0, 0.5, 0.5]);
        assert_eq!(sprite.pivot, [0.25, 1.0]);
    }
    #[test]
    fn loop_events_cover_skipped_frames_without_repeating_stationary_updates() {
        let mut player = AnimationPlayer::default();
        player.add_clip(clip("run", "run.png", true));
        player.play("run");
        player.update(0.625);
        assert_eq!(
            player
                .take_events()
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            ["frame0", "frame1", "frame0", "frame1", "frame0", "frame1"]
        );
        player.update(0.0);
        assert!(player.take_events().is_empty());
        player.update(f32::NAN);
        assert_eq!(player.elapsed(), 0.125);
        player.seek(0.0);
        player.update(0.0625);
        assert!(player.take_events().is_empty());
        player.stop();
        player.update(1.0);
        assert!(player.current_uv_rect().is_none());
    }
}
