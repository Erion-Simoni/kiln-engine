//! Time utilities

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Clock {
    start: Instant,
    last_frame: Instant,
    delta: Duration,
    elapsed: Duration,
}

impl Clock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last_frame;
        self.elapsed = now - self.start;
        self.last_frame = now;
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
