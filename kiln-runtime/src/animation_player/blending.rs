//! Saved timed crossfades and parameter-driven normalized-phase tree sampling.
use super::*;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Logical source clock and real-time progress for a two-texture crossfade.
pub struct FadeSnapshot {
    pub from_clip: String,
    pub from_time: f64,
    pub elapsed: f64,
    pub duration: f64,
}
struct Sample {
    texture: TextureId,
    uv: [f32; 4],
    pivot: [f32; 2],
}
impl AnimationPlayer {
    /// Start a bounded fade, rejecting active fades/trees without disturbing them.
    pub fn crossfade(&mut self, clip: &str, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite()
            || !(0.0..=60.0).contains(&seconds)
            || !self.clips.contains_key(clip)
        {
            return Err("Crossfade needs existing clip and duration 0..60".into());
        }
        if self.fade.is_some() || self.current_tree().is_some() {
            return Err("Finish the current fade or cut before crossfading a tree".into());
        }
        if self.current.as_deref() == Some(clip) {
            return Ok(());
        }
        let from = self.current.clone().map(|from_clip| FadeSnapshot {
            from_clip,
            from_time: self.elapsed,
            elapsed: 0.0,
            duration: seconds,
        });
        self.play(clip);
        if seconds > 0.0 {
            self.fade = from;
        }
        Ok(())
    }
    fn current_tree(&self) -> Option<&kiln_core::animation_graph::BlendTree> {
        let graph = self.graph.as_ref()?;
        graph.definition.blend_trees.get(&graph.state)
    }
    fn sample_clip(&self, clip: &str, time: f64) -> Option<Sample> {
        let data = self.clips.get(clip)?;
        let frame = &data.frames[data.frame_at_time_seconds(time).ok()?.0];
        let (texture, size) = self.textures.get(clip)?;
        let [x, y, w, h] = frame.rect;
        Some(Sample {
            texture: *texture,
            uv: [
                x as f32 / size[0] as f32,
                y as f32 / size[1] as f32,
                (x + w) as f32 / size[0] as f32,
                (y + h) as f32 / size[1] as f32,
            ],
            pivot: frame.pivot,
        })
    }
    pub(super) fn apply_samples(&self, sprite: &mut crate::Sprite) {
        sprite.blend = None;
        let Some(current) = &self.current else {
            return;
        };
        let pair = if let Some(tree) = self.current_tree() {
            let parameter = self.graph.as_ref().unwrap().parameters[&tree.parameter];
            let (a, b, w) = tree.samples(parameter);
            let phase = self.elapsed / self.clips[current].duration_seconds();
            self.sample_clip(a, phase * self.clips[a].duration_seconds())
                .zip(self.sample_clip(b, phase * self.clips[b].duration_seconds()))
                .map(|(a, b)| (a, b, w))
        } else if let Some(fade) = &self.fade {
            self.sample_clip(current, self.elapsed)
                .zip(self.sample_clip(&fade.from_clip, fade.from_time))
                .map(|(a, b)| (a, b, (1.0 - fade.elapsed / fade.duration) as f32))
        } else {
            self.sample_clip(current, self.elapsed).map(|a| {
                let b = Sample {
                    texture: a.texture,
                    uv: a.uv,
                    pivot: a.pivot,
                };
                (a, b, 0.0)
            })
        };
        if let Some((a, b, w)) = pair {
            sprite.texture = a.texture;
            sprite.uv_rect = a.uv;
            sprite.pivot = [
                a.pivot[0] + (b.pivot[0] - a.pivot[0]) * w,
                a.pivot[1] + (b.pivot[1] - a.pivot[1]) * w,
            ];
            if w > 0.0 {
                sprite.blend = Some(crate::sprite::TextureBlend {
                    texture: b.texture,
                    uv: b.uv,
                    weight: w,
                });
            }
        }
    }
    pub(super) fn advance_fade(&mut self, dt: f64) {
        if let Some(f) = &mut self.fade {
            let clip = &self.clips[&f.from_clip];
            let time = f.from_time + dt * self.speed as f64 * clip.speed as f64;
            f.from_time = if clip.loops {
                time.rem_euclid(clip.duration_seconds())
            } else {
                time.min(clip.duration_seconds())
            };
            f.elapsed = (f.elapsed + dt).min(f.duration);
            if f.elapsed >= f.duration {
                self.fade = None;
            }
        }
    }
    pub(super) fn restore_fade(&mut self, fade: Option<FadeSnapshot>) -> Result<(), String> {
        if let Some(f) = &fade {
            if !self.clips.contains_key(&f.from_clip)
                || !f.from_time.is_finite()
                || f.from_time < 0.0
                || f.from_time > self.clips[&f.from_clip].duration_seconds()
                || !f.duration.is_finite()
                || f.duration <= 0.0
                || f.duration > 60.0
                || !f.elapsed.is_finite()
                || f.elapsed < 0.0
                || f.elapsed >= f.duration
                || self.current.is_none()
                || self.current_tree().is_some()
            {
                return Err("Invalid saved crossfade".into());
            }
        }
        self.fade = fade;
        Ok(())
    }
}
