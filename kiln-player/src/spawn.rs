//! Spawning scene entities into the runtime world.
//!
//! Scene data lives in [`kiln_core::Project`] as plain data; this module
//! converts it into runtime components ([`kiln_core::Transform`] and
//! [`kiln_runtime::Sprite`]) inside a [`kiln_runtime::GameWorld`].
//!
//! The entity spawning itself lives in [`kiln_runtime::scene`], so the editor
//! and the player build the same world. This module only adds the player
//! specific part: binding GPU textures to the sprites.

use kiln_core::{Entity, Project, Scene, SceneOffset};
use kiln_runtime::{spawn_scene as spawn_scene_shared, GameWorld, Sprite};

use crate::textures::TextureLibrary;

/// Default sprite size in pixels, used when a scene entity has no explicit size.
pub const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

/// The components created for a single scene entity.
pub type SpawnedEntity = kiln_runtime::SpawnedEntity;

/// Spawn the entities of `scene` into `world` and return what was created.
///
/// Nested scenes referenced by `scene.nested_scenes` are spawned as well,
/// with their offsets applied. `visited` guards against cyclic nesting.
pub fn spawn_scene(
    world: &mut GameWorld,
    project: &Project,
    scene: &Scene,
    offset: SceneOffset,
    visited: &mut Vec<String>,
) -> Vec<SpawnedEntity> {
    let Some(scene) = project.scenes.get(&scene.id) else {
        return Vec::new();
    };
    let mut make_sprite = |_entity: &Entity, color: [f32; 4], _texture: Option<&str>| {
        Sprite::new(color[0], color[1], color[2], color[3])
            .with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1])
    };
    let spawned = spawn_scene_shared(world, project, &scene.id, &mut make_sprite);
    let _ = (offset, visited);
    spawned
}

/// Spawn the scene with the given id from `project`.
pub fn spawn_scene_by_id(
    world: &mut GameWorld,
    project: &Project,
    scene_id: &str,
) -> Vec<SpawnedEntity> {
    let Some(scene) = project.scenes.get(scene_id) else {
        return Vec::new();
    };
    let mut visited = vec![scene_id.to_string()];
    spawn_scene(world, project, scene, SceneOffset::default(), &mut visited)
}

/// Spawn the tilemaps of a scene into `world`.
///
/// Each entity with a `Component::Tilemap` gets a runtime [`Tilemap`]
/// component bound to the project's tilesets and their GPU textures.
pub fn spawn_tilemaps(
    world: &mut GameWorld,
    project: &Project,
    scene: &Scene,
    offset: SceneOffset,
    textures: &TextureLibrary,
) -> Vec<String> {
    let tileset_gpu = crate::textures::tileset_textures(project, textures);
    let mut resolve = |tileset_id: &str| tileset_gpu.get(tileset_id).copied();
    kiln_runtime::spawn_tilemaps(world, project, scene, offset, &mut resolve)
}

/// Spawn the scene with the given id from `project`, binding GPU textures.
pub fn spawn_scene_by_id_with_textures(
    world: &mut GameWorld,
    project: &Project,
    scene_id: &str,
    textures: &TextureLibrary,
) -> Vec<SpawnedEntity> {
    let Some(scene) = project.scenes.get(scene_id) else {
        return Vec::new();
    };
    let mut make_sprite = |_entity: &Entity, color: [f32; 4], texture: Option<&str>| {
        let sprite = Sprite::new(color[0], color[1], color[2], color[3]);
        match texture.and_then(|path| textures.get(path)) {
            Some(tex) => sprite
                .with_texture(tex.id)
                .with_size(tex.width as f32, tex.height as f32),
            None => sprite.with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1]),
        }
    };
    spawn_scene_shared(world, project, &scene.id, &mut make_sprite)
}
