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
        }
    }

    /// Add a frame to the clip.
    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Get the total duration of the clip.
    pub fn total_duration(&self) -> f32 {
        self.frames.iter().map(|f| f.duration).sum()
    }

    /// Get the frame at a given time, assuming the clip starts at t=0.
    ///
    /// Returns `(frame_index, local_time_within_frame)`.
    pub fn frame_at_time(&self, t: f32) -> Result<(usize, f32), ClipError> {
        if self.frames.is_empty() {
            return Err(ClipError::Empty);
        }

        let total = self.total_duration();
        if total <= 0.0 {
            return Ok((0, 0.0));
        }

        let mut time = if self.loops { t % total } else { t.min(total) };

        for (i, frame) in self.frames.iter().enumerate() {
            if time < frame.duration {
                return Ok((i, time));
            }
            time -= frame.duration;
        }

        Ok((self.frames.len() - 1, 0.0))
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
