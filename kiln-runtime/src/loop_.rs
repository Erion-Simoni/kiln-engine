//! Fixed timestep game loop

use kiln_core::Clock;
use std::time::Duration;

pub struct GameLoop {
    clock: Clock,
    playback: crate::play_clock::PlayClock,
}

impl GameLoop {
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            playback: crate::play_clock::PlayClock::default(),
        }
    }

    pub fn tick<F>(&mut self, mut update: F)
    where
        F: FnMut(Duration),
    {
        self.clock.tick();
        let count = self.playback.advance(self.clock.delta().as_secs_f64());
        let fixed_timestep = Duration::from_secs_f64(crate::play_clock::PLAY_STEP);
        for _ in 0..count {
            update(fixed_timestep);
        }
    }

    pub fn delta(&self) -> Duration {
        self.clock.delta()
    }

    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }
}

impl Default for GameLoop {
    fn default() -> Self {
        Self::new()
    }
}
