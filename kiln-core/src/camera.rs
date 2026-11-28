//! Persisted 2D gameplay camera settings (keyed by scene ID in the project).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    /// Stable target entity source ID; None keeps a stationary camera.
    pub target: Option<String>,
    /// Stationary origin, or follow offset from the target center.
    pub position: [f32; 2],
    pub zoom: f32,
    /// Exponential follow response in inverse seconds; zero snaps immediately.
    pub smoothing: f32,
    /// Full centered follow dead-zone width/height in world units; zero centers.
    pub dead_zone: [f32; 2],
    /// World rectangle [left, bottom, right, top].
    pub bounds: Option<[f32; 4]>,
    pub pixel_perfect: bool,
    pub pixels_per_unit: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            target: None,
            position: [0.0; 2],
            zoom: 1.0,
            smoothing: 0.0,
            dead_zone: [0.0; 2],
            bounds: None,
            pixel_perfect: false,
            pixels_per_unit: 1.0,
        }
    }
}

impl CameraSettings {
    /// Reject invalid projections before applying or saving camera edits.
    pub fn validate(&self) -> Result<(), String> {
        if !self.position.iter().all(|value| value.is_finite())
            || !self.zoom.is_finite()
            || self.zoom <= 0.0
            || !self.smoothing.is_finite()
            || self.smoothing < 0.0
            || !self.pixels_per_unit.is_finite()
            || self.pixels_per_unit <= 0.0
        {
            return Err("Camera values must be finite, with positive zoom/pixel density".into());
        }
        if !self
            .dead_zone
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err("Camera dead zone dimensions must be finite and nonnegative".into());
        }
        if self.bounds.is_some_and(|bounds| {
            !bounds.iter().all(|value| value.is_finite())
                || bounds[0] >= bounds[2]
                || bounds[1] >= bounds[3]
        }) {
            return Err("Camera bounds must be finite and ordered".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod dead_zone_tests {
    use super::*;
    #[test]
    fn old_cameras_default_to_center_follow_and_new_zones_roundtrip() {
        let old: CameraSettings = toml::from_str("zoom=2.0\nsmoothing=4.0").unwrap();
        assert_eq!(old.dead_zone, [0.0; 2]);
        let settings = CameraSettings {
            dead_zone: [64.0, 32.0],
            ..old
        };
        assert_eq!(
            toml::from_str::<CameraSettings>(&toml::to_string(&settings).unwrap()).unwrap(),
            settings
        );
        for zone in [[-1.0, 0.0], [f32::NAN, 0.0], [0.0, f32::INFINITY]] {
            assert!(CameraSettings {
                dead_zone: zone,
                ..settings.clone()
            }
            .validate()
            .is_err());
        }
    }
}
