//! Physics Editor panel — rigid body/collider/joint configuration, live preview.

use egui::{Color32, DragValue, RichText, ScrollArea, Ui, Vec2};
use kiln_runtime::physics::{PhysicsWorld, RigidBodyHandle, ColliderHandle, JointHandle};
use kiln_runtime::physics::RigidBodyType;
use egui::Stroke;

/// Physics editor state.
#[derive(Debug, Default)]
pub struct PhysicsEditorState {
    /// Physics world being edited.
    pub physics_world: Option<PhysicsWorld>,
    /// Selected rigid body handle.
    pub selected_body: Option<RigidBodyHandle>,
    /// Selected collider handle.
    pub selected_collider: Option<ColliderHandle>,
    /// Selected joint handle.
    pub selected_joint: Option<JointHandle>,
    /// Preview playing.
    pub preview_playing: bool,
    /// Preview time accumulator.
    pub preview_time: f32,
    /// Preview fixed timestep.
    pub preview_dt: f32,
    /// Gravity X.
    pub gravity_x: f32,
    /// Gravity Y.
    pub gravity_y: f32,
    /// Show add rigid body dialog.
    pub show_add_body_dialog: bool,
    /// New body type.
    pub new_body_type: RigidBodyType,
    /// New body position.
    pub new_body_pos: [f32; 2],
    /// Show add collider dialog.
    pub show_add_collider_dialog: bool,
    /// New collider shape type.
    pub new_collider_shape: ColliderShape,
    /// New collider parameters.
    pub new_collider_radius: f32,
    pub new_collider_half_extents: [f32; 2],
    /// New collider density.
    pub new_collider_density: f32,
    /// New collider friction.
    pub new_collider_friction: f32,
    /// New collider restitution.
    pub new_collider_restitution: f32,
    /// Selected collider parent body.
    pub new_collider_parent: Option<RigidBodyHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderShape {
    Ball,
    Cuboid,
    Capsule,
}

impl PhysicsEditorState {
    pub fn new() -> Self {
        Self {
            physics_world: None,
            selected_body: None,
            selected_collider: None,
            selected_joint: None,
            preview_playing: false,
            preview_time: 0.0,
            preview_dt: 1.0 / 60.0,
            gravity_x: 0.0,
            gravity_y: -9.81,
            show_add_body_dialog: false,
            new_body_type: RigidBodyType::Dynamic,
            new_body_pos: [0.0, 0.0],
            show_add_collider_dialog: false,
            new_collider_shape: ColliderShape::Ball,
            new_collider_radius: 1.0,
            new_collider_half_extents: [1.0, 1.0],
            new_collider_density: 1.0,
            new_collider_friction: 0.5,
            new_collider_restitution: 0.0,
            new_collider_parent: None,
        }
    }

    /// Initialize or get the physics world.
    pub fn physics_world_mut(&mut self) -> &mut PhysicsWorld {
        if self.physics_world.is_none() {
            let mut world = PhysicsWorld::new();
            world.set_gravity(self.gravity_x, self.gravity_y);
            self.physics_world = Some(world);
        }
        self.physics_world.as_mut().unwrap()
    }

