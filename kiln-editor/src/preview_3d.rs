//! Bounded editor-only perspective preview. This is not the game's 3D renderer.

use egui::{Color32, Pos2, Rect, Stroke};
use glam::Vec3;

const AMBER: Color32 = Color32::from_rgb(240, 160, 48);

struct Object {
    name: &'static str,
    position: Vec3,
    scale: Vec3,
    rotation: f32,
    color: Color32,
    sphere: bool,
}

/// Interactive demonstration state, deliberately independent of Project/ECS.
pub struct Preview3d {
    yaw: f32,
    pitch: f32,
    distance: f32,
    selected: usize,
    light: f32,
    roughness: f32,
    objects: Vec<Object>,
}

impl Default for Preview3d {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.48,
            distance: 11.0,
            selected: 1,
            light: 0.85,
            roughness: 0.65,
            objects: vec![
                Object {
                    name: "Ground_Plane",
                    position: Vec3::new(0.0, -0.18, 0.0),
                    scale: Vec3::new(7.0, 0.3, 4.0),
                    rotation: 0.0,
                    color: Color32::from_rgb(72, 123, 85),
                    sphere: false,
                },
                Object {
                    name: "Player_Mesh",
                    position: Vec3::new(-1.7, 0.8, 0.0),
                    scale: Vec3::new(0.7, 1.6, 0.7),
                    rotation: 0.0,
                    color: Color32::from_rgb(74, 144, 217),
                    sphere: false,
                },
                Object {
                    name: "Coin_01",
                    position: Vec3::new(0.0, 1.0, 0.0),
                    scale: Vec3::splat(0.45),
                    rotation: 0.0,
                    color: Color32::from_rgb(225, 183, 57),
                    sphere: true,
                },
                Object {
                    name: "Coin_02",
                    position: Vec3::new(1.0, 1.5, 0.0),
                    scale: Vec3::splat(0.45),
                    rotation: 0.0,
                    color: Color32::from_rgb(225, 183, 57),
                    sphere: true,
                },
                Object {
                    name: "Rock_Mesh",
                    position: Vec3::new(2.3, 0.5, -0.1),
                    scale: Vec3::new(1.3, 1.0, 1.1),
                    rotation: 24.0,
                    color: Color32::from_rgb(117, 118, 134),
                    sphere: true,
                },
            ],
        }
    }
}

#[derive(Clone)]
struct Face {
    points: [Vec3; 3],
    normal: Vec3,
    color: Color32,
    object: usize,
}

struct Projection {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    rect: Rect,
}

impl Projection {
    fn project(&self, point: Vec3) -> Option<Pos2> {
        let relative = point - self.eye;
        let depth = relative.dot(self.forward);
        if depth < 0.15 {
            return None;
        }
        let focal = self.rect.height() * 1.15;
        Some(
            self.rect.center()
                + egui::vec2(relative.dot(self.right), -relative.dot(self.up)) * (focal / depth),
        )
    }
}

impl Preview3d {
    fn projection(&self, rect: Rect) -> Projection {
        let target = Vec3::new(0.0, 0.8, 0.0);
        let eye = target
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            ) * self.distance;
        let forward = (target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        Projection {
            eye,
            right,
            up: right.cross(forward),
            forward,
            rect,
        }
    }

