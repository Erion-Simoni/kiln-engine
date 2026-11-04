//! Axis-aligned solid tile geometry shared with rendering/grid movement queries.
use crate::physics::PhysicsWorld;
use crate::{GameWorld, RigidBody2D, SourceId, Tilemap};
use kiln_core::{TileMap, TileSet, CHUNK_SIZE};
use rapier2d::prelude::*;
use std::collections::{HashMap, HashSet};

pub(crate) struct TrackedTilemap {
    handle: RigidBodyHandle,
    map: TileMap,
    tilesets: HashMap<String, TileSet>,
    offset: [f32; 2],
}

pub(crate) fn synchronize(physics: &mut PhysicsWorld, world: &GameWorld) {
    let maps: Vec<_> = world
        .world_ref()
        .query::<(
            hecs::Entity,
            &Tilemap,
            Option<&SourceId>,
            Option<&RigidBody2D>,
        )>()
        .iter()
        .filter(|(_, _, _, body)| body.is_none())
        .map(|(entity, map, id, _)| {
            (
                entity,
                map.clone(),
                id.map(|id| id.0.clone())
                    .unwrap_or_else(|| format!("{entity:?}")),
            )
        })
        .collect();
    let live: HashSet<_> = maps.iter().map(|(entity, _, _)| *entity).collect();
    let stale: Vec<_> = physics
        .tilemaps
        .keys()
        .filter(|entity| !live.contains(entity))
        .copied()
        .collect();
    for entity in stale {
        remove(physics, entity);
    }
    for (entity, map, source) in maps {
        if !map
            .map
            .tile_size
            .iter()
            .all(|size| size.is_finite() && *size > 0.0)
            || !map.offset.iter().all(|v| v.is_finite())
        {
            remove(physics, entity);
            continue;
        }
        let rebuild = physics.tilemaps.get(&entity).is_none_or(|tracked| {
            tracked.map != map.map
                || tracked.tilesets != map.tilesets
                || physics.rigid_body_set.get(tracked.handle).is_none()
        });
        if rebuild {
            remove(physics, entity);
            let shape = solid_shapes(&map);
            if shape.is_empty() {
                continue;
            }
            let handle = physics.create_rigid_body(
                RigidBodyBuilder::fixed()
                    .translation(rapier2d::math::Vector::from_array(map.offset)),
            );
            physics.create_collider(
                ColliderBuilder::compound(shape)
                    .friction(0.8)
                    .collision_groups(InteractionGroups::new(
                        Group::GROUP_1,
                        Group::ALL,
                        InteractionTestMode::And,
                    )),
                handle,
            );
            physics.source_ids.insert(handle, source);
            physics.tilemaps.insert(
                entity,
                TrackedTilemap {
                    handle,
                    map: map.map.clone(),
                    tilesets: map.tilesets.clone(),
                    offset: map.offset,
                },
            );
        }
        if let Some(tracked) = physics.tilemaps.get_mut(&entity) {
            if tracked.offset != map.offset {
                if let Some(body) = physics.rigid_body_set.get_mut(tracked.handle) {
                    body.set_translation(rapier2d::math::Vector::from_array(map.offset), true);
                }
                tracked.offset = map.offset;
            }
        }
    }
}

fn remove(physics: &mut PhysicsWorld, entity: hecs::Entity) {
    if let Some(tracked) = physics.tilemaps.remove(&entity) {
        physics.source_ids.remove(&tracked.handle);
        physics.remove_body(tracked.handle);
    }
}

fn solid_shapes(map: &Tilemap) -> Vec<(rapier2d::math::Pose, SharedShape)> {
    let [width, height] = map.map.tile_size;
    let mut shapes = Vec::new();
    for layer in &map.map.layers {
        let Some(tileset) = map.tilesets.get(&layer.tileset) else {
            continue;
        };
        if !layer.offset.iter().all(|v| v.is_finite()) {
            continue;
        }
        // Merge contiguous solid cells in each chunk row into one rectangle.
        let mut chunks: Vec<_> = layer.chunks.values().collect();
        chunks.sort_by_key(|chunk| (chunk.y, chunk.x));
        for chunk in chunks {
            for row in 0..CHUNK_SIZE {
                let mut col = 0;
                while col < CHUNK_SIZE {
                    if !tileset.is_solid(chunk.tile(col, row)) {
                        col += 1;
                        continue;
                    }
                    let start = col;
                    while col < CHUNK_SIZE && tileset.is_solid(chunk.tile(col, row)) {
                        col += 1;
                    }
                    let length = (col - start) as f32 * width;
                    let x = layer.offset[0]
                        + (chunk.x as f32 * CHUNK_SIZE as f32 + start as f32) * width
                        + length * 0.5;
                    let y = layer.offset[1]
                        + (chunk.y as f32 * CHUNK_SIZE as f32 + row as f32) * height
                        + height * 0.5;
                    shapes.push((
                        rapier2d::math::Pose::new(rapier2d::math::Vector::new(x, y), 0.0),
                        SharedShape::cuboid(length * 0.5, height * 0.5),
                    ));
                }
            }
        }
    }
    shapes
}
