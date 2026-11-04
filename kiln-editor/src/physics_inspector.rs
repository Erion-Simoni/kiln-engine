//! Authored collider/joint/camera controls, separate from runtime preview tools.
use kiln_core::physics_data::*;

/// Closed world-space outline matching the collider scale/rotation convention.
pub fn collider_outline(pose: kiln_core::Transform, shape: ColliderShape) -> Vec<[f32; 2]> {
    use kiln_core::Vec2;
    let points: Vec<Vec2> = match shape {
        ColliderShape::Circle { radius } => (0..24)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::TAU / 24.0;
                Vec2::new(angle.cos(), angle.sin()) * radius * pose.scale.abs().max_element()
            })
            .collect(),
        ColliderShape::Rectangle {
            half_width,
            half_height,
        } => [
            [-half_width, -half_height],
            [half_width, -half_height],
            [half_width, half_height],
            [-half_width, half_height],
        ]
        .into_iter()
        .map(|point| Vec2::from_array(point) * pose.scale.abs())
        .collect(),
    };
    let mut outline: Vec<_> = points
        .into_iter()
        .map(|point| (pose.position + point.rotate(Vec2::from_angle(pose.rotation))).to_array())
        .collect();
    if let Some(first) = outline.first().copied() {
        outline.push(first);
    }
    outline
}

pub fn body_ui(ui: &mut egui::Ui, body: &mut RigidBody2D, collider: &mut Collider2D) -> bool {
    let before = (*body, *collider);
    egui::ComboBox::from_label("Body type")
        .selected_text(format!("{:?}", body.body_type))
        .show_ui(ui, |ui| {
            for (kind, name) in [
                (RigidBodyType::Static, "Static"),
                (RigidBodyType::Dynamic, "Dynamic"),
                (RigidBodyType::Kinematic, "Kinematic"),
            ] {
                ui.selectable_value(&mut body.body_type, kind, name);
            }
        });
    ui.horizontal(|ui| {
        ui.label("Velocity");
        for value in &mut body.linear_velocity {
            ui.add(egui::DragValue::new(value).speed(0.1));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Angular velocity");
        ui.add(egui::DragValue::new(&mut body.angular_velocity).speed(0.01));
        ui.label("Gravity scale");
        ui.add(egui::DragValue::new(&mut body.gravity_scale).speed(0.01));
    });
    ui.checkbox(&mut body.can_sleep, "Allow sleeping");
    let circle = matches!(collider.shape, ColliderShape::Circle { .. });
    ui.horizontal(|ui| {
        if ui.selectable_label(circle, "Circle").clicked() && !circle {
            collider.shape = ColliderShape::Circle { radius: 16.0 };
        }
        if ui.selectable_label(!circle, "Rectangle").clicked() && circle {
            collider.shape = ColliderShape::Rectangle {
                half_width: 16.0,
                half_height: 16.0,
            };
        }
    });
    match &mut collider.shape {
        ColliderShape::Circle { radius } => {
            ui.add(
                egui::DragValue::new(radius)
                    .prefix("Radius ")
                    .range(0.001..=8192.0),
            );
        }
        ColliderShape::Rectangle {
            half_width,
            half_height,
        } => {
            ui.add(
                egui::DragValue::new(half_width)
                    .prefix("Half width ")
                    .range(0.001..=8192.0),
            );
            ui.add(
                egui::DragValue::new(half_height)
                    .prefix("Half height ")
                    .range(0.001..=8192.0),
            );
        }
    }
    ui.checkbox(
        &mut collider.is_sensor,
        "Sensor / trigger (no collision response)",
    );
    ui.horizontal(|ui| {
        ui.label("Membership / mask bitsets");
        ui.add(egui::DragValue::new(&mut collider.layer));
        ui.add(egui::DragValue::new(&mut collider.mask));
    });
    ui.add(
        egui::DragValue::new(&mut collider.friction)
            .prefix("Friction ")
            .range(0.0..=10.0)
            .speed(0.01),
    );
    ui.add(
        egui::DragValue::new(&mut collider.restitution)
            .prefix("Restitution ")
            .range(0.0..=1.0)
            .speed(0.01),
    );
    ui.add(
        egui::DragValue::new(&mut collider.density)
            .prefix("Density ")
            .range(0.0..=10000.0)
            .speed(0.1),
    );
    before != (*body, *collider)
}

pub fn joint_ui(ui: &mut egui::Ui, joint: &mut Joint2D) -> bool {
    let before = joint.clone();
    ui.horizontal(|ui| {
        ui.label("Endpoint A / B");
        ui.text_edit_singleline(&mut joint.entity_a);
        ui.text_edit_singleline(&mut joint.entity_b);
    });
    egui::ComboBox::from_label("Joint type")
        .selected_text(format!("{:?}", joint.joint_type))
        .show_ui(ui, |ui| {
            for (kind, name) in [
                (JointType::Distance, "Distance"),
                (JointType::Revolute, "Revolute"),
                (JointType::Prismatic, "Prismatic"),
            ] {
                ui.selectable_value(&mut joint.joint_type, kind, name);
            }
        });
    ui.horizontal(|ui| {
        ui.label("Local anchors A / B");
        for value in joint.anchor_a.iter_mut().chain(joint.anchor_b.iter_mut()) {
            ui.add(egui::DragValue::new(value).speed(0.1));
        }
    });
    ui.add(
        egui::DragValue::new(&mut joint.length)
            .prefix("Distance ")
            .range(0.001..=100000.0),
    );
    ui.add(
        egui::DragValue::new(&mut joint.reference_angle)
            .prefix("Slider axis angle ")
            .speed(0.01),
    );
    ui.checkbox(&mut joint.enable_limit, "Slider limits");
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut joint.lower_translation));
        ui.add(egui::DragValue::new(&mut joint.upper_translation));
    });
    ui.checkbox(&mut joint.enable_motor, "Enable motor");
    ui.add(egui::DragValue::new(&mut joint.motor_speed).prefix("Motor speed "));
    ui.add(
        egui::DragValue::new(&mut joint.max_motor_torque)
            .prefix("Maximum motor force ")
            .range(0.0..=100000.0),
    );
    *joint != before
}

