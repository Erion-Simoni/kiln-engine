use crate::{
    AnimationPlayer, EntityDefinition, GameWorld, GlobalTransform, ParentLink, Role,
    SceneDrawOrder, SourceId, Sprite, SpriteResource, Tilemap, Velocity, WorldOffset,
};
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_render::TextureId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedEntity {
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    pub draw_order: u64,
    pub uv: Option<[f32; 4]>,
    pub velocity: Option<[f32; 2]>,
    pub animation: Option<crate::animation_player::AnimationSnapshot>,
    pub tilemap: Option<kiln_core::TileMap>,
    pub tile_color: Option<[f32; 4]>,
}

pub(super) fn capture(
    world: &GameWorld,
    id: &str,
    script: Option<String>,
) -> Result<(Scene, BTreeMap<String, SavedEntity>), String> {
    let ecs = world.world_ref();
    let mut scene = Scene {
        id: id.into(),
        name: id.into(),
        script,
        entities: Vec::new(),
        nested_scenes: Vec::new(),
    };
    let mut extras = BTreeMap::new();
    for entry in ecs.iter() {
        let handle = entry.entity();
        let source = ecs
            .get::<&SourceId>(handle)
            .map_err(|_| "A runtime entity has no stable SourceId")?;
        if extras.contains_key(&source.0) {
            return Err(format!("Duplicate runtime entity ID '{}'", source.0));
        }
        let mut definition = ecs
            .get::<&EntityDefinition>(handle)
            .map(|v| v.0.clone())
            .unwrap_or(Entity {
                id: source.0.clone(),
                name: source.0.clone(),
                components: Vec::new(),
            });
        definition.id = source.0.clone();
        definition.components.retain(|c| {
            !matches!(
                c,
                Component::Transform { .. }
                    | Component::Sprite { .. }
                    | Component::SpriteLayout { .. }
                    | Component::Parent { .. }
                    | Component::Role { .. }
                    | Component::Physics { .. }
                    | Component::Joint { .. }
                    | Component::Tilemap { .. }
            )
        });
        let pose = ecs
            .get::<&Transform>(handle)
            .map(|p| *p)
            .unwrap_or_default();
        definition.components.push(Component::Transform {
            x: pose.position.x,
            y: pose.position.y,
            rotation: pose.rotation,
            scale: pose.scale.x,
        });
        let sprite = ecs.get::<&Sprite>(handle).ok();
        if let Some(sprite) = &sprite {
            let texture = ecs
                .get::<&SpriteResource>(handle)
                .ok()
                .map(|v| v.0.clone())
                .or_else(|| {
                    ecs.get::<&EntityDefinition>(handle).ok().and_then(|d| {
                        d.0.components.iter().find_map(|c| {
                            if let Component::Sprite { texture, .. } = c {
                                texture.clone()
                            } else {
                                None
                            }
                        })
                    })
                });
            definition.components.push(Component::Sprite {
                color: sprite.color,
                width: sprite.size[0],
                height: sprite.size[1],
                texture,
                layer: sprite.layer,
                material: sprite.material.clone(),
                animation_clip: sprite.animation_clip.clone(),
            });
            definition.components.push(Component::SpriteLayout {
                settings: kiln_core::visual::SpriteLayout {
                    pivot: sprite.pivot,
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                    pixels_per_unit: sprite.pixels_per_unit,
                },
            });
        }
        if let Ok(role) = ecs.get::<&Role>(handle) {
            definition.components.push(Component::Role {
                role: role.0.clone(),
            });
        }
        if let Ok(parent) = ecs.get::<&ParentLink>(handle) {
            let id = ecs
                .get::<&SourceId>(parent.0)
                .map_err(|_| "Saved parent has no stable ID")?;
            definition.components.push(Component::Parent {
                parent: id.0.clone(),
            });
        }
        if let (Ok(body), Ok(collider)) = (
            ecs.get::<&crate::world::RigidBody2D>(handle),
            ecs.get::<&crate::world::Collider2D>(handle),
        ) {
            definition.components.push(Component::Physics {
                body: *body,
                collider: *collider,
            });
        }
        if let Ok(joint) = ecs.get::<&crate::world::Joint2D>(handle) {
            definition.components.push(Component::Joint {
                joint: (*joint).clone(),
            });
        }
        let tilemap = ecs.get::<&Tilemap>(handle).ok();
        if let Some(map) = &tilemap {
            definition.components.push(Component::Tilemap {
                map: map.map_id.clone(),
            });
        }
        extras.insert(
            source.0.clone(),
            SavedEntity {
                scale: pose.scale.to_array(),
                offset: ecs
                    .get::<&WorldOffset>(handle)
                    .map(|o| o.0)
                    .unwrap_or_default(),
                draw_order: ecs.get::<&SceneDrawOrder>(handle).map(|v| v.0).unwrap_or(0),
                uv: sprite.as_ref().map(|s| s.uv_rect),
                velocity: ecs.get::<&Velocity>(handle).ok().map(|v| v.0),
                animation: ecs
                    .get::<&AnimationPlayer>(handle)
                    .ok()
                    .map(|a| a.snapshot())
                    .transpose()?,
                tilemap: tilemap.as_ref().map(|m| m.map.clone()),
                tile_color: tilemap.as_ref().map(|m| m.color),
            },
        );
        scene.entities.push(definition);
    }
    scene.entities.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((scene, extras))
}

