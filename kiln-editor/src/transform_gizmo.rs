//! Viewport transform-gizmo drag math.

use egui::{Pos2, Vec2 as EguiVec2};
use kiln_core::{Component, Scene, Transform, Vec2};

/// Transform operation selected for viewport interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    /// Disable viewport transform editing.
    None,
    /// Move the entity in world space.
    #[default]
    Translate,
    /// Rotate around the entity's origin.
    Rotate,
    /// Scale uniformly around the entity's origin.
    Scale,
}

/// Apply a primary gizmo result to every selected world transform around the primary entity.
pub fn apply_group_transform(
    mode: GizmoMode,
    primary_id: &str,
    primary_result: Transform,
    selected: &[(String, Transform)],
) -> Vec<(String, Transform)> {
    let Some((_, pivot)) = selected.iter().find(|(id, _)| id == primary_id) else {
        return selected.to_vec();
    };
    let translation = primary_result.position - pivot.position;
    let rotation = primary_result.rotation - pivot.rotation;
    let scale = primary_result.scale.x / pivot.scale.x;
    let (sin, cos) = rotation.sin_cos();
    selected
        .iter()
        .map(|(id, initial)| {
            let mut target = *initial;
            match mode {
                GizmoMode::None => {}
                GizmoMode::Translate => target.position += translation,
                GizmoMode::Rotate => {
                    let relative = initial.position - pivot.position;
                    target.position = pivot.position
                        + Vec2::new(
                            relative.x * cos - relative.y * sin,
                            relative.x * sin + relative.y * cos,
                        );
                    target.rotation += rotation;
                }
                GizmoMode::Scale => {
                    target.position = pivot.position + (initial.position - pivot.position) * scale;
                    target.scale *= scale;
                }
            }
            (id.clone(), target)
        })
        .collect()
}

/// Order group targets so selected ancestors update before their descendants.
pub fn order_group_targets(
    scene: &Scene,
    targets: &[(String, Transform)],
) -> Vec<(String, Transform)> {
    fn depth(scene: &Scene, entity_id: &str) -> usize {
        let mut current = entity_id;
        let mut depth = 0;
        while depth < scene.entities.len() {
            let parent = scene
                .entities
                .iter()
                .find(|entity| entity.id == current)
                .and_then(|entity| {
                    entity
                        .components
                        .iter()
                        .find_map(|component| match component {
                            Component::Parent { parent } => Some(parent.as_str()),
                            _ => None,
                        })
                });
            let Some(parent) = parent else { break };
            current = scene
                .entities
                .iter()
                .find(|entity| entity.id == parent)
                .map(|entity| entity.id.as_str())
                .unwrap_or(parent);
            depth += 1;
        }
        depth
    }

    let mut ordered = targets.to_vec();
    ordered.sort_by_key(|(entity_id, _)| depth(scene, entity_id));
    ordered
}

/// One pointer gesture evaluated from its press state, independent of frame rate.
#[derive(Debug, Clone)]
pub struct GizmoDrag {
    mode: GizmoMode,
    origin: Pos2,
    initial_world: Transform,
    units_per_point: [f32; 2],
    start_angle: f32,
    start_vector: Vec2,
}

impl GizmoDrag {
    /// Start a gizmo drag. `pointer_world` is the pointer press mapped into world space.
    pub fn new(
        mode: GizmoMode,
        origin: Pos2,
        initial_world: Transform,
        units_per_point: [f32; 2],
        pointer_world: Vec2,
    ) -> Option<Self> {
        if mode == GizmoMode::None
            || !units_per_point
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        {
            return None;
        }
        let start_vector = pointer_world - initial_world.position;
        let start_angle = start_vector.to_angle();
        Some(Self {
            mode,
            origin,
            initial_world,
            units_per_point,
            start_angle,
            start_vector,
        })
    }

