//! Fixed timestep game loop

use kiln_core::Clock;
use std::time::Duration;

const FIXED_TIMESTEP: Duration = Duration::from_secs_f64(1.0 / 60.0);

pub struct GameLoop {
    clock: Clock,
    accumulator: Duration,
}

impl GameLoop {
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            accumulator: Duration::ZERO,
        }
    }

    pub fn tick<F>(&mut self, mut update: F)
    where
        F: FnMut(Duration),
    {
        self.clock.tick();
        self.accumulator += self.clock.delta();

        while self.accumulator >= FIXED_TIMESTEP {
            update(FIXED_TIMESTEP);
            self.accumulator -= FIXED_TIMESTEP;
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
