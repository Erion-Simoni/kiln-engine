//! Clipped viewport layout and pointer routing for up to four gameplay views.
use crate::camera::Camera;
/// Bounded active gameplay views; overlays use their own UI input path.
pub const MAX_VIEWS: usize = 4;
/// A physical/point viewport rectangle; use the same units as the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewRect(pub [f32; 4]);
impl ViewRect {
    /// Validate positive finite extent and representable right/bottom edges.
    pub fn validate(self) -> Result<(), String> {
        let [x, y, w, h] = self.0;
        let right = x as f64 + w as f64;
        let bottom = y as f64 + h as f64;
        if !self.0.iter().all(|v| v.is_finite())
            || w <= 0.0
            || h <= 0.0
            || right > f32::MAX as f64
            || bottom > f32::MAX as f64
            || right as f32 <= x
            || bottom as f32 <= y
        {
            return Err(
                "View rectangle needs finite origin and positive representable extent".into(),
            );
        }
        Ok(())
    }
    /// Half-open edges give shared split boundaries to exactly one adjacent view.
    pub fn contains(self, point: [f32; 2]) -> bool {
        self.validate().is_ok()
            && point.iter().all(|v| v.is_finite())
            && point[0] >= self.0[0]
            && point[1] >= self.0[1]
            && (point[0] as f64) < self.0[0] as f64 + self.0[2] as f64
            && (point[1] as f64) < self.0[1] as f64 + self.0[3] as f64
    }
}
/// Select the last (frontmost) view containing the pointer; gaps map to no view.
/// Rectangles must already be fitted to their cameras, excluding letterbox bars.
pub fn route_pointer(
    point: [f32; 2],
    views: &[(&Camera, ViewRect)],
) -> Result<Option<(usize, [f32; 2])>, String> {
    if views.len() > MAX_VIEWS {
        return Err("At most four gameplay views are supported".into());
    }
    for (_, rect) in views {
        rect.validate()?;
    }
    for (index, (camera, rect)) in views.iter().enumerate().rev() {
        if rect.contains(point) {
            return Ok(camera
                .pointer_to_world(point, rect.0)
                .map(|world| (index, world)));
        }
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_boundary_overlap_letterbox_resize_and_zoom_route_consistently() {
        let a = Camera::new(100.0, 100.0);
        let mut b = Camera::new(100.0, 100.0);
        b.position = [200.0, 40.0];
        b.zoom = 2.0;
        let views = [
            (&a, ViewRect([10.0, 20.0, 100.0, 100.0])),
            (&b, ViewRect([110.0, 20.0, 100.0, 100.0])),
        ];
        assert_eq!(
            route_pointer([110.0, 20.0], &views).unwrap(),
            Some((1, [200.0, 90.0]))
        );
        assert_eq!(
            route_pointer([60.0, 70.0], &views).unwrap(),
            Some((0, [50.0, 50.0]))
        );
        for p in [[9.0, 30.0], [210.0, 30.0], [50.0, 120.0], [f32::NAN, 0.0]] {
            assert_eq!(route_pointer(p, &views).unwrap(), None);
        }
        let overlap = [views[0], (&b, ViewRect([50.0, 20.0, 100.0, 100.0]))];
        assert_eq!(route_pointer([60.0, 70.0], &overlap).unwrap().unwrap().0, 1);
        let fitted = a.fit_viewport([0, 0, 400, 200]).map(|v| v as f32);
        assert!(a.pointer_to_world([50.0, 100.0], fitted).is_none());
        assert_eq!(
            a.pointer_to_world([200.0, 100.0], fitted),
            Some([50.0, 50.0])
        );
        let mut down = b.clone();
        down.top_left_origin = true;
        assert_eq!(
            down.pointer_to_world([160.0, 70.0], views[1].1 .0),
            Some([225.0, 65.0])
        );
    }
    #[test]
    fn invalid_layouts_and_unrepresentable_camera_mappings_reject() {
        let a = Camera::new(100.0, 100.0);
        let rect = ViewRect([0.0, 0.0, 100.0, 100.0]);
        assert!(route_pointer([0.0; 2], &[(&a, rect); 5]).is_err());
        for bad in [
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0, 1.0],
            [f32::NAN, 0.0, 1.0, 1.0],
            [f32::MAX, 0.0, f32::MAX, 1.0],
        ] {
            assert!(route_pointer([0.0; 2], &[(&a, ViewRect(bad))]).is_err());
        }
        let mut bad = a;
        bad.zoom = 0.0;
        assert_eq!(route_pointer([50.0; 2], &[(&bad, rect)]).unwrap(), None);
    }
}