    /// Evaluate the gesture at the current pointer position.
    pub fn at(&self, pointer: Pos2, grid: Option<f32>) -> Transform {
        let mut value = self.initial_world;
        let dx = pointer.x - self.origin.x;
        let dy = pointer.y - self.origin.y;
        match self.mode {
            GizmoMode::None => {}
            GizmoMode::Translate => {
                value.position.x += dx * self.units_per_point[0];
                value.position.y -= dy * self.units_per_point[1];
                if let Some(grid) = grid.filter(|grid| grid.is_finite() && *grid > 0.0) {
                    value.position.x = (value.position.x / grid).round() * grid;
                    value.position.y = (value.position.y / grid).round() * grid;
                }
            }
            GizmoMode::Rotate => {
                let pointer_world = Vec2::new(
                    self.initial_world.position.x
                        + self.start_vector.x
                        + (pointer.x - self.origin.x) * self.units_per_point[0],
                    self.initial_world.position.y + self.start_vector.y
                        - (pointer.y - self.origin.y) * self.units_per_point[1],
                );
                let angle = (pointer_world - self.initial_world.position).to_angle();
                let delta = (angle - self.start_angle + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                value.rotation += delta;
            }
            GizmoMode::Scale => {
                let factor = (-dy * 0.01).exp().clamp(0.001, 1000.0);
                value.scale *= factor;
            }
        }
        value
    }
}

/// Whether a press is close enough to the displayed gizmo handle to begin its gesture.
pub fn hits_handle(mode: GizmoMode, pointer: Pos2, center: Pos2) -> bool {
    match mode {
        GizmoMode::None => false,
        GizmoMode::Translate => true,
        GizmoMode::Rotate => {
            let distance = pointer.distance(center);
            (28.0..=48.0).contains(&distance)
        }
        GizmoMode::Scale => pointer.distance(center + EguiVec2::new(54.0, 0.0)) <= 14.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_drag_uses_world_units_and_grid_snap() {
        let drag = GizmoDrag::new(
            GizmoMode::Translate,
            Pos2::new(10.0, 10.0),
            Transform::new(1.0, 2.0),
            [2.0, 3.0],
            Vec2::new(0.0, 0.0),
        )
        .unwrap();
        let value = drag.at(Pos2::new(12.0, 8.0), Some(2.0));
        assert_eq!(value.position, Vec2::new(6.0, 8.0));
    }

    #[test]
    fn group_rotation_and_scale_keep_primary_pivot_and_relative_offsets() {
        let selected = vec![
            ("pivot".into(), Transform::new(10.0, 20.0)),
            (
                "other".into(),
                Transform::new(20.0, 20.0).with_scale(Vec2::splat(2.0)),
            ),
        ];
        let rotated = apply_group_transform(
            GizmoMode::Rotate,
            "pivot",
            Transform::new(10.0, 20.0).with_rotation(std::f32::consts::FRAC_PI_2),
            &selected,
        );
        assert_eq!(rotated[0].1.position, Vec2::new(10.0, 20.0));
        assert!((rotated[1].1.position.x - 10.0).abs() < 0.0001);
        assert!((rotated[1].1.position.y - 30.0).abs() < 0.0001);
        assert!((rotated[1].1.rotation - std::f32::consts::FRAC_PI_2).abs() < 0.0001);

        let scaled = apply_group_transform(
            GizmoMode::Scale,
            "pivot",
            Transform::new(10.0, 20.0).with_scale(Vec2::splat(2.0)),
            &selected,
        );
        assert_eq!(scaled[0].1.position, Vec2::new(10.0, 20.0));
        assert_eq!(scaled[1].1.position, Vec2::new(30.0, 20.0));
        assert_eq!(scaled[1].1.scale, Vec2::splat(4.0));
    }

    #[test]
    fn group_targets_apply_selected_parents_before_children() {
        let scene = Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                kiln_core::Entity {
                    id: "parent".into(),
                    name: "Parent".into(),
                    components: vec![],
                },
                kiln_core::Entity {
                    id: "child".into(),
                    name: "Child".into(),
                    components: vec![Component::Parent {
                        parent: "parent".into(),
                    }],
                },
            ],
        };
        let targets = vec![
            ("child".into(), Transform::new(1.0, 0.0)),
            ("parent".into(), Transform::new(2.0, 0.0)),
        ];
        let ordered = order_group_targets(&scene, &targets);
        assert_eq!(ordered[0].0, "parent");
        assert_eq!(ordered[1].0, "child");
    }

    #[test]
    fn rotate_drag_tracks_pointer_angle_and_wraps_at_full_turn() {
        let drag = GizmoDrag::new(
            GizmoMode::Rotate,
            Pos2::new(100.0, 100.0),
            Transform::new(0.0, 0.0).with_rotation(0.25),
            [1.0, 1.0],
            Vec2::new(1.0, 0.0),
        )
        .unwrap();
        let rotated = drag.at(Pos2::new(99.0, 99.0), None);
        assert!((rotated.rotation - (0.25 + std::f32::consts::FRAC_PI_2)).abs() < 0.0001);
        let wrapped_drag = GizmoDrag::new(
            GizmoMode::Rotate,
            Pos2::new(100.0, 100.0),
            Transform::new(0.0, 0.0).with_rotation(0.25),
            [1.0, 1.0],
            Vec2::new(-1.0, 0.001),
        )
        .unwrap();
        let wrapped = wrapped_drag.at(Pos2::new(100.0, 100.002), None);
        assert!(wrapped.rotation.is_finite());
        assert!(
            (wrapped.rotation - 0.253).abs() < 0.01,
            "wrapped angle was {}",
            wrapped.rotation
        );
    }

    #[test]
    fn scale_drag_is_uniform_and_stays_positive() {
        let drag = GizmoDrag::new(
            GizmoMode::Scale,
            Pos2::new(50.0, 50.0),
            Transform::new(1.0, 2.0).with_scale(Vec2::splat(2.0)),
            [1.0, 1.0],
            Vec2::new(2.0, 3.0),
        )
        .unwrap();
        let enlarged = drag.at(Pos2::new(50.0, -50.0), None);
        assert!(enlarged.scale.x > 2.0);
        assert_eq!(enlarged.scale.x, enlarged.scale.y);
        let reduced = drag.at(Pos2::new(50.0, 500.0), None);
        assert!(reduced.scale.x > 0.0);
    }

    #[test]
    fn rotation_and_scale_handles_only_start_on_their_visible_targets() {
        let center = Pos2::new(100.0, 100.0);
        assert!(hits_handle(
            GizmoMode::Rotate,
            Pos2::new(140.0, 100.0),
            center
        ));
        assert!(!hits_handle(
            GizmoMode::Rotate,
            Pos2::new(110.0, 100.0),
            center
        ));
        assert!(hits_handle(
            GizmoMode::Scale,
            Pos2::new(154.0, 100.0),
            center
        ));
        assert!(!hits_handle(
            GizmoMode::Scale,
            Pos2::new(130.0, 100.0),
            center
        ));
    }
}
