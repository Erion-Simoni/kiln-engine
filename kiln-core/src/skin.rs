//! Bounded world-bind linear-blend skinning data for 2D triangle meshes.
use crate::{Transform, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Invertible world-space TRS captured at binding time.
pub struct BindPose {
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
}
impl BindPose {
    /// Capture the shared runtime transform representation.
    pub fn from_transform(t: Transform) -> Self {
        Self {
            position: t.position.to_array(),
            rotation: t.rotation,
            scale: t.scale.to_array(),
        }
    }
    /// Check finite values and non-singular scale axes.
    pub fn valid(&self) -> bool {
        self.position
            .iter()
            .chain(&self.scale)
            .all(|v| v.is_finite())
            && self.rotation.is_finite()
            && self.scale.iter().all(|v| v.abs() >= 0.000001)
    }
    /// Transform a bind-local point into world space.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        (p * Vec2::from_array(self.scale)).rotate(Vec2::from_angle(self.rotation))
            + Vec2::from_array(self.position)
    }
    /// Transform a world point into this bind space.
    pub fn inverse(&self, p: Vec2) -> Vec2 {
        (p - Vec2::from_array(self.position)).rotate(Vec2::from_angle(-self.rotation))
            / Vec2::from_array(self.scale)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Stable bone identity and its captured world bind transform.
pub struct BoneBind {
    pub entity: String,
    pub pose: BindPose,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// A positive normalized contribution from one bound bone.
pub struct Influence {
    pub bone: u16,
    pub weight: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// A local bind vertex, normalized UV and up to four influences.
pub struct SkinVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub influences: Vec<Influence>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Bounded indexed triangle skin with persistent inverse-bind inputs.
pub struct SkinMesh {
    pub bind: BindPose,
    pub bones: Vec<BoneBind>,
    pub vertices: Vec<SkinVertex>,
    pub triangles: Vec<[u16; 3]>,
}
impl SkinMesh {
    /// Check finite values and non-singular scale axes.
    /// Validate topology, normalization, limits and finite bind data.
    pub fn validate(&self) -> Result<(), String> {
        if !self.bind.valid()
            || self.bones.is_empty()
            || self.bones.len() > 64
            || self.vertices.len() < 3
            || self.vertices.len() > 256
            || self.triangles.is_empty()
            || self.triangles.len() > 512
        {
            return Err(
                "Skin needs valid bind pose, 1..64 bones, 3..256 vertices and 1..512 triangles"
                    .into(),
            );
        }
        let mut names = BTreeSet::new();
        for b in &self.bones {
            if b.entity.is_empty()
                || b.entity.len() > 256
                || !b.pose.valid()
                || !names.insert(&b.entity)
            {
                return Err("Skin bones need unique IDs and invertible finite binds".into());
            }
        }
        for v in &self.vertices {
            if v.position.iter().any(|v| !v.is_finite())
                || v.uv
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || v.influences.is_empty()
                || v.influences.len() > 4
            {
                return Err("Skin vertex needs finite position/UV and 1..4 weights".into());
            }
            let mut ids = BTreeSet::new();
            let mut sum = 0.0_f64;
            for i in &v.influences {
                if i.bone as usize >= self.bones.len()
                    || !i.weight.is_finite()
                    || i.weight <= 0.0
                    || !ids.insert(i.bone)
                {
                    return Err(
                        "Skin weights need unique existing bones and positive finite weights"
                            .into(),
                    );
                }
                sum += i.weight as f64;
            }
            if (sum - 1.0).abs() > 0.0001 {
                return Err("Skin vertex weights must sum to one".into());
            }
        }
        for t in &self.triangles {
            if t.iter().any(|i| *i as usize >= self.vertices.len())
                || t[0] == t[1]
                || t[1] == t[2]
                || t[0] == t[2]
            {
                return Err("Invalid skin triangle indices".into());
            }
        }
        Ok(())
    }
    /// Check finite values and non-singular scale axes.
    /// Validate that every named bone has an authored Transform.
    pub fn validate_scene(&self, scene: &crate::Scene) -> Result<(), String> {
        self.validate()?;
        for b in &self.bones {
            if !scene.entities.iter().any(|e| {
                e.id == b.entity
                    && e.components
                        .iter()
                        .any(|c| matches!(c, crate::Component::Transform { .. }))
            }) {
                return Err(format!("Missing skin bone {}", b.entity));
            }
        }
        Ok(())
    }
    /// Skin a vertex using normalized weights, rejecting missing/nonfinite live bones.
    pub fn deform(
        &self,
        current: &std::collections::HashMap<String, Transform>,
    ) -> Result<Vec<[f32; 2]>, String> {
        self.validate()?;
        let mut matrices = vec![];
        for b in &self.bones {
            let pose = current.get(&b.entity).ok_or("Missing live skin bone")?;
            let p = BindPose::from_transform(*pose);
            if !p.valid() {
                return Err("Invalid live skin pose".into());
            }
            matrices.push(p);
        }
        let mut out = vec![];
        for vertex in &self.vertices {
            let rest = self.bind.apply(Vec2::from_array(vertex.position));
            let mut x = 0.0_f64;
            let mut y = 0.0_f64;
            let sum: f64 = vertex.influences.iter().map(|i| i.weight as f64).sum();
            for i in &vertex.influences {
                let index = i.bone as usize;
                let p = matrices[index].apply(self.bones[index].pose.inverse(rest));
                let w = i.weight as f64 / sum;
                x += p.x as f64 * w;
                y += p.y as f64 * w;
            }
            let p = [x as f32, y as f32];
            if p.iter().any(|v| !v.is_finite()) {
                return Err("Nonfinite skinned vertex".into());
            }
            out.push(p);
        }
        Ok(out)
    }
}
