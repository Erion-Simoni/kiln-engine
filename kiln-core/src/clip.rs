//! Spritesheet clip definitions stored on a project.

use serde::{Deserialize, Serialize};

/// A single frame in a spritesheet clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// Texture region rectangle (x, y, width, height) in pixels.
    pub rect: [u32; 4],
    /// Pivot point relative to the frame (0.0 to 1.0, 0.5 is center).
    pub pivot: [f32; 2],
    /// Duration of this frame in seconds.
    pub duration: f32,
    /// Optional events triggered at this frame (e.g., "footstep", "hit").
    #[serde(default)]
    pub events: Vec<FrameEvent>,
}

/// An event triggered at a specific frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameEvent {
    /// Event name (e.g., "footstep", "hit", "sound_play").
    pub name: String,
    /// Optional event payload (e.g., sound name, parameter value).
    pub payload: Option<String>,
}

/// A spritesheet animation clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    /// Clip name.
    pub name: String,
    /// Asset path to the spritesheet texture.
    pub texture: String,
    /// Frames in the clip.
    #[serde(default)]
    pub frames: Vec<Frame>,
    /// Whether the clip loops.
    #[serde(default = "default_true")]
    pub loops: bool,
    /// Play reverse interiors after forward frames; one-shot returns to frame zero.
    #[serde(default)]
    pub ping_pong: bool,
    /// Authored playback multiplier; zero pauses without dispatching events.
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Optional automatic transition after a non-looping clip completes.
    #[serde(default)]
    pub next: Option<String>,
}

fn default_speed() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

impl Clip {
    /// Create a new empty clip.
    pub fn new(name: String, texture: String) -> Self {
        Self {
            name,
            texture,
            frames: Vec::new(),
            loops: true,
            ping_pong: false,
            speed: 1.0,
            next: None,
        }
    }

    /// Add a frame to the clip.
    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Frames in playback order, retaining authored frame indices.
    pub fn playback_frames(&self) -> impl Iterator<Item = (usize, &Frame)> {
        let reverse = if self.ping_pong {
            self.frames
                .len()
                .saturating_sub(if self.loops { 2 } else { 1 })
        } else {
            0
        };
        self.frames
            .iter()
            .enumerate()
            .chain(self.frames.iter().enumerate().rev().skip(1).take(reverse))
    }
    /// Precise playback duration before speed scaling.
    pub fn duration_seconds(&self) -> f64 {
        self.playback_frames()
            .map(|(_, frame)| frame.duration as f64)
            .sum()
    }
    /// Compatibility float duration, including reverse frames when authored.
    pub fn total_duration(&self) -> f32 {
        self.duration_seconds() as f32
    }
    /// Sample the canonical timeline without float rounding at frame boundaries.
    pub fn frame_at_time_seconds(&self, t: f64) -> Result<(usize, f64), ClipError> {
        if self.frames.is_empty() {
            return Err(ClipError::Empty);
        }
        let total = self.duration_seconds();
        if !total.is_finite() || total <= 0.0 {
            return Ok((0, 0.0));
        }
        let t = if t.is_finite() { t.max(0.0) } else { 0.0 };
        let mut time = if self.loops {
            t.rem_euclid(total)
        } else {
            t.min(total)
        };
        let mut last = 0;
        for (index, frame) in self.playback_frames() {
            last = index;
            if time < frame.duration as f64 {
                return Ok((index, time));
            }
            time -= frame.duration as f64;
        }
        Ok((last, 0.0))
    }
    /// Frame index and local time using the same timeline as runtime events.
    pub fn frame_at_time(&self, t: f32) -> Result<(usize, f32), ClipError> {
        self.frame_at_time_seconds(t as f64)
            .map(|(index, time)| (index, time as f32))
    }

