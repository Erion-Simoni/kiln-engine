//! Shared scene draw collection and deterministic painter ordering.
use crate::sprite::ParticleSystemComponent;
use crate::{GameWorld, GlobalTransform, SourceId, Sprite, Tilemap};
use kiln_render::{SpriteInstance, TextureId, FULL_UV};

/// Collect tiles, sprites and particles in one sorting-layer order. Stable
/// authored order resolves equal layers identically in editor and player.
pub fn collect_scene_draws(world: &GameWorld, view: [f32; 4]) -> Vec<(TextureId, SpriteInstance)> {
    collect_scene_draws_with_materials(world, view, |_| None)
}
/// Resolve material IDs per host without changing painter order or draw geometry.
pub fn collect_scene_draws_with_materials(
    world: &GameWorld,
    view: [f32; 4],
    resolve: impl Fn(&str) -> Option<u32>,
) -> Vec<(TextureId, SpriteInstance)> {
    collect_scene_draws_filtered(world, view, resolve, |_| true)
}

/// Collect the same geometry while a host filters source IDs (e.g. editor hide).
pub fn collect_scene_draws_filtered(
    world: &GameWorld,
    view: [f32; 4],
    resolve: impl Fn(&str) -> Option<u32>,
    include: impl Fn(Option<&str>) -> bool,
) -> Vec<(TextureId, SpriteInstance)> {
    collect_scene_draws_filtered_mode(world, view, resolve, include, true)
}

/// Authoring geometry stays in world coordinates for selection and gizmos.
/// Play/gameplay uses the normal collector with camera-relative parallax.
pub fn collect_authoring_draws_filtered(
    world: &GameWorld,
    view: [f32; 4],
    resolve: impl Fn(&str) -> Option<u32>,
    include: impl Fn(Option<&str>) -> bool,
) -> Vec<(TextureId, SpriteInstance)> {
    collect_scene_draws_filtered_mode(world, view, resolve, include, false)
}

/// Select gameplay parallax or logical authoring geometry while retaining all filters.
pub fn collect_scene_draws_filtered_mode(
    world: &GameWorld,
    view: [f32; 4],
    resolve: impl Fn(&str) -> Option<u32>,
    include: impl Fn(Option<&str>) -> bool,
    parallax: bool,
) -> Vec<(TextureId, SpriteInstance)> {
    if !view.iter().all(|value| value.is_finite()) || view[0] >= view[2] || view[1] >= view[3] {
        return Vec::new();
    }
    let mut draws = Vec::new();
    let bone_poses: std::collections::HashMap<_, _> = world
        .world_ref()
        .query::<(&SourceId, &GlobalTransform)>()
        .iter()
        .map(|(id, p)| (id.0.clone(), p.0))
        .collect();
    for (entity, source, pose, sprite) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &GlobalTransform, &Sprite)>()
        .iter()
    {
        if !include(source.map(|id| id.0.as_str())) {
            continue;
        }
        let order = world
            .world_ref()
            .get::<&crate::SceneDrawOrder>(entity)
            .map(|order| order.0)
            .unwrap_or(entity.id() as u64);
        let id = source
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"));
        let mut instance = if parallax {
            let Some(instance) = sprite.parallax_instance(pose.0, [view[0], view[1]]) else {
                continue;
            };
            instance
        } else {
            sprite.draw_instance(pose.0)
        };
        if let Some(name) = &sprite.material {
            if let Some(id) = resolve(name) {
                instance.material_id = id;
            } else {
                instance.color = [1.0, 0.0, 1.0, 1.0];
            }
        }
        if let Ok(mesh) = world.world_ref().get::<&kiln_core::skin::SkinMesh>(entity) {
            if let Ok(points) = mesh.deform(&bone_poses) {
                let shift = if parallax {
                    [
                        instance.position[0] - sprite.draw_instance(pose.0).position[0],
                        instance.position[1] - sprite.draw_instance(pose.0).position[1],
                    ]
                } else {
                    [0.0; 2]
                };
                for (index, t) in mesh.triangles.iter().enumerate() {
                    let p = t.map(|i| {
                        [
                            points[i as usize][0] + shift[0],
                            points[i as usize][1] + shift[1],
                        ]
                    });
                    let uv = t.map(|i| mesh.vertices[i as usize].uv);
                    let mut tri = SpriteInstance::triangle(p, uv, instance.color);
                    tri.uv_rect = instance.uv_rect;
                    tri.material_id = instance.material_id;
                    tri.blend_uv = instance.blend_uv;
                    tri.blend_weight = instance.blend_weight;
                    tri.secondary_texture = instance.secondary_texture;
                    if tri.intersects_view(view) {
                        draws.push((sprite.layer, order, id.clone(), index, sprite.texture, tri));
                    }
                }
            }
        } else if instance.intersects_view(view) {
            draws.push((sprite.layer, order, id, 0, sprite.texture, instance));
        }
    }
    for (entity, source, map) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &Tilemap)>()
        .iter()
    {
        if !include(source.map(|id| id.0.as_str())) {
            continue;
        }
        let order = world
            .world_ref()
            .get::<&crate::SceneDrawOrder>(entity)
            .map(|order| order.0)
            .unwrap_or(entity.id() as u64);
        let id = source
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"));
        for (index, (layer, texture, instance)) in map.collect_layered(view).into_iter().enumerate()
        {
            if instance.intersects_view(view) {
                draws.push((layer, order, id.clone(), index, texture, instance));
            }
        }
    }
    for (entity, source, effect) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &ParticleSystemComponent)>()
        .iter()
    {
        if !include(source.map(|id| id.0.as_str())) {
            continue;
        }
        let order = world
            .world_ref()
            .get::<&crate::SceneDrawOrder>(entity)
            .map(|order| order.0)
            .unwrap_or(entity.id() as u64);
        let id = source
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"));
        for (index, particle) in effect.system().particles().iter().enumerate() {
            let instance = SpriteInstance {
                position: particle.position,
                size: [particle.size; 2],
                color: particle.color,
                uv_rect: FULL_UV,
                material_id: 0,
                rotation: 0.0,
                ..kiln_render::SpriteInstance::colored([0.0; 2], [1.0; 2], [1.0; 4])
            };
            if !instance.intersects_view(view) {
                continue;
            }
            draws.push((
                effect.layer,
                order,
                id.clone(),
                index,
                TextureId::WHITE,
                instance,
            ));
        }
    }
    draws.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    draws
        .into_iter()
        .map(|(_, _, _, _, texture, instance)| (texture, instance))
        .collect()
}
