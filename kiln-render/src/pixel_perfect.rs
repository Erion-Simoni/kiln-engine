//! Pixel-perfect rendering utilities.
//!
//! Provides utilities for pixel-perfect rendering, including snapping
//! sprite positions to the pixel grid and calculating optimal camera
//! positions for pixel-perfect display.

/// Snap a world position to the nearest pixel given the camera zoom.
///
/// This ensures sprites are drawn at integer pixel coordinates,
/// preventing sub-pixel blurring for pixel-art games.
pub fn snap_to_pixel(position: [f32; 2], zoom: f32) -> [f32; 2] {
    if zoom <= 0.0 {
        return position;
    }
    let inv_zoom = 1.0 / zoom;
    [
        (position[0] * inv_zoom).round() * zoom,
        (position[1] * inv_zoom).round() * zoom,
    ]
}

/// Calculate the optimal camera position for pixel-perfect rendering.
///
/// Given a desired world position and camera zoom, returns the camera
/// position that aligns the world origin to the nearest pixel boundary,
/// ensuring sprites are drawn at integer pixel coordinates.
pub fn pixel_perfect_camera_position(
    target: [f32; 2],
    zoom: f32,
    viewport_size: [f32; 2],
) -> [f32; 2] {
    let inv_zoom = 1.0 / zoom;
    let view_width = viewport_size[0] * inv_zoom;
    let view_height = viewport_size[1] * inv_zoom;

    // Center the target on screen, then snap to pixel grid
    let cam_x = target[0] - view_width / 2.0;
    let cam_y = target[1] - view_height / 2.0;

    [
        (cam_x * inv_zoom).round() * zoom,
        (cam_y * inv_zoom).round() * zoom,
    ]
}

/// Calculate the optimal zoom level for pixel-perfect rendering
/// given a target pixel size for game units.
pub fn pixel_perfect_zoom(
    world_units_per_pixel: f32,
    current_zoom: f32,
) -> f32 {
    if world_units_per_pixel <= 0.0 {
        return current_zoom;
    }
    let ideal_zoom = 1.0 / world_units_per_pixel;
    // Snap to nearest reasonable zoom level
    let zoomed = (ideal_zoom * 10.0).round() / 10.0;
    zoomed.clamp(0.1, 10.0)
}