    /// Validate a clip against the decoded sheet, including overflow-safe rects.
    pub fn validate(&self, sheet: [u32; 2]) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.texture.trim().is_empty()
            || self.frames.is_empty()
            || self.frames.len() > 4096
        {
            return Err("Clip needs a name, texture and 1–4096 frames".into());
        }
        if !self.speed.is_finite()
            || !(0.0..=16.0).contains(&self.speed)
            || !self.total_duration().is_finite()
        {
            return Err(
                "Clip speed must be finite in 0..16 and total duration representable".into(),
            );
        }
        for frame in &self.frames {
            let [x, y, w, h] = frame.rect;
            if w == 0
                || h == 0
                || x.checked_add(w).is_none_or(|end| end > sheet[0])
                || y.checked_add(h).is_none_or(|end| end > sheet[1])
                || !frame.duration.is_finite()
                || frame.duration <= 0.0
                || !frame
                    .pivot
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            {
                return Err(
                    "Frames need positive duration, valid pivots and rectangles inside the sheet"
                        .into(),
                );
            }
        }
        Ok(())
    }

    /// Get all events for a frame at the given time.
    pub fn events_at_time(&self, t: f32) -> Vec<FrameEvent> {
        match self.frame_at_time(t) {
            Ok((frame_index, _)) => self
                .frames
                .get(frame_index)
                .map(|f| f.events.clone())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
}

/// Error from clip playback queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipError {
    /// The clip has no frames.
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_frame_at_time() {
        let mut clip = Clip::new("walk".to_string(), "walk.png".to_string());
        clip.add_frame(Frame {
            rect: [0, 0, 32, 32],
            pivot: [0.5, 0.5],
            duration: 0.1,
            events: vec![],
        });
        clip.add_frame(Frame {
            rect: [32, 0, 32, 32],
            pivot: [0.5, 0.5],
            duration: 0.1,
            events: vec![],
        });
        clip.add_frame(Frame {
            rect: [64, 0, 32, 32],
            pivot: [0.5, 0.5],
            duration: 0.1,
            events: vec![],
        });

        let (idx, local_t) = clip.frame_at_time(0.05).unwrap();
        assert_eq!(idx, 0);
        assert!((local_t - 0.05).abs() < 0.001);

        let (idx, local_t) = clip.frame_at_time(0.15).unwrap();
        assert_eq!(idx, 1);
        assert!((local_t - 0.05).abs() < 0.001);
    }
}

#[cfg(test)]
mod playback_controls_tests {
    use super::*;
    fn three() -> Clip {
        let mut c = Clip::new("bounce".into(), "white.png".into());
        c.ping_pong = true;
        for _ in 0..3 {
            c.add_frame(Frame {
                rect: [0, 0, 1, 1],
                pivot: [0.5; 2],
                duration: 0.125,
                events: vec![],
            });
        }
        c
    }
    #[test]
    fn ping_pong_endpoints_defaults_and_authored_controls_roundtrip() {
        let mut c = three();
        assert_eq!(
            c.playback_frames().map(|(i, _)| i).collect::<Vec<_>>(),
            [0, 1, 2, 1]
        );
        assert_eq!(c.duration_seconds(), 0.5);
        assert_eq!(c.frame_at_time(0.375).unwrap().0, 1);
        assert_eq!(c.frame_at_time(0.5).unwrap().0, 0);
        c.loops = false;
        assert_eq!(c.duration_seconds(), 0.625);
        assert_eq!(c.frame_at_time(0.625).unwrap().0, 0);
        c.speed = 2.0;
        c.validate([1, 1]).unwrap();
        let json = serde_json::to_value(&c).unwrap();
        let mut legacy = json.clone();
        legacy.as_object_mut().unwrap().remove("ping_pong");
        legacy.as_object_mut().unwrap().remove("speed");
        let old: Clip = serde_json::from_value(legacy).unwrap();
        assert!(!old.ping_pong);
        assert_eq!(old.speed, 1.0);
        let round: Clip = serde_json::from_value(json).unwrap();
        assert!(round.ping_pong);
        assert_eq!(round.speed, 2.0);
        for speed in [-1.0, 17.0, f32::NAN] {
            c.speed = speed;
            assert!(c.validate([1, 1]).is_err());
        }
    }
}
