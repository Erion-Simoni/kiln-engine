//! Draw-only preview guard: authoring and save paths never observe sampled poses.
use kiln_core::Transform;
use kiln_runtime::{GameWorld, Sprite};
/// Restores local properties and resolved hierarchy when rendering leaves scope.
pub struct ScenePreview<'a> {
    pub world: &'a mut GameWorld,
    original: Vec<(hecs::Entity, Option<Transform>, Option<[f32; 4]>)>,
}
impl<'a> ScenePreview<'a> {
    pub fn begin(
        world: &'a mut GameWorld,
        doc: &crate::cutscene::Cutscene,
        time: f64,
    ) -> Result<Self, String> {
        doc.validate()?;
        if !time.is_finite() {
            return Err("Preview time must be finite".into());
        }
        let mut original = vec![];
        for track in &doc.tracks {
            let h = kiln_runtime::tweens::handle(world, &track.entity)
                .ok_or("Missing preview entity")?;
            if kiln_runtime::tweens::read_handle(world, h, track.property).is_none() {
                return Err("Ineligible preview binding".into());
            }
            if !original.iter().any(|(id, _, _)| *id == h) {
                original.push((
                    h,
                    world.world_ref().get::<&Transform>(h).ok().map(|p| *p),
                    world.world_ref().get::<&Sprite>(h).ok().map(|s| s.color),
                ));
            }
        }
        for id in doc.ik.iter().flat_map(|ik| ik.bones.iter()) {
            let h = kiln_runtime::tweens::handle(world, id).ok_or("Missing preview bone")?;
            if !original.iter().any(|(id, _, _)| *id == h) {
                original.push((
                    h,
                    world.world_ref().get::<&Transform>(h).ok().map(|p| *p),
                    None,
                ));
            }
        }
        let preview = Self { world, original };
        for track in &doc.tracks {
            let h = kiln_runtime::tweens::handle(preview.world, &track.entity)
                .ok_or("Missing preview binding")?;
            let values = crate::cutscene::sample(track, time)
                .iter()
                .map(|v| *v as f32)
                .collect::<Vec<_>>();
            if !kiln_runtime::tweens::write_handle(preview.world, h, track.property, &values) {
                return Err("Invalid preview value".into());
            }
        }
        for ik in &doc.ik {
            let target = kiln_runtime::tweens::read_property(
                preview.world,
                &ik.target,
                kiln_runtime::tweens::TweenProperty::Position,
            )
            .ok_or("Missing preview IK target")?;
            kiln_runtime::rig::solve(
                preview.world,
                &kiln_runtime::rig::IkRequest {
                    bones: ik.bones.clone(),
                    target: [target[0], target[1]],
                    bend: ik.bend,
                    weight: ik.weight,
                },
            )?;
        }
        kiln_runtime::update_global_transforms(preview.world)?;
        Ok(preview)
    }
}
impl Drop for ScenePreview<'_> {
    fn drop(&mut self) {
        for (h, pose, color) in &self.original {
            if let Some(p) = pose {
                if let Ok(mut current) = self.world.world_mut().get::<&mut Transform>(*h) {
                    *current = *p;
                }
            }
            if let Some(c) = color {
                if let Ok(mut sprite) = self.world.world_mut().get::<&mut Sprite>(*h) {
                    sprite.color = *c;
                }
            }
        }
        let _ = kiln_runtime::update_global_transforms(self.world);
    }
}

/// Draw sampled poses if valid, otherwise retain the authored world.
pub fn collect<T>(
    world: &mut GameWorld,
    preview: Option<(&crate::cutscene::Cutscene, f64)>,
    draw: impl FnOnce(&GameWorld) -> T,
) -> T {
    if let Some((doc, time)) = preview {
        // Prevalidate before borrowing for the restoring guard.
        let valid = doc.validate().is_ok()
            && time.is_finite()
            && doc.tracks.iter().all(|t| {
                kiln_runtime::tweens::read_property(world, &t.entity, t.property).is_some()
            });
        if valid {
            if let Ok(guard) = ScenePreview::begin(world, doc, time) {
                return draw(guard.world);
            }
        }
    }
    draw(world)
}
