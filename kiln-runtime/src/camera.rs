//! Shared gameplay camera follow, limits and pixel snapping.
use crate::world::GlobalTransform;
use crate::{GameWorld, SourceId};
use kiln_core::camera::CameraSettings;
use kiln_render::camera::Camera;

#[derive(Debug, Default)]
pub struct CameraController {
    position: [f32; 2],
    initialized: bool,
}

impl CameraController {
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
        let extent = [size[0] / camera.zoom, size[1] / camera.zoom];
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
        let desired = target
            .map(|target| {
                [
                    target.x - extent[0] / 2.0 + settings.position[0],
                    target.y - extent[1] / 2.0 + settings.position[1],
                ]
            })
            .unwrap_or(if settings.target.is_some() && self.initialized {
                self.position
            } else {
                settings.position
            });
        let alpha = if !self.initialized || settings.smoothing == 0.0 {
            1.0
        } else {
            1.0 - (-settings.smoothing * dt).exp()
        };
        for (position, desired) in self.position.iter_mut().zip(desired) {
            *position += (desired - *position) * alpha;
        }
        if let Some(bounds) = settings.bounds {
            for axis in 0..2 {
                self.position[axis] = if extent[axis] > bounds[axis + 2] - bounds[axis] {
                    (bounds[axis] + bounds[axis + 2] - extent[axis]) / 2.0
                } else {
                    self.position[axis].clamp(bounds[axis], bounds[axis + 2] - extent[axis])
                };
            }
        }
        self.initialized = true;
        camera.position = self.position;
        if settings.pixel_perfect {
            for value in &mut camera.position {
                *value = (*value * settings.pixels_per_unit).round() / settings.pixels_per_unit;
            }
        }
        if let Some(bounds) = settings.bounds {
            for axis in 0..2 {
                camera.position[axis] = if extent[axis] > bounds[axis + 2] - bounds[axis] {
                    (bounds[axis] + bounds[axis + 2] - extent[axis]) / 2.0
                } else {
                    camera.position[axis].clamp(bounds[axis], bounds[axis + 2] - extent[axis])
                };
            }
        }
        if !camera.position.iter().all(|value| value.is_finite()) {
            return Err("Camera position overflow".into());
        }
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
