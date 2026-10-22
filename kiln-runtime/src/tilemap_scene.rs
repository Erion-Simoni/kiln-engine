//! Spawning the tilemaps of a scene into the runtime world.
//!
//! A scene entity with a [`Component::Tilemap`] refers to a tilemap in
//! [`Project::tilemaps`]. This module turns that reference into a runtime
//! [`Tilemap`] component inside a [`GameWorld`], so the editor and the player
//! build the same world.
//!
//! The GPU textures are resolved by the caller, because only the host that
//! owns a device can upload them.

use kiln_core::project::{Component, Entity, Project, Scene, SceneOffset};
use kiln_core::Transform;
use kiln_render::TextureId;

use crate::tilemap::Tilemap;
use crate::world::{GameWorld, SourceId};

/// Resolves the GPU texture of a tileset by its id.
///
/// Returns `None` when the tileset has no texture uploaded, e.g. because the
/// asset is missing.
pub type TilesetTextureResolver<'a> = dyn FnMut(&str) -> Option<TextureId> + 'a;

/// Spawn the tilemaps of a scene into `world`.
///
/// Every entity with a [`Component::Tilemap`] gets a [`Tilemap`] bound to the
/// project's tilesets and the textures the resolver returns. The returned
/// vector holds the ids of the maps that were spawned.
///
/// Entities whose map is not in the project are skipped.
pub fn spawn_tilemaps(
    world: &mut GameWorld,
    project: &Project,
    scene: &Scene,
    offset: SceneOffset,
    resolve_texture: &mut TilesetTextureResolver<'_>,
) -> Vec<String> {
    let mut spawned = Vec::new();

    for entity in &scene.entities {
        let Some(map_id) = entity_tilemap_id(entity) else {
            continue;
        };
        let Some(map) = project.tilemaps.get(&map_id) else {
            log::warn!("tilemap '{map_id}' not found in project");
            continue;
        };

        let transform = entity_transform(entity);
        let mut textures = std::collections::HashMap::new();
        for tileset_id in project.tilesets.keys() {
            if let Some(texture) = resolve_texture(tileset_id) {
                textures.insert(tileset_id.clone(), texture);
            }
        }

        let mut tilemap = Tilemap::new(
            map_id.clone(),
            map.clone(),
            project.tilesets.clone(),
            textures,
        );
        tilemap.offset = [
            transform.position.x + offset.x,
            transform.position.y + offset.y,
        ];

        // The offset belongs on the transform too, so the entity sits where it
        // is drawn even when the scene is nested.
        let world_transform = Transform::new(
            transform.position.x + offset.x,
            transform.position.y + offset.y,
        )
        .with_rotation(transform.rotation)
        .with_scale(transform.scale);

        world.spawn((world_transform, tilemap, SourceId::new(entity.id.clone())));
        spawned.push(map_id);
    }

    spawned
}

/// Read the transform of a scene entity, defaulting to the origin.
fn entity_transform(entity: &Entity) -> Transform {
    for component in &entity.components {
        if let Component::Transform {
            x,
            y,
            rotation,
            scale,
        } = component
        {
            return Transform::new(*x, *y)
                .with_rotation(*rotation)
                .with_scale(kiln_core::Vec2::splat(*scale));
        }
    }
    Transform::default()
}

/// Read the tilemap id of a scene entity, if it has one.
fn entity_tilemap_id(entity: &Entity) -> Option<String> {
    for component in &entity.components {
        if let Component::Tilemap { map } = component {
            return Some(map.clone());
        }
    }
    None
}
