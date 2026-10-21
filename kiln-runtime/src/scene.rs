//! Spawning project scenes into a runtime world.
//!
//! Both the standalone player and the editor build their world from the same
//! project data, so a scene looks the same in both. Textures are resolved by
//! the caller through [`SpriteFactory`], because only the player has a GPU to
//! upload them to.

use kiln_core::project::{Component, Entity, Project, Scene, SceneOffset};
use kiln_core::Transform;

use crate::world::{GameWorld, Role, SourceId};

/// Default sprite size in pixels, used when an entity has no texture.
pub const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

/// Creates the runtime sprite for a scene entity.
///
/// The factory receives the entity's tint and the asset path of its texture
/// (`None` when the entity has no texture) and returns the sprite to spawn.
pub type SpriteFactory<'a> = dyn FnMut(&Entity, [f32; 4], Option<&str>) -> crate::sprite::Sprite + 'a;

/// A scene entity that was spawned into the world.
#[derive(Debug, Clone)]
pub struct SpawnedEntity {
    /// Id of the entity in the project data.
    pub source_id: String,
    /// Position in world space.
    pub position: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Uniform scale.
    pub scale: kiln_core::Vec2,
    /// Tint colour, when the entity has a sprite.
    pub color: Option<[f32; 4]>,
    /// Asset path of the sprite texture, when the entity has one.
    pub texture: Option<String>,
    /// Gameplay role of the entity, when it has one.
    pub role: Option<String>,
}

/// Spawn the scene with the given id from `project`.
///
/// Nested scenes are spawned as well, with their offsets applied.
pub fn spawn_scene(
    world: &mut GameWorld,
    project: &Project,
    scene_id: &str,
    make_sprite: &mut SpriteFactory<'_>,
) -> Vec<SpawnedEntity> {
    let Some(scene) = project.scenes.get(scene_id) else {
        return Vec::new();
    };
    let mut visited = vec![scene_id.to_string()];
    spawn_scene_inner(
        world,
        project,
        scene,
        SceneOffset::default(),
        &mut visited,
        make_sprite,
    )
}

/// Spawn a scene and its nested scenes.
fn spawn_scene_inner(
    world: &mut GameWorld,
    project: &Project,
    scene: &Scene,
    offset: SceneOffset,
    visited: &mut Vec<String>,
    make_sprite: &mut SpriteFactory<'_>,
) -> Vec<SpawnedEntity> {
    let mut spawned = Vec::new();

    for entity in &scene.entities {
        let transform = entity_transform(entity);
        let color = entity_sprite_color(entity);
        let texture_path = entity_sprite_texture(entity);
        let role = entity_role(entity);

        let position = [
            transform.position.x + offset.x,
            transform.position.y + offset.y,
        ];
        let runtime_transform = Transform::new(position[0], position[1])
            .with_rotation(transform.rotation)
            .with_scale(transform.scale);
        let source_id = SourceId::new(entity.id.clone());

        match color {
            Some(rgba) => {
                let sprite = make_sprite(entity, rgba, texture_path.as_deref());
                match &role {
                    Some(role) => {
                        world.spawn((runtime_transform, sprite, Role::new(role.clone()), source_id));
                    }
                    None => {
                        world.spawn((runtime_transform, sprite, source_id));
                    }
                }
            }
            None => match &role {
                Some(role) => {
                    world.spawn((runtime_transform, Role::new(role.clone()), source_id));
                }
                None => {
                    world.spawn((runtime_transform, source_id));
                }
            },
        }

        spawned.push(SpawnedEntity {
            source_id: entity.id.clone(),
            position,
            rotation: transform.rotation,
            scale: transform.scale,
            color,
            texture: texture_path,
            role,
        });
    }

    for nested in &scene.nested_scenes {
        if visited.contains(&nested.scene_id) {
            continue;
        }
        let Some(child) = project.scenes.get(&nested.scene_id) else {
            continue;
        };
        visited.push(nested.scene_id.clone());
        let child_offset = SceneOffset {
            x: offset.x + nested.offset.x,
            y: offset.y + nested.offset.y,
        };
        spawned.extend(spawn_scene_inner(
            world,
            project,
            child,
            child_offset,
            visited,
            make_sprite,
        ));
        visited.pop();
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

/// Read the sprite tint of a scene entity, if it has one.
fn entity_sprite_color(entity: &Entity) -> Option<[f32; 4]> {
    for component in &entity.components {
        if let Component::Sprite { color, .. } = component {
            return Some(*color);
        }
    }
    None
}

/// Read the sprite texture path of a scene entity, if it has one.
fn entity_sprite_texture(entity: &Entity) -> Option<String> {
    for component in &entity.components {
        if let Component::Sprite { texture, .. } = component {
            return texture.clone();
        }
    }
    None
}

/// Read the gameplay role of a scene entity, if it has one.
fn entity_role(entity: &Entity) -> Option<String> {
    for component in &entity.components {
        if let Component::Role { role } = component {
            return Some(role.clone());
        }
    }
    None
}

/// Count the entities a scene would spawn, including nested scenes.
pub fn count_scene_entities(project: &Project, scene_id: &str) -> usize {
    fn count(project: &Project, scene_id: &str, visited: &mut Vec<String>) -> usize {
        if visited.contains(&scene_id.to_string()) {
            return 0;
        }
        let Some(scene) = project.scenes.get(scene_id) else {
            return 0;
        };
        visited.push(scene_id.to_string());
        let mut total = scene.entities.len();
        for nested in &scene.nested_scenes {
            total += count(project, &nested.scene_id, visited);
        }
        visited.pop();
        total
    }
    let mut visited = Vec::new();
    count(project, scene_id, &mut visited)
}
