//! Orthographic camera for 2D rendering

use glam::Mat4;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: [f32; 2],
    pub zoom: f32,
    pub viewport_size: [f32; 2],
    /// Fit the reference viewport at integer physical scales when space permits.
    pub pixel_perfect: bool,
    /// Whether the camera treats the projection origin as top-left.
    ///
    /// When false, world Y grows upward, matching tilemaps and gameplay. When
    /// true, world Y grows downward from the viewport top, pairing with the
    /// y-down UI renderer.
    pub top_left_origin: bool,
}

impl Camera {
    /// Center the reference aspect inside a physical [x,y,width,height] area.
    /// Small windows downscale to fit; pixel-perfect mode uses whole-number
    /// upscales so resizing cannot silently introduce fractional pixel scaling.
    pub fn fit_viewport(&self, area: [u32; 4]) -> [u32; 4] {
        if area[2] == 0
            || area[3] == 0
            || !self
                .viewport_size
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        {
            return [area[0], area[1], 0, 0];
        }
        let scale =
            (area[2] as f32 / self.viewport_size[0]).min(area[3] as f32 / self.viewport_size[1]);
        let scale = if self.pixel_perfect && scale >= 1.0 {
            scale.floor()
        } else {
            scale
        };
        let width = ((self.viewport_size[0] * scale).round() as u32).clamp(1, area[2]);
        let height = ((self.viewport_size[1] * scale).round() as u32).clamp(1, area[3]);
        [
            area[0] + (area[2] - width) / 2,
            area[1] + (area[3] - height) / 2,
            width,
            height,
        ]
    }
    /// Map a pixel/point coordinate through a viewport [x,y,width,height] into
    /// world space. Both coordinates must use the same pixel/point unit.
    pub fn screen_to_world(&self, point: [f32; 2], viewport: [f32; 4]) -> Option<[f32; 2]> {
        if viewport[2] <= 0.0
            || viewport[3] <= 0.0
            || self.zoom <= 0.0
            || !self.zoom.is_finite()
            || !self
                .viewport_size
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            || !self.position.iter().all(|value| value.is_finite())
            || !point
                .iter()
                .chain(viewport.iter())
                .all(|value| value.is_finite())
        {
            return None;
        }
        let x = (point[0] - viewport[0]) / viewport[2];
        let y = (point[1] - viewport[1]) / viewport[3];
        Some([
            self.position[0] + x * self.viewport_size[0] / self.zoom,
            self.position[1]
                + (if self.top_left_origin { y } else { 1.0 - y }) * self.viewport_size[1]
                    / self.zoom,
        ])
    }