    /// Update the preview.
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            if let Some(world) = &mut self.physics_world {
                world.step();
            }
        }
    }


    /// Update the preview.
    pub fn update_preview(&mut self, dt: f32) {
        if self.preview_playing {
            if let Some(world) = &mut self.physics_world {
                world.step();
            }
        }
    }

    /// Render the physics editor panel.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Physics Editor");
        ui.add_space(8.0);

        // Initialize physics world if needed
        let _world = self.physics_world_mut();

        // Gravity controls
        ui.horizontal(|ui| {
            ui.label("Gravity:");
            ui.add(egui::DragValue::new(&mut self.gravity_x).speed(0.1).suffix(" x"));
            ui.add(egui::DragValue::new(&mut self.gravity_y).speed(0.1).suffix(" y"));
            if ui.button("Apply").clicked() {
                if let Some(world) = &mut self.physics_world {
                    world.set_gravity(self.gravity_x, self.gravity_y);
                }
            }
            if ui.button("Reset").clicked() {
                self.gravity_x = 0.0;
                self.gravity_y = -9.81;
                if let Some(world) = &mut self.physics_world {
                    world.set_gravity(0.0, -9.81);
                }
            }
        );

        ui.add_space(8.0);

        // Preview controls
        ui.horizontal(|ui| {
            if ui.button(if self.preview_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                self.preview_playing = !self.preview_playing;
            }
            if ui.button("⏹ Stop").clicked() {
                self.preview_playing = false;
                self.preview_time = 0.0;
            }
            if ui.button("↺ Reset").clicked() {
                self.preview_playing = false;
                self.preview_time = 0.0;
                self.physics_world = None;
                self.selected_body = None;
                self.selected_collider = None;
                self.selected_joint = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(world) = &self.physics_world {
                    ui.label(format!("Bodies: {}", world.rigid_body_set.len()));
                    ui.label(format!("Colliders: {}", world.collider_set.len()));
                    ui.label(format!("Joints: {}", world.joint_set.len()));
                }
            });
        });

        ui.add_space(8.0);

        // Rigid Bodies
        ui.horizontal(|ui| {
            ui.heading("Rigid Bodies");
            if ui.button("➕ Add Body").clicked() {
                self.show_add_body_dialog = true;
                self.new_body_type = RigidBodyType::Dynamic;
                self.new_body_pos = [0.0, 0.0];
            }
        );

        ui.add_space(4.0);

        if let Some(world) = &self.physics_world {
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                let mut remove_body = None;
                for (handle, body) in world.rigid_body_set.iter() {
                    let pos = body.translation();
                    let vel = body.linvel();
                    let body_type = match body.body_type() {
                        RigidBodyType::Dynamic => "Dynamic",
                        RigidBodyType::Static => "Static",
                        RigidBodyType::KinematicPositionBased => "Kinematic (Pos)",
                        RigidBodyType::KinematicVelocityBased => "Kinematic (Vel)",
                        _ => "Other",
                    };
                    let is_selected = self.selected_body == Some(handle);
                    let mut selected = is_selected;

                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, format!("Body {:?} [{}]", handle, body_type)).clicked() {
                            self.selected_body = Some(handle);
                            self.selected_collider = None;
                            self.selected_joint = None;
                        }
                        ui.label(format!("Pos: ({:.1}, {:.1})", pos.x, pos.y));
                        ui.label(format!("Vel: ({:.1}, {:.1})", vel.x, vel.y));
                        if ui.small_button("🗑").clicked() {
                            remove_body = Some(handle);
                        }
                    });
                }

                if let Some(handle) = remove_body {
                    if let Some(world) = &mut self.physics_world {
                        world.rigid_body_set.remove(
                            handle,
                            &mut world.collider_set,
                            &mut world.joint_set,
                        );
                        if self.selected_body == Some(handle) {
                            self.selected_body = None;
                        }
                    }
                }

                if world.rigid_body_set.is_empty() {
                    ui.label("No rigid bodies. Click 'Add Body' to create one.");
                }
            }
        }


        ui.add_space(8.0);

        // Add body dialog
        if self.show_add_body_dialog {
            egui::Window::new("Add Rigid Body")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Body Type:");
                    egui::ComboBox::from_label("")
                        .selected_text(match self.new_body_type {
                            RigidBodyType::Dynamic => "Dynamic",
                            RigidBodyType::Static => "Static",
                            RigidBodyType::KinematicPositionBased => "Kinematic (Pos)",
                            RigidBodyType::KinematicVelocityBased => "Kinematic (Vel)",
                            _ => "Other",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.new_body_type, RigidBodyType::Dynamic, "Dynamic");
                            ui.selectable_value(&mut self.new_body_type, RigidBodyType::Static, "Static");
                            ui.selectable_value(&mut self.new_body_type, RigidBodyType::KinematicPositionBased, "Kinematic (Position)");
                            ui.selectable_value(&mut self.new_body_type, RigidBodyType::KinematicVelocityBased, "Kinematic (Velocity)");
                        });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Position X:");
                        ui.add(egui::DragValue::new(&mut self.new_body_pos[0]).speed(0.1).suffix(" px"));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut self.new_body_pos[1]).speed(0.1).suffix(" px"));
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_add_body_dialog = false;
                        }
                        if ui.button("Create").clicked() {
                            if let Some(world) = &mut self.physics_world {
                                let body = match self.new_body_type {
                                    RigidBodyType::Dynamic => rapier2d::prelude::RigidBodyBuilder::dynamic(),
                                    RigidBodyType::Static => rapier2d::prelude::RigidBodyBuilder::static(),
                                    RigidBodyType::KinematicPositionBased => rapier2d::prelude::RigidBodyBuilder::kinematic_position_based(),
                                    RigidBodyType::KinematicVelocityBased => rapier2d::prelude::RigidBodyBuilder::kinematic_velocity_based(),
                                    _ => rapier2d::prelude::RigidBodyBuilder::dynamic(),
                                }
                                .translation(rapier2d::prelude::Vec2::new(self.new_body_pos[0], self.new_body_pos[1]));
                                world.create_rigid_body(body);
                            }
                            self.show_add_body_dialog = false;
                        });
            });
        }

        ui.add_space(8.0);

        // Colliders
        ui.horizontal(|ui| {
            ui.heading("Colliders");
            if ui.button("➕ Add Collider").clicked() {
                self.show_add_collider_dialog = true;
                self.new_collider_shape = ColliderShape::Ball;
                self.new_collider_radius = 1.0;
                self.new_collider_half_extents = [1.0, 1.0];
                self.new_collider_density = 1.0;
                self.new_collider_friction = 0.5;
                self.new_collider_restitution = 0.0;
                // Set parent to selected body if any
                self.new_collider_parent = self.selected_body;
            }
        );

        ui.add_space(4.0);

        if let Some(world) = &self.physics_world {
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for (handle, collider) in world.collider_set.iter() {
                    let shape_type = format!("{:?}", collider.shape().shape_type());
                    let parent = collider.parent();
                    let is_selected = self.selected_collider == Some(handle);
                    let mut selected = false;

                    ui.horizontal(|ui| {
                        if ui.selectable_label(false, format!("Collider {:?} [{}]", handle, shape_type)).clicked() {
                            self.selected_collider = Some(handle);
                        }
                        ui.label(format!("Parent: {:?}", parent));
                        if ui.small_button("🗑").clicked() {
                            if let Some(world) = &mut self.physics_world {
                                world.collider_set.remove(handle);
                            }
                        }
                    });
                }
            }

            // Add collider dialog
            if self.show_add_collider_dialog {
                egui::Window::new("Add Collider")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label("Shape:");
                        egui::ComboBox::from_label("")
                            .selected_text(match self.new_collider_shape {
                                ColliderShape::Ball => "Ball",
                                ColliderShape::Cuboid => "Cuboid",
                                ColliderShape::Capsule => "Capsule",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.new_collider_shape, ColliderShape::Ball, "Ball");
                                ui.selectable_value(&mut self.new_collider_shape, ColliderShape::Cuboid, "Cuboid");
                                ui.selectable_value(&mut self.new_collider_shape, ColliderShape::Capsule, "Capsule");
                            });

                        match self.new_collider_shape {
                            ColliderShape::Ball => {
                                ui.horizontal(|ui| {
                                    ui.label("Radius:");
                                    ui.add(egui::DragValue::new(&mut self.new_collider_radius).speed(0.1).suffix(" px"));
                                });
                            }
                            ColliderShape::Cuboid => {
                                ui.horizontal(|ui| {
                                    ui.label("Half Width:");
                                    ui.add(egui::DragValue::new(&mut self.new_collider_half_extents[0]).speed(0.1).suffix(" px"));
                                    ui.label("Half Height:");
                                    ui.add(egui::DragValue::new(&mut self.new_collider_half_extents[1]).speed(0.1).suffix(" px"));
                                });
                            }
                            ColliderShape::Capsule => {
                                ui.horizontal(|ui| {
                                    ui.label("Radius:");
                                    ui.add(egui::DragValue::new(&mut self.new_collider_radius).speed(0.1).suffix(" px"));
                                    ui.label("Half Height:");
                                    ui.add(egui::DragValue::new(&mut self.new_collider_half_extents[1]).speed(0.1).suffix(" px"));
                                });
                            }
                        }

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Density:");
                            ui.add(egui::DragValue::new(&mut self.new_collider_density).speed(0.1).range(0.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Friction:");
                            ui.add(egui::DragValue::new(&mut self.new_collider_friction).speed(0.01).range(0.0..=10.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Restitution:");
                            ui.add(egui::DragValue::new(&mut self.new_collider_restitution).speed(0.01).range(0.0..=1.0));
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_add_collider_dialog = false;
                            }
                            if ui.button("Create").clicked() {
                                if let Some(parent) = self.new_collider_parent {
                                    if let Some(world) = &mut self.physics_world {
                                        let collider = match self.new_collider_shape {
                                            ColliderShape::Ball => {
                                                rapier2d::prelude::ColliderBuilder::ball(self.new_collider_radius)
                                            }
                                            ColliderShape::Cuboid => {
                                                rapier2d::prelude::ColliderBuilder::cuboid(
                                                    self.new_collider_half_extents[0],
                                                    self.new_collider_half_extents[1],
                                                )
                                            }
                                            ColliderShape::Capsule => {
                                                rapier2d::prelude::ColliderBuilder::capsule(
                                                    self.new_collider_radius,
                                                    self.new_collider_half_extents[1],
                                                )
                                            }
                                        }
                                        .density(self.new_collider_density)
                                        .friction(self.new_collider_friction)
                                        .restitution(self.new_collider_restitution);
                                        world.create_collider(collider, parent);
                                    }
                                }
                                self.show_add_collider_dialog = false;
                            }
                        });
            });
        }

        ui.add_space(8.0);


        ui.add_space(8.0);

        // Preview
        ui.separator();
        ui.heading("Physics Preview");
        ui.add_space(4.0);

        let preview_size = Vec2::new(400.0, 300.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(400.0, 300.0), egui::Sense::hover());

        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(20, 20, 30));
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(80)));

        if let Some(world) = &self.physics_world {
            let center = rect.center();
            let scale = 20.0; // 1 world unit = 20 pixels

            // Draw colliders
            for (handle, collider) in world.collider_set.iter() {
                let pos = collider.position().translation;
                let center_pos = egui::pos2(
                    center.x + pos.x * 20.0,
                    center.y - pos.y * 20.0,
                );

                // Draw based on shape type
                if let Some(ball) = collider.shape().as_ball() {
                    ui.painter().circle_stroke(center_pos, ball.radius * 20.0, Stroke::new(1.5, Color32::from_rgb(100, 200, 255)));
                } else if let Some(cuboid) = collider.shape().as_cuboid() {
                    let half_extents = cuboid.half_extents();
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(
                            center_pos.x - half_extents.x * 20.0,
                            center_pos.y - half_extents.y * 20.0,
                        ),
                        egui::pos2(
                            center_pos.x + half_extents.x * 20.0,
                            center_pos.y + half_extents.y * 20.0,
                        ),
                    );
                    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.5, Color32::from_rgb(100, 200, 255)));
                } else if let Some(capsule) = collider.shape().as_capsule() {
                    // Draw capsule as two circles + rectangle
                    let radius = capsule.radius * 20.0;
                    let half_height = capsule.half_height * 20.0;
                    ui.painter().circle_stroke(center_pos + egui::vec2(0.0, -half_height), radius, Stroke::new(1.5, Color32::from_rgb(100, 200, 255)));
                    ui.painter().circle_stroke(center_pos + egui::vec2(0.0, half_height), radius, Stroke::new(1.5, Color32::from_rgb(100, 200, 255)));
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(center_pos.x - radius, center_pos.y - half_height),
                        egui::pos2(center_pos.x + radius, center_pos.y + half_height),
                    );
                    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.5, Color32::from_rgb(100, 200, 255)));
                }
            }

            // Draw body positions
            for (handle, body) in world.rigid_body_set.iter() {
                let pos = body.translation();
                let center_pos = egui::pos2(
                    center.x + pos.x * 20.0,
                    center.y - pos.y * 20.0,
                );
                ui.painter().circle_filled(center_pos, 4.0, Color32::from_rgb(255, 200, 100));
            }
        }

        self.update_preview(self.preview_dt);
    }
}


