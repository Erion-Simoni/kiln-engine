//! Collision against the solid tiles of the tilemaps in a world.
//!
//! A tilemap knows which of its tiles are solid (the `solid` flag of the
//! tileset). This module turns that into movement for the entities that walk
//! on the map: a move is split into its two axes and each axis is taken only
//! when the destination is free, so an entity slides along a wall instead of
//! sticking to it.

use crate::tilemap::Tilemap;
use crate::world::GameWorld;

/// The world-space box of an entity, as `[min_x, min_y, max_x, max_y]`.
///
/// The box is centred on the entity's position and as big as its sprite.
pub fn entity_box(position: [f32; 2], size: [f32; 2]) -> [f32; 4] {
    let half_x = size[0].abs() * 0.5;
    let half_y = size[1].abs() * 0.5;
    [
        position[0] - half_x,
        position[1] - half_y,
        position[0] + half_x,
        position[1] + half_y,
    ]
}

/// Whether the given world-space box overlaps a solid tile of any tilemap.
///
/// Returns the world position of the first solid tile found.
pub fn first_solid_hit(world: &GameWorld, rect: [f32; 4]) -> Option<[f32; 2]> {
    let mut hit = None;
    let mut query = world
        .world_ref()
        .query::<(&crate::world::SourceId, &Tilemap)>();
    for (_, tilemap) in query.iter() {
        if let Some(found) = tilemap.first_solid_hit(rect) {
            if hit.is_none() {
                hit = Some(found);
            }
        }
    }
    hit
}

/// Move a box by `delta`, stopping at the solid tiles it runs into.
///
/// The two axes are handled separately: the horizontal move is taken first,
/// and the vertical move is only taken when the entity is still free after
/// it. An entity that walks into a wall therefore slides along it.
///
/// Returns the new position of the centre of the box.
pub fn move_with_collision(
    world: &GameWorld,
    position: [f32; 2],
    size: [f32; 2],
    delta: [f32; 2],
) -> [f32; 2] {
    let mut result = position;

    // Horizontal axis first.
    let moved_x = result[0] + delta[0];
    let box_x = entity_box([moved_x, result[1]], size);
    if first_solid_hit(world, box_x).is_none() {
        result[0] = moved_x;
    }

    // Vertical axis, using the position the horizontal move ended at.
    let moved_y = result[1] + delta[1];
    let box_y = entity_box([result[0], moved_y], size);
    if first_solid_hit(world, box_y).is_none() {
        result[1] = moved_y;
    }

    result
}