    /// Inverse of screen_to_world, including viewport offsets and Y direction.
    pub fn world_to_screen(&self, point: [f32; 2], viewport: [f32; 4]) -> Option<[f32; 2]> {
        if self.viewport_size[0] <= 0.0
            || self.viewport_size[1] <= 0.0
            || viewport[2] <= 0.0
            || viewport[3] <= 0.0
            || !self.zoom.is_finite()
            || !self.viewport_size.iter().all(|value| value.is_finite())
            || !self.position.iter().all(|value| value.is_finite())
            || self.zoom <= 0.0
            || !point
                .iter()
                .chain(viewport.iter())
                .all(|value| value.is_finite())
        {
            return None;
        }
        let x = (point[0] - self.position[0]) * self.zoom / self.viewport_size[0];
        let y = (point[1] - self.position[1]) * self.zoom / self.viewport_size[1];
        Some([
            viewport[0] + x * viewport[2],
            viewport[1] + (if self.top_left_origin { y } else { 1.0 - y }) * viewport[3],
        ])
    }
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            viewport_size: [width, height],
            pixel_perfect: false,
            top_left_origin: false,
        }
    }

    /// Top-left-origin camera for y-down UI overlays.
    pub fn new_top_left(width: f32, height: f32) -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            viewport_size: [width, height],
            pixel_perfect: false,
            top_left_origin: true,
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        let left = self.position[0];
        let right = self.position[0] + self.viewport_size[0] / self.zoom;
        let bottom = self.position[1];
        let top = self.position[1] + self.viewport_size[1] / self.zoom;
        let (bottom, top) = if self.top_left_origin {
            (top, bottom)
        } else {
            (bottom, top)
        };

        // Manual orthographic matrix construction
        let tx = -(right + left) / (right - left);
        let ty = -(top + bottom) / (top - bottom);

        Mat4::from_cols_array(&[
            2.0 / (right - left),
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / (top - bottom),
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            tx,
            ty,
            0.0,
            1.0,
        ])
    }

    /// World-space rectangle of the camera view: `[min_x, min_y, max_x, max_y]`.
    pub fn view_bounds(&self) -> [f32; 4] {
        let w = self.viewport_size[0] / self.zoom;
        let h = self.viewport_size[1] / self.zoom;
        [
            self.position[0],
            self.position[1],
            self.position[0] + w,
            self.position[1] + h,
        ]
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.position[0] += dx;
        self.position[1] += dy;
    }

    /// Clamp zoom to the valid [0.1, 10.0] range.
    pub fn zoom(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_viewport_preserves_aspect_and_integer_upscaling() {
        let mut camera = Camera::new(320.0, 180.0);
        assert_eq!(
            camera.fit_viewport([20, 30, 1000, 700]),
            [20, 98, 1000, 563]
        );
        camera.pixel_perfect = true;
        assert_eq!(
            camera.fit_viewport([20, 30, 1000, 700]),
            [40, 110, 960, 540]
        );
        assert_eq!(camera.fit_viewport([0, 0, 160, 100]), [0, 5, 160, 90]);
        assert_eq!(camera.fit_viewport([0; 4]), [0; 4]);
    }

    #[test]
    fn input_mapping_matches_projection_with_letterbox_zoom_and_both_y_origins() {
        for top_left_origin in [false, true] {
            let mut camera = Camera::new(640.0, 480.0);
            camera.position = [12.0, -30.0];
            camera.zoom = 2.0;
            camera.top_left_origin = top_left_origin;
            let viewport = [80.0, 100.0, 960.0, 720.0];
            let point = [120.0, 90.0];
            let screen = camera.world_to_screen(point, viewport).unwrap();
            let back = camera.screen_to_world(screen, viewport).unwrap();
            assert!((back[0] - point[0]).abs() < 0.0001 && (back[1] - point[1]).abs() < 0.0001);
            let ndc = camera.projection_matrix() * glam::Vec4::new(point[0], point[1], 0.0, 1.0);
            assert!((screen[0] - (viewport[0] + (ndc.x + 1.0) * 0.5 * viewport[2])).abs() < 0.001);
            assert!((screen[1] - (viewport[1] + (1.0 - ndc.y) * 0.5 * viewport[3])).abs() < 0.001);
            assert!(camera.screen_to_world(point, [0.0; 4]).is_none());
            assert!(camera.world_to_screen([f32::NAN, 0.0], viewport).is_none());
        }
    }

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The world origin (0, 0) should map to NDC (−1, −1) for a camera
    /// positioned at (0, 0) with a 200 × 100 viewport.
    #[test]
    fn origin_maps_to_bottom_left_ndc() {
        let cam = Camera::new(200.0, 100.0);
        let proj = cam.projection_matrix();
        let ndc = proj * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!(approx_eq(ndc.x, -1.0), "ndc.x = {}", ndc.x);
        assert!(approx_eq(ndc.y, -1.0), "ndc.y = {}", ndc.y);
    }

    /// The viewport centre should map to NDC origin.
    #[test]
    fn centre_maps_to_ndc_origin() {
        let cam = Camera::new(200.0, 100.0);
        let proj = cam.projection_matrix();
        let ndc = proj * glam::Vec4::new(100.0, 50.0, 0.0, 1.0);
        assert!(approx_eq(ndc.x, 0.0), "ndc.x = {}", ndc.x);
        assert!(approx_eq(ndc.y, 0.0), "ndc.y = {}", ndc.y);
    }

    /// The visible world rectangle grows as the camera zooms out.
    #[test]
    fn view_bounds_follow_zoom_and_pan() {
        let mut cam = Camera::new(200.0, 100.0);
        assert_eq!(cam.view_bounds(), [0.0, 0.0, 200.0, 100.0]);
        cam.pan(10.0, 20.0);
        assert_eq!(cam.view_bounds(), [10.0, 20.0, 210.0, 120.0]);
        cam.zoom(2.0);
        assert_eq!(cam.view_bounds(), [10.0, 20.0, 110.0, 70.0]);
    }

    #[test]
    fn zoom_clamps_to_bounds() {
        let mut cam = Camera::new(800.0, 600.0);
        for _ in 0..100 {
            cam.zoom(10.0);
        }
        assert!(approx_eq(cam.zoom, 10.0));
        for _ in 0..100 {
            cam.zoom(0.01);
        }
        assert!(approx_eq(cam.zoom, 0.1));
    }
}
