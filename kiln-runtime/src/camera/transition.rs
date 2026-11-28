//! Transient smooth pan/zoom, independent of authored follow and shake.
/// A world-origin/zoom transition; zero seconds cuts immediately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraTransition {
    pub position: [f32; 2],
    pub zoom: f32,
    pub duration: f32,
}
impl CameraTransition {
    /// Validate before enqueueing or replacing active state.
    pub fn validate(&self) -> Result<(), String> {
        if !self.position.iter().all(|v| v.is_finite())
            || !self.zoom.is_finite()
            || !(1.0 / 64.0..=64.0).contains(&self.zoom)
            || !self.duration.is_finite()
            || !(0.0..=60.0).contains(&self.duration)
        {
            return Err(
                "Camera transition needs finite origin, zoom 1/64..64 and duration 0..60 seconds"
                    .into(),
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct View {
    pub position: [f32; 2],
    pub zoom: f32,
}
#[derive(Debug, Clone)]
struct Active {
    request: CameraTransition,
    from: Option<View>,
    elapsed: f64,
    fresh: bool,
}
#[derive(Debug, Default, Clone)]
pub(super) struct Controller {
    active: Option<Active>,
    last: Option<View>,
}
impl Controller {
    pub(super) fn start(&mut self, request: CameraTransition) -> Result<(), String> {
        request.validate()?;
        self.active = Some(Active {
            request,
            from: self.last,
            elapsed: 0.0,
            fresh: true,
        });
        Ok(())
    }
    pub(super) fn record(&mut self, view: View) {
        self.last = Some(view);
    }
    pub(super) fn stop(&mut self) {
        self.active = None;
    }
    pub(super) fn advance(&mut self, base: View, dt: f64) -> View {
        let view = if let Some(active) = &mut self.active {
            let from = *active.from.get_or_insert(base);
            if active.fresh {
                active.fresh = false;
            } else {
                active.elapsed += dt;
            }
            let r = active.request;
            if active.elapsed >= r.duration as f64 {
                View {
                    position: r.position,
                    zoom: r.zoom,
                }
            } else {
                let t = active.elapsed / r.duration as f64;
                let alpha = t * t * (3.0 - 2.0 * t);
                let lerp = |a: f32, b: f32| (a as f64 + (b as f64 - a as f64) * alpha) as f32;
                View {
                    position: [
                        lerp(from.position[0], r.position[0]),
                        lerp(from.position[1], r.position[1]),
                    ],
                    zoom: lerp(from.zoom, r.zoom),
                }
            }
        } else {
            base
        };
        self.last = Some(view);
        view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> CameraTransition {
        CameraTransition {
            position: [80.0, 40.0],
            zoom: 2.0,
            duration: 1.0,
        }
    }
    #[test]
    fn fresh_pause_endpoint_interruption_and_invalid_replacement() {
        let base = View {
            position: [0.0; 2],
            zoom: 1.0,
        };
        let mut c = Controller::default();
        c.start(request()).unwrap();
        assert_eq!(c.advance(base, 0.5).position, [0.0; 2]);
        let half = c.advance(base, 0.5);
        assert_eq!(half.position, [40.0, 20.0]);
        assert_eq!(half.zoom, 1.5);
        assert_eq!(c.advance(base, 0.0).position, half.position);
        for bad in [
            CameraTransition {
                zoom: 0.0,
                ..request()
            },
            CameraTransition {
                position: [f32::NAN, 0.0],
                ..request()
            },
            CameraTransition {
                duration: -1.0,
                ..request()
            },
            CameraTransition {
                duration: f32::INFINITY,
                ..request()
            },
        ] {
            assert!(c.start(bad).is_err());
        }
        assert_eq!(c.advance(base, 0.0).position, half.position);
        c.start(CameraTransition {
            position: [0.0; 2],
            zoom: 1.0,
            ..request()
        })
        .unwrap();
        assert_eq!(c.advance(base, 0.5).position, half.position);
        assert_eq!(c.advance(base, 1.0).position, [0.0; 2]);
        assert_eq!(c.advance(base, 10.0).zoom, 1.0);
        c.start(CameraTransition {
            duration: 0.0,
            ..request()
        })
        .unwrap();
        assert_eq!(c.advance(base, 0.0).position, request().position);
        assert_eq!(c.advance(base, 100.0).position, request().position);
        c.stop();
        assert_eq!(c.advance(base, 0.0).position, base.position);
    }
    #[test]
    fn elapsed_time_and_extreme_interpolation_remain_finite() {
        let base = View {
            position: [-f32::MAX; 2],
            zoom: 1.0,
        };
        let mut a = Controller::default();
        let mut b = Controller::default();
        for c in [&mut a, &mut b] {
            c.start(CameraTransition {
                position: [f32::MAX; 2],
                ..request()
            })
            .unwrap();
            c.advance(base, 0.0);
        }
        for _ in 0..30 {
            a.advance(base, 1.0 / 60.0);
        }
        for _ in 0..60 {
            b.advance(base, 1.0 / 120.0);
        }
        let x = a.advance(base, 0.0);
        let y = b.advance(base, 0.0);
        assert!(x.position.iter().all(|v| v.is_finite()));
        assert!((x.zoom - y.zoom).abs() < 0.00001);
        assert!(
            x.position[0].abs() < f32::MAX * 0.00001 && y.position[0].abs() < f32::MAX * 0.00001
        );
    }
}
