//! Deterministic bounded camera offsets, separate from the follow origin.
/// Maximum pending/active camera effects; active overflow replaces the oldest.
pub const MAX_CAMERA_EFFECTS: usize = 32;
/// World-space amplitude, seconds, hertz and an exact deterministic integer seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraShake {
    pub amplitude: [f32; 2],
    pub duration: f32,
    pub frequency: f32,
    pub seed: u32,
}
impl CameraShake {
    /// Validate before changing either the request queue or camera controller.
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .amplitude
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0 && *v <= 4096.0)
            || !self.duration.is_finite()
            || self.duration <= 0.0
            || self.duration > 60.0
            || !self.frequency.is_finite()
            || self.frequency <= 0.0
            || self.frequency > 120.0
        {
            return Err("Camera shake needs amplitude 0..4096, duration (0,60] seconds and frequency (0,120] Hz".into());
        }
        Ok(())
    }
}
/// Transient callback effect; not a saved camera setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraEffect {
    Shake(CameraShake),
    StopShake,
}
#[derive(Debug, Clone)]
struct Active {
    request: CameraShake,
    elapsed: f64,
    fresh: bool,
}
#[derive(Debug, Default, Clone)]
pub(super) struct Mixer {
    active: Vec<Active>,
}
impl Mixer {
    pub(super) fn add(&mut self, request: CameraShake) -> Result<(), String> {
        request.validate()?;
        if self.active.len() == MAX_CAMERA_EFFECTS {
            self.active.remove(0);
        }
        self.active.push(Active {
            request,
            elapsed: 0.0,
            fresh: true,
        });
        Ok(())
    }
    pub(super) fn clear(&mut self) {
        self.active.clear();
    }
    pub(super) fn advance(&mut self, dt: f64) -> [f64; 2] {
        let mut offset = [0.0; 2];
        for effect in &mut self.active {
            if effect.fresh {
                effect.fresh = false;
            } else {
                effect.elapsed += dt;
            }
            if effect.elapsed >= effect.request.duration as f64 {
                continue;
            }
            let r = effect.request;
            let envelope = (1.0 - effect.elapsed / r.duration as f64).powi(2);
            let time = effect.elapsed * r.frequency as f64 * std::f64::consts::TAU;
            for (axis, value) in offset.iter_mut().enumerate() {
                let seed = r
                    .seed
                    .wrapping_add((axis as u32 + 1).wrapping_mul(0x9e3779b9));
                let mixed = seed.wrapping_mul(0x85ebca6b) ^ (seed >> 13);
                let phase = mixed as f64 / u32::MAX as f64 * std::f64::consts::TAU;
                let wave = ((time + phase).sin() + (time * 1.7 + phase * 0.73).sin()) * 0.5;
                *value += r.amplitude[axis] as f64 * envelope * wave;
            }
        }
        self.active
            .retain(|effect| effect.elapsed < effect.request.duration as f64);
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_seed_pause_decay_and_limits_are_bounded() {
        let request = CameraShake {
            amplitude: [10.0, 8.0],
            duration: 1.0,
            frequency: 8.0,
            seed: 17,
        };
        let mut a = Mixer::default();
        let mut b = Mixer::default();
        a.add(request).unwrap();
        b.add(request).unwrap();
        let initial = a.advance(0.0);
        assert_eq!(initial, b.advance(0.0));
        assert_ne!(initial, [0.0, 0.0]);
        assert_eq!(a.advance(0.0), initial);
        for _ in 0..30 {
            let x = a.advance(1.0 / 60.0);
            let y = b.advance(1.0 / 60.0);
            assert_eq!(x, y);
            assert!(x[0].abs() <= 10.0 && x[1].abs() <= 8.0);
        }
        assert_eq!(a.advance(1.0), [0.0, 0.0]);
        assert!(a.active.is_empty());
        let mut full = Mixer::default();
        for seed in 0..40 {
            full.add(CameraShake { seed, ..request }).unwrap();
        }
        assert_eq!(full.active.len(), 32);
        assert_eq!(full.active[0].request.seed, 8);
        full.clear();
        assert_eq!(full.advance(0.0), [0.0, 0.0]);
    }
}
