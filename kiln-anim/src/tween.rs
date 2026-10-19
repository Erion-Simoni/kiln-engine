//! Tween library with ease curves for smooth interpolation.

/// Easing functions for smooth interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ease {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuartIn,
    QuartOut,
    QuartInOut,
    QuintIn,
    QuintOut,
    QuintInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    CircIn,
    CircOut,
    CircInOut,
    BackIn,
    BackOut,
    BackInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
}

impl Ease {
    /// Apply the easing function to a value t in [0, 1].
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::QuadIn => t * t,
            Ease::QuadOut => t * (2.0 - t),
            Ease::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Ease::CubicIn => t * t * t,
            Ease::CubicOut => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Ease::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t = t - 1.0;
                    1.0 + 4.0 * t * t * t
                }
            }
            Ease::QuartIn => t * t * t * t,
            Ease::QuartOut => {
                let t = t - 1.0;
                1.0 - t * t * t * t
            }
            Ease::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    let t = t - 1.0;
                    1.0 - 8.0 * t * t * t * t
                }
            }
            Ease::QuintIn => t * t * t * t * t,
            Ease::QuintOut => {
                let t = t - 1.0;
                t * t * t * t * t + 1.0
            }
            Ease::QuintInOut => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    let t = t - 1.0;
                    1.0 + 16.0 * t * t * t * t * t
                }
            }
            Ease::SineIn => 1.0 - (t * std::f32::consts::PI / 2.0).cos(),
            Ease::SineOut => (t * std::f32::consts::PI / 2.0).sin(),
            Ease::SineInOut => -(std::f32::consts::PI).cos() / 2.0 * (t * 2.0).cos() + 0.5,
            Ease::ExpoIn => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(10.0 * t - 10.0)
                }
            }
            Ease::ExpoOut => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            Ease::ExpoInOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0_f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Ease::CircIn => 1.0 - (1.0 - t * t).sqrt(),
            Ease::CircOut => {
                let t = t - 1.0;
                (1.0 - t * t).sqrt()
            }
            Ease::CircInOut => {
                if t < 0.5 {
                    (1.0 - (2.0 * t) * (2.0 * t)).sqrt() / 2.0
                } else {
                    ((1.0 - (2.0 * t - 2.0) * (2.0 * t - 2.0)).sqrt() + 1.0) / 2.0
                }
            }
            Ease::BackIn => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
            Ease::BackOut => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Ease::BackInOut => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    (2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            Ease::ElasticIn => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    -(2.0_f32.powf(10.0 * t - 10.0))
                        * ((t * 10.0 - 10.75) * (2.0 * std::f32::consts::PI / 3.0)).sin()
                }
            }
            Ease::ElasticOut => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    2.0_f32.powf(-10.0 * t)
                        * ((t * 10.0 - 0.75) * (2.0 * std::f32::consts::PI / 3.0)).sin()
                        + 1.0
                }
            }
            Ease::ElasticInOut => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    -(2.0_f32.powf(20.0 * t - 10.0)
                        * ((t * 20.0 - 11.125) * (2.0 * std::f32::consts::PI / 4.5)).sin())
                        / 2.0
                } else {
                    (2.0_f32.powf(-20.0 * t + 10.0)
                        * ((t * 20.0 - 11.125) * (2.0 * std::f32::consts::PI / 4.5)).sin())
                        / 2.0
                        + 1.0
                }
            }
            Ease::BounceIn => 1.0 - Ease::BounceOut.apply(1.0 - t),
            Ease::BounceOut => {
                const N1: f32 = 7.5625;
                const D1: f32 = 2.75;
                if t < 1.0 / D1 {
                    N1 * t * t
                } else if t < 2.0 / D1 {
                    let t = t - 1.5 / D1;
                    N1 * t * t + 0.75
                } else if t < 2.5 / D1 {
                    let t = t - 2.25 / D1;
                    N1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / D1;
                    N1 * t * t + 0.984375
                }
            }
            Ease::BounceInOut => {
                if t < 0.5 {
                    (1.0 - Ease::BounceOut.apply(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Ease::BounceOut.apply(2.0 * t - 1.0)) / 2.0
                }
            }
        }
    }
}

/// A tween animation between two values.
#[derive(Debug, Clone)]
pub struct Tween {
    /// Start value.
    pub from: f32,
    /// End value.
    pub to: f32,
    /// Duration in seconds.
    pub duration: f32,
    /// Easing function.
    pub ease: Ease,
    /// Whether the tween loops.
    pub loops: bool,
}

impl Tween {
    /// Create a new tween.
    pub fn new(from: f32, to: f32, duration: f32, ease: Ease) -> Self {
        Self {
            from,
            to,
            duration,
            ease,
            loops: false,
        }
    }

    /// Set whether the tween loops.
    pub fn with_loops(mut self, loops: bool) -> Self {
        self.loops = loops;
        self
    }

    /// Get the interpolated value at time t.
    pub fn value_at(&self, t: f32) -> Result<f32, super::AnimError> {
        if self.duration <= 0.0 {
            return Err(super::AnimError::InvalidDuration(self.duration));
        }

        let t = if self.loops {
            t % self.duration
        } else {
            t.min(self.duration)
        };

        let progress = t / self.duration;
        let eased = self.ease.apply(progress);
        Ok(self.from + (self.to - self.from) * eased)
    }

    /// Check if the tween is complete at time t (non-looping only).
    pub fn is_complete(&self, t: f32) -> bool {
        !self.loops && t >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ease_linear() {
        assert_eq!(Ease::Linear.apply(0.0), 0.0);
        assert_eq!(Ease::Linear.apply(0.5), 0.5);
        assert_eq!(Ease::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn test_ease_quad() {
        assert!(Ease::QuadIn.apply(0.5) < 0.5);
        assert!(Ease::QuadOut.apply(0.5) > 0.5);
        assert_eq!(Ease::QuadIn.apply(0.0), 0.0);
        assert_eq!(Ease::QuadIn.apply(1.0), 1.0);
    }

    #[test]
    fn test_tween_value() {
        let tween = Tween::new(0.0, 100.0, 1.0, Ease::Linear);
        assert_eq!(tween.value_at(0.0).unwrap(), 0.0);
        assert_eq!(tween.value_at(0.5).unwrap(), 50.0);
        assert_eq!(tween.value_at(1.0).unwrap(), 100.0);
    }

    #[test]
    fn test_tween_with_ease() {
        let tween = Tween::new(0.0, 100.0, 1.0, Ease::QuadIn);
        let v = tween.value_at(0.5).unwrap();
        // QuadIn at 0.5 should be less than 50
        assert!(v < 50.0);
    }

    #[test]
    fn test_tween_loops() {
        let tween = Tween::new(0.0, 100.0, 1.0, Ease::Linear).with_loops(true);
        assert_eq!(tween.value_at(1.5).unwrap(), 50.0);
        assert_eq!(tween.value_at(2.0).unwrap(), 0.0);
    }

    #[test]
    fn test_tween_no_loop_clamp() {
        let tween = Tween::new(0.0, 100.0, 1.0, Ease::Linear);
        assert_eq!(tween.value_at(2.0).unwrap(), 100.0);
    }

    #[test]
    fn test_tween_invalid_duration() {
        let tween = Tween::new(0.0, 100.0, 0.0, Ease::Linear);
        assert!(tween.value_at(0.5).is_err());
    }
}
