//! Shared gameplay camera follow, limits and pixel snapping.
use crate::world::GlobalTransform;
use crate::{GameWorld, SourceId};
use kiln_core::camera::CameraSettings;
use kiln_render::camera::Camera;

mod shake;
mod transition;
pub use shake::{CameraEffect, CameraShake, MAX_CAMERA_EFFECTS};
pub use transition::CameraTransition;

#[derive(Debug, Default)]
pub struct CameraController {
    position: [f32; 2],
    initialized: bool,
    shakes: shake::Mixer,
    transition: transition::Controller,
}

fn coordinate(value: f64) -> Result<f32, String> {
    if value.is_finite() && value.abs() <= f32::MAX as f64 {
        Ok(value as f32)
    } else {
        Err("Camera position overflow".into())
    }
}
fn clamp_position(position: &mut [f64; 2], extent: [f64; 2], bounds: Option<[f32; 4]>) {
    if let Some(bounds) = bounds.map(|b| b.map(f64::from)) {
        for axis in 0..2 {
            position[axis] = if extent[axis] > bounds[axis + 2] - bounds[axis] {
                (bounds[axis] + bounds[axis + 2] - extent[axis]) / 2.0
            } else {
                position[axis].clamp(bounds[axis], bounds[axis + 2] - extent[axis])
            };
        }
    }
}

fn finish_position(position: &mut [f64; 2], extent: [f64; 2], settings: &CameraSettings) {
    if settings.pixel_perfect {
        for value in position.iter_mut() {
            *value = (*value * settings.pixels_per_unit as f64).round()
                / settings.pixels_per_unit as f64;
        }
    }
    clamp_position(position, extent, settings.bounds);
}

impl CameraController {
    /// Apply validated transient effects; reset/scene replacement clears all effects.
    pub fn apply_effect(&mut self, effect: CameraEffect) -> Result<(), String> {
        match effect {
            CameraEffect::Shake(request) => self.shakes.add(request),
            CameraEffect::Transition(request) => self.transition.start(request),
            CameraEffect::StopTransition => {
                self.transition.stop();
                Ok(())
            }
            CameraEffect::StopShake => {
                self.shakes.clear();
                Ok(())
            }
        }
    }