    /// Preview hierarchy; selecting these entries never changes 2D selection.
    pub fn hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.weak("Preview scene · session only");
        ui.separator();
        for (index, name) in self
            .objects
            .iter()
            .map(|o| o.name)
            .chain(["Sun_Light"])
            .enumerate()
        {
            let label = if index == self.objects.len() {
                format!("Light  {name}")
            } else {
                name.to_owned()
            };
            if ui
                .add_sized(
                    [ui.available_width(), 22.0],
                    egui::Button::new(label)
                        .selected(self.selected == index)
                        .frame(false),
                )
                .clicked()
            {
                self.selected = index;
            }
        }
        ui.add_space(12.0);
        ui.weak("These objects demonstrate the future 3D workspace. Your 2D scene is retained.");
    }

    /// Temporary mesh/material/light properties that immediately affect preview.
    pub fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.colored_label(AMBER, "3D VISUAL PREVIEW");
        ui.weak("Session only · not saved or exported");
        ui.separator();
        if let Some(object) = self.objects.get_mut(self.selected) {
            ui.strong(object.name);
            ui.add_space(8.0);
            ui.collapsing("Transform", |ui| {
                egui::Grid::new("preview-transform")
                    .num_columns(4)
                    .spacing([5.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Position");
                        for (label, value) in [
                            ("X ", &mut object.position.x),
                            ("Y ", &mut object.position.y),
                            ("Z ", &mut object.position.z),
                        ] {
                            ui.add(
                                egui::DragValue::new(value)
                                    .speed(0.05)
                                    .range(-5.0..=5.0)
                                    .prefix(label),
                            );
                        }
                        ui.end_row();
                        ui.label("Scale");
                        for value in [
                            &mut object.scale.x,
                            &mut object.scale.y,
                            &mut object.scale.z,
                        ] {
                            ui.add(egui::DragValue::new(value).speed(0.02).range(0.1..=8.0));
                        }
                        ui.end_row();
                    });
                ui.add(egui::Slider::new(&mut object.rotation, -180.0..=180.0).text("Y rotation"));
            });
            ui.separator();
            ui.strong("Mesh Renderer");
            ui.label(if object.sphere {
                "Faceted sphere"
            } else {
                "Cube primitive"
            });
            ui.separator();
            ui.strong("Material");
            ui.horizontal(|ui| {
                ui.label("Base color");
                ui.color_edit_button_srgba(&mut object.color);
            });
            ui.add(egui::Slider::new(&mut self.roughness, 0.0..=1.0).text("Roughness"));
        } else {
            ui.strong("Sun_Light");
            ui.label("Directional · preview shading");
        }
        ui.separator();
        ui.strong("Lighting");
        ui.add(egui::Slider::new(&mut self.light, 0.0..=1.5).text("Sun intensity"));
        ui.add_space(12.0);
        if ui.button("Reset preview scene").clicked() {
            *self = Self::default();
        }
        ui.weak("Mesh import, 3D gameplay and 3D saving are not implemented.");
    }

    fn faces(&self) -> Vec<Face> {
        let mut faces = Vec::new();
        for (index, object) in self.objects.iter().enumerate() {
            let transform = |vertex: Vec3| {
                let v = vertex * object.scale;
                let (sin, cos) = object.rotation.to_radians().sin_cos();
                Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos) + object.position
            };
            let mut triangle = |vertices: [Vec3; 3]| {
                let points = vertices.map(transform);
                let normal = (points[1] - points[0])
                    .cross(points[2] - points[0])
                    .normalize_or_zero();
                if normal.length_squared() > 0.5 {
                    faces.push(Face {
                        points,
                        normal,
                        color: object.color,
                        object: index,
                    });
                }
            };
            if object.sphere {
                const LAT: usize = 8;
                const LON: usize = 12;
                let point = |lat: usize, lon: usize| {
                    let a = lat as f32 / LAT as f32 * std::f32::consts::PI;
                    let b = lon as f32 / LON as f32 * std::f32::consts::TAU;
                    Vec3::new(a.sin() * b.cos(), a.cos(), a.sin() * b.sin()) * 0.5
                };
                for lat in 0..LAT {
                    for lon in 0..LON {
                        let [a, b, c, d] = [
                            point(lat, lon),
                            point(lat + 1, lon),
                            point(lat + 1, lon + 1),
                            point(lat, lon + 1),
                        ];
                        if lat + 1 < LAT {
                            triangle([a, c, b]);
                        }
                        if lat > 0 {
                            triangle([a, d, c]);
                        }
                    }
                }
            } else {
                for normal in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
                    let u = if normal.y.abs() > 0.5 {
                        Vec3::X
                    } else {
                        Vec3::Y
                    };
                    let v = normal.cross(u);
                    let a = (normal - u - v) * 0.5;
                    let b = (normal + u - v) * 0.5;
                    let c = (normal + u + v) * 0.5;
                    let d = (normal - u + v) * 0.5;
                    triangle([a, b, c]);
                    triangle([a, c, d]);
                }
            }
        }
        faces
    }

    /// Paint the perspective scene, with orbit, zoom and triangle hit selection.
    pub fn viewport(&mut self, ui: &mut egui::Ui, grid: bool, gizmos: bool) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        if response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            self.yaw -= delta.x * 0.008;
            self.pitch = (self.pitch + delta.y * 0.006).clamp(0.12, 1.35);
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.distance = (self.distance * (-scroll * 0.002).exp()).clamp(4.0, 24.0);
            if ui.input(|i| i.key_pressed(egui::Key::F)) {
                self.yaw = 0.65;
                self.pitch = 0.48;
                self.distance = 11.0;
            }
        }
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(25));
        let projection = self.projection(rect);
        self.paint_scene(&painter, &projection, &response, grid, gizmos);
        painter.text(
            rect.right_top() + egui::vec2(-12.0, 12.0),
            egui::Align2::RIGHT_TOP,
            "3D · Perspective · VISUAL PREVIEW",
            egui::FontId::proportional(11.0),
            Color32::from_gray(160),
        );
        painter.text(
            rect.left_bottom() + egui::vec2(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag to orbit   ·   Scroll to zoom   ·   Click to select   ·   F to reset",
            egui::FontId::proportional(11.0),
            Color32::from_gray(150),
        );
        // Orientation compass, independent of scene selection.
        let center = rect.right_top() + egui::vec2(-42.0, 67.0);
        for (axis, name, color) in [
            (Vec3::X, "X", Color32::LIGHT_RED),
            (Vec3::Y, "Y", Color32::LIGHT_GREEN),
            (Vec3::Z, "Z", Color32::LIGHT_BLUE),
        ] {
            let end =
                center + egui::vec2(axis.dot(projection.right), -axis.dot(projection.up)) * 23.0;
            painter.line_segment([center, end], Stroke::new(1.5_f32, color));
            painter.text(
                end,
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(10.0),
                color,
            );
        }
    }
    fn paint_scene(
        &mut self,
        painter: &egui::Painter,
        projection: &Projection,
        response: &egui::Response,
        grid: bool,
        gizmos: bool,
    ) {
        let line = |a: Vec3, b: Vec3, color: Color32, width: f32| {
            if let (Some(a), Some(b)) = (projection.project(a), projection.project(b)) {
                painter.line_segment([a, b], Stroke::new(width, color));
            }
        };
        if grid {
            for i in -10..=10 {
                let color = if i == 0 {
                    Color32::from_gray(65)
                } else {
                    Color32::from_gray(38)
                };
                line(
                    Vec3::new(i as f32, -0.35, -10.0),
                    Vec3::new(i as f32, -0.35, 10.0),
                    color,
                    1.0,
                );
                line(
                    Vec3::new(-10.0, -0.35, i as f32),
                    Vec3::new(10.0, -0.35, i as f32),
                    color,
                    1.0,
                );
            }
            line(
                Vec3::new(-10.0, -0.34, 0.0),
                Vec3::new(10.0, -0.34, 0.0),
                Color32::from_rgb(105, 55, 52),
                1.0,
            );
            line(
                Vec3::new(0.0, -0.34, -10.0),
                Vec3::new(0.0, -0.34, 10.0),
                Color32::from_rgb(48, 74, 109),
                1.0,
            );
        }
        let mut faces = self.faces();
        faces.retain(|face| face.normal.dot(projection.eye - face.points[0]) > 0.0);
        faces.sort_by(|a, b| {
            let depth = |face: &Face| {
                ((face.points[0] + face.points[1] + face.points[2]) / 3.0 - projection.eye)
                    .dot(projection.forward)
            };
            depth(b).total_cmp(&depth(a))
        });
        for face in faces.iter().filter(|face| face.object == 0) {
            paint_face(painter, projection, face, self.light, self.roughness);
        }
        // Soft-looking contact silhouettes on top of the demonstration platform.
        for object in self.objects.iter().skip(1) {
            let points: Vec<_> = (0..24)
                .filter_map(|i| {
                    let a = i as f32 / 24.0 * std::f32::consts::TAU;
                    projection.project(Vec3::new(
                        object.position.x + a.cos() * object.scale.x * 0.7,
                        -0.02,
                        object.position.z + a.sin() * object.scale.z * 0.7,
                    ))
                })
                .collect();
            if points.len() == 24 {
                painter.add(egui::Shape::convex_polygon(
                    points,
                    Color32::from_black_alpha(35),
                    Stroke::NONE,
                ));
            }
        }
        let mut clicked = None;
        for face in faces.iter().filter(|face| face.object != 0) {
            let Some([a, b, c]) = paint_face(painter, projection, face, self.light, self.roughness)
            else {
                continue;
            };
            if response.clicked()
                && response
                    .interact_pointer_pos()
                    .is_some_and(|p| inside_triangle(p, [a, b, c]))
            {
                clicked = Some(face.object);
            }
        }
        if let Some(selected) = clicked {
            self.selected = selected;
        }
        if gizmos {
            if let Some(object) = self.objects.get(self.selected) {
                let rotation = glam::Quat::from_rotation_y(object.rotation.to_radians());
                let corners: Vec<_> = (0..8)
                    .map(|i| {
                        let sign = Vec3::new(
                            if i & 1 == 0 { -0.5 } else { 0.5 },
                            if i & 2 == 0 { -0.5 } else { 0.5 },
                            if i & 4 == 0 { -0.5 } else { 0.5 },
                        );
                        projection.project(object.position + rotation * (sign * object.scale))
                    })
                    .collect();
                for i in 0..8 {
                    for bit in [1, 2, 4] {
                        let j = i ^ bit;
                        if i < j {
                            if let (Some(a), Some(b)) = (corners[i], corners[j]) {
                                painter.line_segment([a, b], Stroke::new(1.0_f32, AMBER));
                            }
                        }
                    }
                }
            }
        }
        if gizmos {
            if let Some(object) = self.objects.get(self.selected) {
                if let Some(origin) = projection.project(object.position) {
                    for (axis, name, color) in [
                        (Vec3::X, "X", Color32::from_rgb(229, 83, 75)),
                        (Vec3::Y, "Y", Color32::from_rgb(79, 179, 107)),
                        (Vec3::Z, "Z", Color32::from_rgb(83, 147, 229)),
                    ] {
                        if let Some(tip) = projection.project(object.position + axis * 1.3) {
                            painter.arrow(origin, tip - origin, Stroke::new(2.0_f32, color));
                            painter.text(
                                tip,
                                egui::Align2::CENTER_BOTTOM,
                                name,
                                egui::FontId::proportional(11.0),
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn paint_face(
    painter: &egui::Painter,
    projection: &Projection,
    face: &Face,
    intensity: f32,
    roughness: f32,
) -> Option<[Pos2; 3]> {
    let [Some(a), Some(b), Some(c)] = face.points.map(|p| projection.project(p)) else {
        return None;
    };
    let diffuse = face
        .normal
        .dot(Vec3::new(-0.6, 0.85, 0.5).normalize())
        .max(0.0);
    let amount =
        (0.32 + diffuse * intensity * 0.75 + (1.0 - roughness) * diffuse.powi(12) * 0.2).min(1.3);
    let color = Color32::from_rgb(
        (face.color.r() as f32 * amount).min(255.0) as u8,
        (face.color.g() as f32 * amount).min(255.0) as u8,
        (face.color.b() as f32 * amount).min(255.0) as u8,
    );
    let mut mesh = egui::Mesh::default();
    for point in [a, b, c] {
        mesh.colored_vertex(point, color);
    }
    mesh.add_triangle(0, 1, 2);
    painter.add(egui::Shape::mesh(mesh));
    Some([a, b, c])
}

fn inside_triangle(p: Pos2, points: [Pos2; 3]) -> bool {
    let cross = |a: Pos2, b: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let signs = [
        cross(points[0], points[1]),
        cross(points[1], points[2]),
        cross(points[2], points[0]),
    ];
    signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perspective_is_finite_centered_and_changes_with_orbit() {
        let mut preview = Preview3d::default();
        let rect = Rect::from_min_size(Pos2::new(200.0, 100.0), egui::vec2(800.0, 500.0));
        let projection = preview.projection(rect);
        let center = projection.project(Vec3::new(0.0, 0.8, 0.0)).unwrap();
        assert!(center.distance(rect.center()) < 0.001);
        assert!(projection
            .project(projection.eye - projection.forward)
            .is_none());
        let before = projection.project(Vec3::X).unwrap();
        preview.yaw += 1.0;
        let after = preview.projection(rect).project(Vec3::X).unwrap();
        assert!(before.distance(after) > 1.0);
        assert!(after.is_finite());
    }

    #[test]
    fn preview_geometry_is_bounded_and_normals_point_outwards() {
        let preview = Preview3d::default();
        let faces = preview.faces();
        assert!(faces.len() > 100 && faces.len() < 1000);
        for face in faces {
            let object = &preview.objects[face.object];
            let center = (face.points[0] + face.points[1] + face.points[2]) / 3.0;
            assert!(face.normal.is_finite());
            assert!(face.normal.dot(center - object.position) > 0.0);
        }
    }

    #[test]
    fn selection_hit_test_rejects_outside_points() {
        let triangle = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!(inside_triangle(Pos2::new(2.0, 2.0), triangle));
        assert!(!inside_triangle(Pos2::new(8.0, 8.0), triangle));
    }
}
