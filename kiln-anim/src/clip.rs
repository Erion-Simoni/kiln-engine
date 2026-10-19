//! Spritesheet clip definitions with frame events.

/// A single frame in a spritesheet clip.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Texture region rectangle (x, y, width, height) in pixels.
    pub rect: [u32; 4],
    /// Pivot point relative to the frame (0.0 to 1.0, 0.5 is center).
    pub pivot: [f32; 2],
    /// Duration of this frame in seconds.
    pub duration: f32,
    /// Optional events triggered at this frame (e.g., "footstep", "hit").
    pub events: Vec<FrameEvent>,
}

/// An event triggered at a specific frame.
#[derive(Debug, Clone)]
pub struct FrameEvent {
    /// Event name (e.g., "footstep", "hit", "sound_play").
    pub name: String,
    /// Optional event payload (e.g., sound name, parameter value).
    pub payload: Option<String>,
}

/// A spritesheet animation clip.
#[derive(Debug, Clone)]
pub struct Clip {
    /// Clip name.
    pub name: String,
    /// Asset path to the spritesheet texture.
    pub texture: String,
    /// Frames in the clip.
    pub frames: Vec<Frame>,
    /// Whether the clip loops.
    pub loops: bool,
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
    /// Returns (frame_index, local_time_within_frame).
    /// If the clip loops, time wraps around.
    pub fn frame_at_time(&self, t: f32) -> Result<(usize, f32), super::AnimError> {
        if self.frames.is_empty() {
            return Err(super::AnimError::FrameOutOfBounds(0));
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

        // Should not reach here, but return last frame as fallback
        Ok((self.frames.len() - 1, 0.0))
    }

    /// Get all events for a frame at the given time.
    pub fn events_at_time(&self, t: f32) -> Vec<FrameEvent> {
        match self.frame_at_time(t) {
            Ok((frame_index, _)) => {
                if frame_index < self.frames.len() {
                    self.frames[frame_index].events.clone()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }
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

        // Test frame 0
        let (idx, local_t) = clip.frame_at_time(0.05).unwrap();
        assert_eq!(idx, 0);
        assert!((local_t - 0.05).abs() < 0.001);

        // Test frame 1
        let (idx, local_t) = clip.frame_at_time(0.15).unwrap();
        assert_eq!(idx, 1);
        assert!((local_t - 0.05).abs() < 0.001);

        // Test frame 2
        let (idx, local_t) = clip.frame_at_time(0.25).unwrap();
        assert_eq!(idx, 2);
        assert!((local_t - 0.05).abs() < 0.001);

        // Test looping
        let (idx, local_t) = clip.frame_at_time(0.35).unwrap();
        assert_eq!(idx, 0);
        assert!((local_t - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_clip_events_at_time() {
        let mut clip = Clip::new("attack".to_string(), "attack.png".to_string());
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
            events: vec![FrameEvent {
                name: "hit".to_string(),
                payload: Some("damage".to_string()),
            }],
        });

        let events = clip.events_at_time(0.15);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "hit");
        assert_eq!(events[0].payload, Some("damage".to_string()));
    }

    #[test]
    fn test_clip_no_loop() {
        let mut clip = Clip::new("once".to_string(), "once.png".to_string());
        clip.loops = false;
        clip.add_frame(Frame {
            rect: [0, 0, 32, 32],
            pivot: [0.5, 0.5],
            duration: 0.1,
            events: vec![],
        });

        // Beyond total duration should clamp to last frame
        let (idx, _) = clip.frame_at_time(1.0).unwrap();
        assert_eq!(idx, 0);
    }
}
