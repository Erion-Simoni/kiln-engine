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
    let mut draws = Vec::new();
    for (entity, source, pose, sprite) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &GlobalTransform, &Sprite)>()
        .iter()
    {
        let order = world
            .world_ref()
            .get::<&crate::SceneDrawOrder>(entity)
            .map(|order| order.0)
            .unwrap_or(entity.id() as u64);
        let id = source
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"));
        let mut instance = sprite.draw_instance(pose.0);
        if let Some(name) = &sprite.material {
            if let Some(id) = resolve(name) {
                instance.material_id = id;
            } else {
                instance.color = [1.0, 0.0, 1.0, 1.0];
            }
        }
        draws.push((sprite.layer, order, id, 0, sprite.texture, instance));
    }
    for (entity, source, map) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &Tilemap)>()
        .iter()
    {
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
            draws.push((layer, order, id.clone(), index, texture, instance));
        }
    }
    for (entity, source, effect) in world
        .world_ref()
        .query::<(hecs::Entity, Option<&SourceId>, &ParticleSystemComponent)>()
        .iter()
    {
        let order = world
            .world_ref()
            .get::<&crate::SceneDrawOrder>(entity)
            .map(|order| order.0)
            .unwrap_or(entity.id() as u64);
        let id = source
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"));
        for (index, particle) in effect.system().particles().iter().enumerate() {
            draws.push((
                effect.layer,
                order,
                id.clone(),
                index,
                TextureId::WHITE,
                SpriteInstance {
                    position: particle.position,
                    size: [particle.size; 2],
                    color: particle.color,
                    uv_rect: FULL_UV,
                    material_id: 0,
                    rotation: 0.0,
                },
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