    /// Update a camera in world units using frame-rate independent smoothing.
    pub fn update(
        &mut self,
        settings: &CameraSettings,
        world: &GameWorld,
        size: [f32; 2],
        dt: f32,
    ) -> Result<Camera, String> {
        settings.validate()?;
        if !size.iter().all(|value| value.is_finite() && *value > 0.0)
            || !dt.is_finite()
            || dt < 0.0
        {
            return Err("Camera viewport/time must be valid".into());
        }
        let mut camera = Camera::new(size[0], size[1]);
        camera.pixel_perfect = settings.pixel_perfect;
        camera.zoom = if settings.pixel_perfect {
            settings.zoom.round().max(1.0)
        } else {
            settings.zoom
        };
        let extent = [
            size[0] as f64 / camera.zoom as f64,
            size[1] as f64 / camera.zoom as f64,
        ];
        if !extent.iter().all(|value| value.is_finite()) {
            return Err("Camera extent overflow".into());
        }
        let target = settings.target.as_ref().and_then(|id| {
            world
                .world_ref()
                .query::<(&SourceId, &GlobalTransform)>()
                .iter()
                .find(|(source, _)| &source.0 == id)
                .map(|(_, pose)| pose.position)
        });
        if target.is_some_and(|point| !point.is_finite()) {
            return Err("Camera target position must be finite".into());
        }
        let mut desired = target
            .map(|point| {
                [
                    point.x as f64 - extent[0] / 2.0 + settings.position[0] as f64,
                    point.y as f64 - extent[1] / 2.0 + settings.position[1] as f64,
                ]
            })
            .unwrap_or(if settings.target.is_some() && self.initialized {
                self.position.map(f64::from)
            } else {
                settings.position.map(f64::from)
            });
        if self.initialized && target.is_some() {
            for (axis, goal) in desired.iter_mut().enumerate() {
                let half = (settings.dead_zone[axis] as f64).min(extent[axis]) * 0.5;
                if half > 0.0 {
                    let origin = self.position[axis] as f64;
                    let delta = *goal - origin;
                    *goal = origin + delta.signum() * (delta.abs() - half).max(0.0);
                }
            }
        }
        let alpha = if !self.initialized || settings.smoothing == 0.0 {
            1.0
        } else {
            1.0 - (-(settings.smoothing as f64) * dt as f64).exp()
        };
        let mut candidate = self.position.map(f64::from);
        for (position, desired) in candidate.iter_mut().zip(desired) {
            *position += (desired - *position) * alpha;
        }
        clamp_position(&mut candidate, extent, settings.bounds);
        let base = [coordinate(candidate[0])?, coordinate(candidate[1])?];
        let mut transition = self.transition.clone();
        let view = transition.advance(
            transition::View {
                position: base,
                zoom: camera.zoom,
            },
            dt as f64,
        );
        camera.zoom = if settings.pixel_perfect {
            view.zoom.round().max(1.0)
        } else {
            view.zoom
        };
        let extent = [
            size[0] as f64 / camera.zoom as f64,
            size[1] as f64 / camera.zoom as f64,
        ];
        candidate = view.position.map(f64::from);
        let mut unshaken = candidate;
        finish_position(&mut unshaken, extent, settings);
        transition.record(transition::View {
            position: [coordinate(unshaken[0])?, coordinate(unshaken[1])?],
            zoom: camera.zoom,
        });
        let mut shakes = self.shakes.clone();
        let offset = shakes.advance(dt as f64);
        for axis in 0..2 {
            candidate[axis] += offset[axis];
        }
        finish_position(&mut candidate, extent, settings);
        camera.position = [coordinate(candidate[0])?, coordinate(candidate[1])?];
        let bounds = camera.view_bounds();
        if !bounds.iter().all(|value| value.is_finite())
            || bounds[0] >= bounds[2]
            || bounds[1] >= bounds[3]
            || !camera.projection_matrix().is_finite()
        {
            return Err("Camera projection is not representable at this position/extent".into());
        }
        self.position = base;
        self.shakes = shakes;
        self.transition = transition;
        self.initialized = true;
        Ok(camera)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::Transform;

    #[test]
    fn pixel_grid_missing_target_and_invalid_settings_are_handled() {
        let mut controller = CameraController::default();
        let mut world = GameWorld::new();
        let entity = world.world_mut().spawn((
            SourceId::new("hero"),
            GlobalTransform(Transform::new(17.3, 21.7)),
        ));
        let mut settings = CameraSettings {
            target: Some("hero".into()),
            zoom: 1.8,
            pixel_perfect: true,
            pixels_per_unit: 2.0,
            ..Default::default()
        };
        let camera = controller
            .update(&settings, &world, [20.0, 10.0], 0.0)
            .unwrap();
        assert_eq!(camera.zoom, 2.0);
        assert_eq!(camera.position, [12.5, 19.0]);
        world.world_mut().despawn(entity).unwrap();
        assert_eq!(
            controller
                .update(&settings, &world, [20.0, 10.0], 1.0)
                .unwrap()
                .position,
            camera.position
        );
        settings.bounds = Some([0.1, 0.1, 14.3, 10.3]);
        let bounded = controller
            .update(&settings, &world, [20.0, 10.0], 1.0)
            .unwrap();
        assert!(bounded.position[0] <= 4.3 && bounded.position[1] <= 5.3);
        settings.zoom = f32::NAN;
        assert!(controller
            .update(&settings, &world, [20.0, 10.0], 1.0)
            .is_err());
        settings.zoom = 1.0;
        assert!(controller
            .update(&settings, &world, [0.0, 10.0], 1.0)
            .is_err());
    }
    #[test]
    fn smoothing_is_frame_rate_independent_and_bounds_handle_small_levels() {
        let mut world = GameWorld::new();
        let entity = world.world_mut().spawn((
            SourceId::new("hero"),
            Transform::new(10.0, 10.0),
            GlobalTransform(Transform::new(10.0, 10.0)),
        ));
        let settings = CameraSettings {
            target: Some("hero".into()),
            smoothing: 4.0,
            ..Default::default()
        };
        let mut a = CameraController::default();
        let mut b = CameraController::default();
        a.update(&settings, &world, [10.0, 10.0], 0.0).unwrap();
        b.update(&settings, &world, [10.0, 10.0], 0.0).unwrap();
        world
            .world_mut()
            .get::<&mut GlobalTransform>(entity)
            .unwrap()
            .0
            .position
            .x = 30.0;
        let mut ca = Camera::new(10.0, 10.0);
        let mut cb = ca.clone();
        for _ in 0..30 {
            ca = a
                .update(&settings, &world, [10.0, 10.0], 1.0 / 30.0)
                .unwrap();
        }
        for _ in 0..120 {
            cb = b
                .update(&settings, &world, [10.0, 10.0], 1.0 / 120.0)
                .unwrap();
        }
        assert!((ca.position[0] - cb.position[0]).abs() < 0.0001);
        let bounded = CameraSettings {
            bounds: Some([0.0, 0.0, 4.0, 4.0]),
            ..settings
        };
        assert_eq!(
            a.update(&bounded, &world, [10.0, 10.0], 1.0)
                .unwrap()
                .position,
            [-3.0, -3.0]
        );
        let screen = ca
            .world_to_screen([13.0, 9.0], [30.0, 40.0, 600.0, 400.0])
            .unwrap();
        let back = ca
            .screen_to_world(screen, [30.0, 40.0, 600.0, 400.0])
            .unwrap();
        assert!((back[0] - 13.0).abs() < 0.0001 && (back[1] - 9.0).abs() < 0.0001);
    }
}

#[cfg(test)]
mod dead_zone_tests {
    use super::*;
    use kiln_core::Transform;
    #[test]
    fn dead_zone_holds_inside_moves_minimally_outside_and_survives_bad_updates() {
        let mut world = GameWorld::new();
        let entity = world.world_mut().spawn((
            SourceId::new("hero"),
            GlobalTransform(Transform::new(50.0, 40.0)),
        ));
        let settings = CameraSettings {
            target: Some("hero".into()),
            dead_zone: [20.0, 10.0],
            ..Default::default()
        };
        let mut controller = CameraController::default();
        assert_eq!(
            controller
                .update(&settings, &world, [100.0, 80.0], 0.0)
                .unwrap()
                .position,
            [0.0, 0.0]
        );
        let set = |world: &mut GameWorld, x, y| {
            world
                .world_mut()
                .get::<&mut GlobalTransform>(entity)
                .unwrap()
                .0
                .position = kiln_core::Vec2::new(x, y);
        };
        set(&mut world, 59.0, 44.0);
        assert_eq!(
            controller
                .update(&settings, &world, [100.0, 80.0], 0.016)
                .unwrap()
                .position,
            [0.0, 0.0]
        );
        set(&mut world, 75.0, 20.0);
        assert_eq!(
            controller
                .update(&settings, &world, [100.0, 80.0], 0.016)
                .unwrap()
                .position,
            [15.0, -15.0]
        );
        set(&mut world, f32::NAN, 20.0);
        assert!(controller
            .update(&settings, &world, [100.0, 80.0], 0.016)
            .is_err());
        set(&mut world, 75.0, 20.0);
        assert_eq!(
            controller
                .update(&settings, &world, [100.0, 80.0], 0.0)
                .unwrap()
                .position,
            [15.0, -15.0]
        );
        let bad = CameraSettings {
            target: None,
            position: [f32::MAX, f32::MAX],
            ..settings.clone()
        };
        assert!(controller.update(&bad, &world, [100.0, 80.0], 0.1).is_err());
        assert_eq!(
            controller
                .update(&settings, &world, [100.0, 80.0], 0.0)
                .unwrap()
                .position,
            [15.0, -15.0]
        );
        set(&mut world, 200.3, 200.7);
        let bounded = CameraSettings {
            bounds: Some([0.1, 0.1, 120.3, 100.3]),
            pixel_perfect: true,
            pixels_per_unit: 2.0,
            ..settings
        };
        let camera = controller
            .update(&bounded, &world, [100.0, 80.0], 0.016)
            .unwrap();
        for axis in 0..2 {
            assert!((camera.position[axis] - 20.3).abs() < 0.0001);
        }
    }
    #[test]
    fn camera_extreme_valid_values_remain_finite_and_failed_first_update_can_retry() {
        let world = GameWorld::new();
        let mut controller = CameraController::default();
        let bad = CameraSettings {
            position: [f32::MAX, f32::MAX],
            ..Default::default()
        };
        assert!(controller.update(&bad, &world, [10.0, 10.0], 0.0).is_err());
        let large = CameraSettings {
            position: [2e38, 2e38],
            ..Default::default()
        };
        let camera = controller
            .update(&large, &world, [1e37, 1e37], 0.0)
            .unwrap();
        assert!(camera.projection_matrix().is_finite());
        assert_eq!(
            controller
                .update(&CameraSettings::default(), &world, [100.0, 80.0], 0.0)
                .unwrap()
                .position,
            [0.0, 0.0]
        );
    }
    #[test]
    fn dead_zone_caps_to_zoomed_extent_and_smoothing_remains_rate_independent() {
        let mut world = GameWorld::new();
        let entity = world.world_mut().spawn((
            SourceId::new("hero"),
            GlobalTransform(Transform::new(50.0, 40.0)),
        ));
        let mut settings = CameraSettings {
            target: Some("hero".into()),
            dead_zone: [20.0, 10.0],
            smoothing: 4.0,
            ..Default::default()
        };
        let mut a = CameraController::default();
        let mut b = CameraController::default();
        a.update(&settings, &world, [100.0, 80.0], 0.0).unwrap();
        b.update(&settings, &world, [100.0, 80.0], 0.0).unwrap();
        world
            .world_mut()
            .get::<&mut GlobalTransform>(entity)
            .unwrap()
            .0
            .position
            .x = 100.0;
        for _ in 0..30 {
            a.update(&settings, &world, [100.0, 80.0], 1.0 / 30.0)
                .unwrap();
        }
        for _ in 0..120 {
            b.update(&settings, &world, [100.0, 80.0], 1.0 / 120.0)
                .unwrap();
        }
        assert!((a.position[0] - b.position[0]).abs() < 0.0001);
        settings.smoothing = 0.0;
        settings.zoom = 2.0;
        settings.dead_zone = [1000.0; 2];
        let camera = a.update(&settings, &world, [20.0, 20.0], 0.016).unwrap();
        assert!(camera.position[0] >= 90.0 && camera.position[0] <= 100.0);
        let point = camera
            .world_to_screen([100.0, 40.0], [30.0, 40.0, 400.0, 400.0])
            .unwrap();
        let back = camera
            .screen_to_world(point, [30.0, 40.0, 400.0, 400.0])
            .unwrap();
        assert!((back[0] - 100.0).abs() < 0.001 && (back[1] - 40.0).abs() < 0.001);
    }
}

#[cfg(test)]
mod shake_controller_tests {
    use super::*;
    #[test]
    fn shake_does_not_feed_follow_origin_and_failed_updates_do_not_advance_time() {
        let mut world = GameWorld::new();
        world.spawn((
            SourceId::new("hero"),
            GlobalTransform(kiln_core::Transform::new(160.0, 120.0)),
        ));
        let settings = CameraSettings {
            target: Some("hero".into()),
            dead_zone: [64.0, 32.0],
            ..Default::default()
        };
        let mut a = CameraController::default();
        let mut b = CameraController::default();
        for c in [&mut a, &mut b] {
            c.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
            c.apply_effect(CameraEffect::Shake(CameraShake {
                amplitude: [12.0, 8.0],
                duration: 1.0,
                frequency: 7.0,
                seed: 17,
            }))
            .unwrap();
            c.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
            assert_eq!(c.position, [0.0, 0.0]);
        }
        let invalid = CameraSettings {
            target: None,
            position: [f32::MAX; 2],
            ..settings.clone()
        };
        assert!(a.update(&invalid, &world, [320.0, 240.0], 0.5).is_err());
        let current = a.update(&settings, &world, [320.0, 240.0], 0.1).unwrap();
        assert_eq!(
            current.position,
            b.update(&settings, &world, [320.0, 240.0], 0.1)
                .unwrap()
                .position
        );
        assert_eq!(
            current.position,
            a.update(&settings, &world, [320.0, 240.0], 0.0)
                .unwrap()
                .position
        );
        assert_eq!(
            a.update(&settings, &world, [320.0, 240.0], 2.0)
                .unwrap()
                .position,
            [0.0, 0.0]
        );
        assert_eq!(a.position, [0.0, 0.0]);
    }
}

#[cfg(test)]
mod transition_controller_tests {
    use super::*;
    #[test]
    fn follow_stays_independent_and_bad_projection_does_not_advance_transition() {
        let mut world = GameWorld::new();
        world.spawn((
            SourceId::new("hero"),
            GlobalTransform(kiln_core::Transform::new(160.0, 120.0)),
        ));
        let settings = CameraSettings {
            target: Some("hero".into()),
            ..Default::default()
        };
        let mut a = CameraController::default();
        a.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
        a.apply_effect(CameraEffect::Transition(CameraTransition {
            position: [80.0, 40.0],
            zoom: 2.0,
            duration: 1.0,
        }))
        .unwrap();
        a.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
        assert!(a
            .update(&settings, &world, [f32::MIN_POSITIVE, 240.0], 0.5)
            .is_err());
        let half = a.update(&settings, &world, [320.0, 240.0], 0.5).unwrap();
        assert_eq!(half.position, [40.0, 20.0]);
        assert_eq!(half.zoom, 1.5);
        for pose in world.world_mut().query::<&mut GlobalTransform>().iter() {
            pose.0.position.x = 200.0;
        }
        let end = a.update(&settings, &world, [320.0, 240.0], 0.5).unwrap();
        assert_eq!(end.position, [80.0, 40.0]);
        assert_eq!(end.zoom, 2.0);
        a.apply_effect(CameraEffect::StopTransition).unwrap();
        let restored = a.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
        assert_eq!(restored.position, [40.0, 0.0]);
        assert_eq!(restored.zoom, 1.0);
    }
    #[test]
    fn transitioned_zoom_drives_bounds_snapping_and_shake_is_independent() {
        let world = GameWorld::new();
        let mut c = CameraController::default();
        let settings = CameraSettings {
            pixel_perfect: true,
            bounds: Some([0.0, 0.0, 320.0, 240.0]),
            ..Default::default()
        };
        c.apply_effect(CameraEffect::Transition(CameraTransition {
            position: [999.0, 999.0],
            zoom: 2.4,
            duration: 0.0,
        }))
        .unwrap();
        let end = c.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
        assert_eq!(end.position, [160.0, 120.0]);
        assert_eq!(end.zoom, 2.0);
        c.apply_effect(CameraEffect::Shake(CameraShake {
            amplitude: [8.0, 8.0],
            duration: 1.0,
            frequency: 8.0,
            seed: 17,
        }))
        .unwrap();
        let _ = c.update(&settings, &world, [320.0, 240.0], 0.0).unwrap();
        c.apply_effect(CameraEffect::Transition(CameraTransition {
            position: [0.0; 2],
            zoom: 1.0,
            duration: 1.0,
        }))
        .unwrap();
        let from = c
            .update(&CameraSettings::default(), &world, [320.0, 240.0], 0.0)
            .unwrap();
        c.apply_effect(CameraEffect::StopShake).unwrap();
        let unshaken = c
            .update(&CameraSettings::default(), &world, [320.0, 240.0], 0.0)
            .unwrap();
        assert_eq!(unshaken.position, [160.0, 120.0]);
        assert_eq!(unshaken.zoom, 2.0);
        assert_ne!(from.position, unshaken.position);
    }
}
