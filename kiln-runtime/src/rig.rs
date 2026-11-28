//! Cutout skeletal two-link IK over the shared local-transform hierarchy.
use crate::{GameWorld, GlobalTransform, ParentLink};
use kiln_core::{physics_data::RigidBody2D, Transform};
/// Callback-atomic IK request. Entity names identify upper, lower and endpoint bones.
#[derive(Debug, Clone, PartialEq)]
pub struct IkRequest {
    pub bones: [String; 3],
    pub target: [f64; 2],
    pub bend: f64,
    pub weight: f64,
}
impl IkRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.bones.iter().any(|id| id.is_empty() || id.len() > 256)
            || self.bones[0] == self.bones[1]
            || self.bones[1] == self.bones[2]
            || self.bones[0] == self.bones[2]
            || self
                .target
                .iter()
                .any(|v| !v.is_finite() || v.abs() > f32::MAX as f64)
            || ![-1.0, 1.0].contains(&self.bend)
            || !self.weight.is_finite()
            || !(0.0..=1.0).contains(&self.weight)
        {
            return Err(
                "IK needs three distinct bones, finite target, bend -1/1 and weight 0..1".into(),
            );
        }
        Ok(())
    }
}
fn uniform(scale: kiln_core::Vec2) -> bool {
    scale.is_finite() && scale.x > 0.000001 && (scale.x - scale.y).abs() <= scale.x * 0.00001
}
fn mixed_angle(from: f32, to: f64, weight: f64) -> f32 {
    if weight == 0.0 {
        return from;
    }
    let delta = (to - f64::from(from) + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    (f64::from(from) + delta * weight) as f32
}
/// Solve only validated +X rest chains with positive uniform scales; failure writes nothing.
pub fn solve(world: &mut GameWorld, request: &IkRequest) -> Result<(), String> {
    request.validate()?;
    crate::update_global_transforms(world)?;
    let mut handles = vec![];
    let mut locals = vec![];
    for id in &request.bones {
        let h = crate::tweens::handle(world, id).ok_or("Missing IK bone")?;
        if world.world_ref().get::<&RigidBody2D>(h).is_ok() {
            return Err("Physics owns IK bone".into());
        }
        let pose = *world
            .world_ref()
            .get::<&Transform>(h)
            .map_err(|_| "IK bone needs Transform")?;
        if !pose.position.is_finite() || !pose.rotation.is_finite() || !uniform(pose.scale) {
            return Err("IK needs finite poses and positive uniform bone scale".into());
        }
        handles.push(h);
        locals.push(pose);
    }
    for i in 1..3 {
        if world
            .world_ref()
            .get::<&ParentLink>(handles[i])
            .map(|p| p.0)
            .ok()
            != Some(handles[i - 1])
            || locals[i].position.x <= 0.000001
            || locals[i].position.y != 0.0
        {
            return Err("IK needs directly parented +X rest links".into());
        }
    }
    // Reject nonuniform ancestry instead of solving an unsupported sheared space.
    let mut parent = handles[0];
    let mut depth = 0;
    loop {
        if let Ok(pose) = world.world_ref().get::<&Transform>(parent) {
            if !uniform(pose.scale) {
                return Err("IK ancestry needs positive uniform scales".into());
            }
        }
        let next = world
            .world_ref()
            .get::<&ParentLink>(parent)
            .ok()
            .map(|p| p.0);
        match next {
            Some(p) => {
                parent = p;
                depth += 1;
                if depth > 256 {
                    return Err("IK hierarchy too deep".into());
                }
            }
            None => break,
        }
    }
    let upper = world
        .world_ref()
        .get::<&GlobalTransform>(handles[0])
        .map_err(|_| "Missing IK world pose")?
        .0;
    let lower = world
        .world_ref()
        .get::<&GlobalTransform>(handles[1])
        .map_err(|_| "Missing IK world pose")?
        .0;
    if !uniform(upper.scale) || !uniform(lower.scale) {
        return Err("IK world scale must be uniform".into());
    }
    let l1 = f64::from(locals[1].position.x) * f64::from(upper.scale.x);
    let l2 = f64::from(locals[2].position.x) * f64::from(lower.scale.x);
    let x = request.target[0] - f64::from(upper.position.x);
    let y = request.target[1] - f64::from(upper.position.y);
    let cosine = ((x * x + y * y - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
    let elbow = cosine.acos() * request.bend;
    let angle = y.atan2(x) - (l2 * elbow.sin()).atan2(l1 + l2 * elbow.cos());
    let ancestor = f64::from(upper.rotation) - f64::from(locals[0].rotation);
    let root_angle = mixed_angle(locals[0].rotation, angle - ancestor, request.weight);
    let lower_angle = mixed_angle(locals[1].rotation, elbow, request.weight);
    if !root_angle.is_finite() || !lower_angle.is_finite() {
        return Err("Invalid solved IK pose".into());
    }
    world
        .world_mut()
        .get::<&mut Transform>(handles[0])
        .map_err(|_| "Missing IK root")?
        .rotation = root_angle;
    world
        .world_mut()
        .get::<&mut Transform>(handles[1])
        .map_err(|_| "Missing IK lower")?
        .rotation = lower_angle;
    if let Err(error) = crate::update_global_transforms(world) {
        world
            .world_mut()
            .get::<&mut Transform>(handles[0])
            .map_err(|_| "Missing IK root")?
            .rotation = locals[0].rotation;
        world
            .world_mut()
            .get::<&mut Transform>(handles[1])
            .map_err(|_| "Missing IK lower")?
            .rotation = locals[1].rotation;
        let _ = crate::update_global_transforms(world);
        return Err(error);
    }
    Ok(())
}