pub fn camera_ui(
    ui: &mut egui::Ui,
    camera: &mut kiln_core::camera::CameraSettings,
    ids: &[String],
    size: [f32; 2],
) -> bool {
    let before = camera.clone();
    egui::ComboBox::from_label("Follow entity")
        .selected_text(camera.target.as_deref().unwrap_or("Stationary"))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut camera.target, None, "Stationary");
            for id in ids {
                ui.selectable_value(&mut camera.target, Some(id.clone()), id);
            }
        });
    ui.horizontal(|ui| {
        ui.label("Origin / follow offset");
        for value in &mut camera.position {
            ui.add(egui::DragValue::new(value).speed(0.1));
        }
    });
    ui.add(
        egui::DragValue::new(&mut camera.zoom)
            .prefix("Zoom ")
            .speed(0.01)
            .range(0.05..=32.0),
    );
    ui.add(
        egui::DragValue::new(&mut camera.smoothing)
            .prefix("Follow response ")
            .speed(0.1)
            .range(0.0..=100.0),
    );
    ui.checkbox(&mut camera.pixel_perfect, "Pixel-perfect (integer zoom)");
    ui.add(
        egui::DragValue::new(&mut camera.pixels_per_unit)
            .prefix("Pixels per world unit ")
            .range(0.01..=10000.0),
    );
    let mut bounded = camera.bounds.is_some();
    if ui.checkbox(&mut bounded, "Clamp to world bounds").changed() {
        camera.bounds = bounded.then_some([0.0, 0.0, size[0], size[1]]);
    }
    if let Some(bounds) = &mut camera.bounds {
        ui.horizontal(|ui| {
            for (value, name) in bounds
                .iter_mut()
                .zip(["Left ", "Bottom ", "Right ", "Top "])
            {
                ui.add(egui::DragValue::new(value).prefix(name));
            }
        });
    }
    if let Err(error) = camera.validate() {
        ui.colored_label(egui::Color32::YELLOW, error);
    }
    *camera != before
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_outlines_match_scaled_rotated_body_geometry() {
        let pose = kiln_core::Transform::new(10.0, 20.0)
            .with_scale(kiln_core::Vec2::new(-2.0, 3.0))
            .with_rotation(std::f32::consts::FRAC_PI_2);
        let rectangle = collider_outline(
            pose,
            ColliderShape::Rectangle {
                half_width: 2.0,
                half_height: 1.0,
            },
        );
        assert_eq!(rectangle.len(), 5);
        assert_eq!(rectangle.first(), rectangle.last());
        assert!((rectangle[0][0] - 13.0).abs() < 0.0001 && (rectangle[0][1] - 16.0).abs() < 0.0001);
        let circle = collider_outline(pose, ColliderShape::Circle { radius: 2.0 });
        assert_eq!(circle.len(), 25);
        assert_eq!(circle.first(), circle.last());
        for point in circle {
            let distance = kiln_core::Vec2::from_array(point).distance(pose.position);
            assert!((distance - 6.0).abs() < 0.0001);
        }
    }
}
