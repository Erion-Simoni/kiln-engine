//! Shared fixed-step timing and frame-level Play controls.

/// Fixed gameplay step used by editor and standalone hosts.
pub const PLAY_STEP: f64 = 1.0 / 60.0;

/// Accumulates wall time without advancing paused games or accumulating a
/// focus-loss backlog. A requested step always advances exactly one frame.
#[derive(Debug, Default)]
pub struct PlayClock {
    paused: bool,
    step_requested: bool,
    accumulator: f64,
}

impl PlayClock {
    /// Whether normal gameplay updates are suspended.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Pause or resume, discarding any partial frame.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.accumulator = 0.0;
        self.step_requested = false;
    }

    /// Pause and request exactly one fixed gameplay frame.
    pub fn request_step(&mut self) {
        self.paused = true;
        self.step_requested = true;
    }

    /// Number of fixed updates due for a wall-time interval. Long frames are
    /// capped at 250 ms to bound catch-up work after a stalled host.
    pub fn advance(&mut self, elapsed: f64) -> usize {
        if self.step_requested {
            self.step_requested = false;
            return 1;
        }
        if self.paused || !elapsed.is_finite() || elapsed < 0.0 {
            return 0;
        }
        self.accumulator += elapsed.min(0.25);
        let count = ((self.accumulator + 1e-12) / PLAY_STEP).floor() as usize;
        self.accumulator = (self.accumulator - count as f64 * PLAY_STEP).max(0.0);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_step_resume_discards_paused_time() {
        let mut clock = PlayClock::default();
        assert_eq!(clock.advance(PLAY_STEP / 2.0), 0);
        assert_eq!(clock.advance(PLAY_STEP / 2.0), 1);
        clock.set_paused(true);
        assert_eq!(clock.advance(100.0), 0);
        clock.request_step();
        assert_eq!(clock.advance(100.0), 1);
        assert_eq!(clock.advance(PLAY_STEP), 0);
        clock.set_paused(false);
        assert_eq!(clock.advance(PLAY_STEP), 1);
        assert_eq!(clock.advance(f64::NAN), 0);
        assert_eq!(clock.advance(100.0), 15);
    }
}