pub(super) fn validate(
    scene: &Scene,
    extras: &BTreeMap<String, SavedEntity>,
    project: &Project,
) -> Result<(), String> {
    if scene.entities.len() > 16384
        || extras.len() != scene.entities.len()
        || !scene.nested_scenes.is_empty()
    {
        return Err("Invalid saved entity count or nested scene".into());
    }
    let mut ids = HashSet::new();
    for entity in &scene.entities {
        if entity.id.is_empty()
            || entity.id.len() > 256
            || !ids.insert(&entity.id)
            || entity.components.len() > 64
        {
            return Err("Invalid/duplicate saved entity ID or component count".into());
        }
        let extra = extras.get(&entity.id).ok_or("Missing saved entity data")?;
        let finite = extra
            .scale
            .iter()
            .chain(&extra.offset)
            .chain(extra.uv.iter().flatten())
            .chain(extra.velocity.iter().flatten())
            .chain(extra.tile_color.iter().flatten())
            .all(|v| v.is_finite());
        if !finite || extra.scale.iter().any(|v| v.abs() < 0.000001) {
            return Err("Invalid saved entity geometry".into());
        }
        if let Some(map) = &extra.tilemap {
            if !map.tile_size.iter().all(|v| v.is_finite() && *v > 0.0) || map.layers.len() > 256 {
                return Err("Invalid saved tilemap geometry".into());
            }
            for layer in &map.layers {
                if !project.tilesets.contains_key(&layer.tileset)
                    || layer.offset.iter().any(|v| !v.is_finite())
                    || layer.chunks.len() > 16384
                {
                    return Err("Invalid saved tilemap layer or tileset".into());
                }
                for (key, chunk) in &layer.chunks {
                    if *key != kiln_core::TileMapLayer::chunk_key(chunk.x, chunk.y)
                        || chunk.tiles.len()
                            != (kiln_core::CHUNK_SIZE * kiln_core::CHUNK_SIZE) as usize
                    {
                        return Err("Invalid saved tilemap chunk".into());
                    }
                }
            }
        }
        let mut kinds = HashSet::new();
        for component in &entity.components {
            if !kinds.insert(std::mem::discriminant(component)) {
                return Err("Duplicate saved component".into());
            }
            match component {
                Component::Transform {
                    x,
                    y,
                    rotation,
                    scale,
                } if ![x, y, rotation, scale].iter().all(|v| v.is_finite()) => {
                    return Err("Non-finite saved transform".into())
                }
                Component::Sprite {
                    width,
                    height,
                    color,
                    layer,
                    texture,
                    ..
                } => {
                    if !width.is_finite()
                        || !height.is_finite()
                        || *width <= 0.0
                        || *height <= 0.0
                        || !layer.is_finite()
                        || color.iter().any(|v| !v.is_finite())
                    {
                        return Err("Invalid saved sprite".into());
                    }
                    if let Some(path) = texture {
                        resource_path(path)?;
                    }
                }
                Component::SpriteLayout { settings } => settings.validate()?,
                Component::Physics { body, collider } => {
                    kiln_core::physics_data::validate_body(body, collider)?
                }
                Component::Joint { joint } => kiln_core::physics_data::validate_joint(joint)?,
                Component::Particles { effect } => effect.validate()?,
                Component::Script { path, properties } => {
                    resource_path(path)?;
                    if properties.len() > 4096 || properties.values().any(|v| !v.is_finite()) {
                        return Err("Invalid saved script properties".into());
                    }
                }
                Component::Tilemap { map: tilemap_id }
                    if !project.tilemaps.contains_key(tilemap_id) =>
                {
                    return Err("Saved tilemap resource is missing".into())
                }
                _ => {}
            }
        }
    }
    kiln_core::hierarchy_transform::world_transforms(scene)?;
    for entity in &scene.entities {
        for c in &entity.components {
            if let Component::Joint { joint } = c {
                if !ids.contains(&joint.entity_a) || !ids.contains(&joint.entity_b) {
                    return Err("Saved joint references a missing entity".into());
                }
            }
        }
    }
    Ok(())
}

