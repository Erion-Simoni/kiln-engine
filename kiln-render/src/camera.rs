//! Orthographic camera for 2D rendering

use glam::Mat4;

pub struct Camera {
    pub position: [f32; 2],
    pub zoom: f32,
    pub viewport_size: [f32; 2],
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            viewport_size: [width, height],
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        let left = self.position[0];
        let right = self.position[0] + self.viewport_size[0] / self.zoom;
        let bottom = self.position[1];
        let top = self.position[1] + self.viewport_size[1] / self.zoom;

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