pub(super) fn resource_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 1024
        || std::path::Path::new(path)
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        return Err("Invalid saved resource path".into());
    }
    Ok(())
}

pub(super) fn restore(
    scene: &Scene,
    extras: &BTreeMap<String, SavedEntity>,
    project: &Project,
    mut resolve: impl FnMut(&str) -> Option<(TextureId, [u32; 2])>,
) -> Result<GameWorld, String> {
    let mut resources = HashMap::new();
    let mut paths = Vec::new();
    for entity in &scene.entities {
        for c in &entity.components {
            if let Component::Sprite {
                texture: Some(path),
                ..
            } = c
            {
                paths.push(path.clone());
            }
        }
    }
    for extra in extras.values() {
        if let Some(animation) = &extra.animation {
            paths.extend(animation.clips.values().map(|c| c.texture.clone()));
        }
    }
    for tileset in project.tilesets.values() {
        paths.push(tileset.texture.clone());
    }
    for path in paths {
        resource_path(&path)?;
        if let std::collections::hash_map::Entry::Vacant(entry) = resources.entry(path) {
            let texture = resolve(entry.key())
                .ok_or_else(|| format!("Saved texture resource is missing: {}", entry.key()))?;
            entry.insert(texture);
        }
    }
    let mut world = GameWorld::new();
    crate::scene::spawn_entities(
        &mut world,
        &scene.entities,
        kiln_core::SceneOffset::default(),
        &mut |_, color, path| {
            let mut sprite = Sprite::new(color[0], color[1], color[2], color[3]);
            if let Some(texture) = path.and_then(|p| resources.get(p)) {
                sprite.texture = texture.0;
            }
            sprite
        },
        project,
    );
    let mut tile_resolve = |id: &str| {
        project
            .tilesets
            .get(id)
            .and_then(|s| resources.get(&s.texture))
            .map(|v| v.0)
    };
    crate::spawn_tilemaps(
        &mut world,
        project,
        scene,
        kiln_core::SceneOffset::default(),
        &mut tile_resolve,
    );
    let handles: Vec<_> = world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .map(|(e, id)| (e, id.0.clone()))
        .collect();
    for (handle, id) in handles {
        let extra = &extras[&id];
        if let Ok(mut pose) = world.world_mut().get::<&mut Transform>(handle) {
            pose.scale = kiln_core::Vec2::from_array(extra.scale);
        }
        world
            .world_mut()
            .insert(
                handle,
                (WorldOffset(extra.offset), SceneDrawOrder(extra.draw_order)),
            )
            .map_err(|e| e.to_string())?;
        if let Some(uv) = extra.uv {
            if let Ok(mut s) = world.world_mut().get::<&mut Sprite>(handle) {
                s.uv_rect = uv;
            }
        }
        if let Some(v) = extra.velocity {
            world
                .world_mut()
                .insert_one(handle, Velocity(v))
                .map_err(|e| e.to_string())?;
        }
        if let Some(saved) = &extra.animation {
            let animation =
                AnimationPlayer::restore_snapshot(saved, |path| resources.get(path).copied())?;
            world
                .world_mut()
                .insert_one(handle, animation)
                .map_err(|e| e.to_string())?;
        }
        if let Some(map) = &extra.tilemap {
            if let Ok(mut target) = world.world_mut().get::<&mut Tilemap>(handle) {
                target.map = map.clone();
                target.color = extra.tile_color.unwrap_or([1.0; 4]);
            }
        }
    }
    crate::update_global_transforms(&mut world)?;
    for (_, map, pose) in world
        .world_mut()
        .query_mut::<(hecs::Entity, &mut Tilemap, &GlobalTransform)>()
    {
        map.offset = pose.position.to_array();
    }
    Ok(world)
}
