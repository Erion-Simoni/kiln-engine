//! Spawning scene entities into the runtime world.
//!
//! Scene data lives in [`kiln_core::Project`] as plain data; this module
//! converts it into runtime components ([`kiln_core::Transform`] and
//! [`kiln_runtime::Sprite`]) inside a [`kiln_runtime::GameWorld`].

use kiln_core::{Component, Entity, Project, Scene, SceneOffset, Transform};
use kiln_runtime::{GameWorld, Sprite, Tilemap};
use std::collections::HashMap;

use crate::textures::TextureLibrary;

/// Default sprite size in pixels, used when a scene entity has no explicit size.
pub const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

/// The components created for a single scene entity.
#[derive(Debug, Clone)]
pub struct SpawnedEntity {
    /// Position of the entity in world space.
    pub position: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Uniform scale.
    pub scale: kiln_core::Vec2,
    /// Tint colour (RGBA) when the entity has a sprite.
    pub color: Option<[f32; 4]>,
    /// Asset path of the sprite texture, when the entity references one.
    pub texture: Option<String>,
}

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
    spawn_scene_with_textures(world, project, scene, offset, visited, None)
}

/// Spawn the entities of `scene` into `world` and return what was created.
///
/// Nested scenes referenced by `scene.nested_scenes` are spawned as well,
/// with their offsets applied. `visited` guards against cyclic nesting.
///
/// When `textures` is given, sprites referencing a texture use that texture
/// and are sized to the image's pixel dimensions.
pub fn spawn_scene_with_textures(
    world: &mut GameWorld,
    project: &Project,
    scene: &Scene,
    offset: SceneOffset,
    visited: &mut Vec<String>,
    textures: Option<&TextureLibrary>,
) -> Vec<SpawnedEntity> {
    let mut spawned = Vec::new();

    for entity in &scene.entities {
        let transform = entity_transform(entity);
        let color = entity_sprite_color(entity);
        let texture_path = entity_sprite_texture(entity);

        let position = [
            transform.position.x + offset.x,
            transform.position.y + offset.y,
        ];
        let runtime_transform = Transform::new(position[0], position[1])
            .with_rotation(transform.rotation)
            .with_scale(transform.scale);

        match color {
            Some(rgba) => {
                let mut sprite = Sprite::new(rgba[0], rgba[1], rgba[2], rgba[3]);
                let gpu_texture = texture_path
                    .as_deref()
                    .and_then(|path| textures.and_then(|lib| lib.get(path)));

                match gpu_texture {
                    Some(tex) => {
                        sprite = sprite
                            .with_texture(tex.id)
                            .with_size(tex.width as f32, tex.height as f32);
                    }
                    None => {
                        sprite = sprite.with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1]);
                    }
                }

                world.spawn((runtime_transform, sprite));
            }
            None => {
                world.spawn((runtime_transform,));
            }
        }

        spawned.push(SpawnedEntity {
            position,
            rotation: transform.rotation,
            scale: transform.scale,
            color,
            texture: texture_path,
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
        spawned.extend(spawn_scene_with_textures(
            world,
            project,
            child,
            child_offset,
            visited,
            textures,
        ));
        visited.pop();
    }

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
    let mut spawned = Vec::new();
    let tileset_gpu = crate::textures::tileset_textures(project, textures);

    for entity in &scene.entities {
        let Some(map_id) = entity_tilemap_id(entity) else {
            continue;
        };
        let Some(map) = project.tilemaps.get(&map_id) else {
            log::warn!("tilemap '{map_id}' not found in project");
            continue;
        };

        let transform = entity_transform(entity);
        let mut tilemap = Tilemap::new(map.clone(), project.tilesets.clone(), tileset_gpu.clone());
        tilemap.offset = [transform.position.x + offset.x, transform.position.y + offset.y];
        world.spawn((transform, tilemap));
        spawned.push(map_id);
    }

    spawned
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
    let mut visited = vec![scene_id.to_string()];
    spawn_scene_with_textures(
        world,
        project,
        scene,
        SceneOffset::default(),
        &mut visited,
        Some(textures),
    )
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

/// Read the tilemap id of a scene entity, if it has one.
fn entity_tilemap_id(entity: &Entity) -> Option<String> {
    for component in &entity.components {
        if let Component::Tilemap { map } = component {
            return Some(map.clone());
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

    count(project, scene_id, &mut Vec::new())
}

/// Map of scene id to entity count, useful for logging.
pub fn scene_entity_counts(project: &Project) -> HashMap<String, usize> {
    project
        .scenes
        .keys()
        .map(|id| (id.clone(), count_scene_entities(project, id)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::Scene;

    fn project_with_scene() -> Project {
        let mut project = Project::default();
        let scene = Scene {
            id: "main".to_string(),
            name: "Main".to_string(),
            entities: vec![
                Entity {
                    id: "e1".to_string(),
                    name: "Player".to_string(),
                    components: vec![
                        Component::Transform {
                            x: 10.0,
                            y: 20.0,
                            rotation: 0.5,
                            scale: 2.0,
                        },
                        Component::Sprite {
                            color: [1.0, 0.0, 0.0, 1.0],
                            texture: None,
                        },
                    ],
                },
                Entity {
                    id: "e2".to_string(),
                    name: "Empty".to_string(),
                    components: vec![],
                },
            ],
            nested_scenes: Vec::new(),
        };
        project.scenes.insert("main".to_string(), scene);
        project
    }

    #[test]
    fn spawns_transform_and_sprite() {
        let project = project_with_scene();
        let mut world = GameWorld::new();
        let spawned = spawn_scene_by_id(&mut world, &project, "main");

        assert_eq!(spawned.len(), 2);
        assert_eq!(spawned[0].position, [10.0, 20.0]);
        assert_eq!(spawned[0].rotation, 0.5);
        assert_eq!(spawned[0].scale, kiln_core::Vec2::new(2.0, 2.0));
        assert_eq!(spawned[0].color, Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(spawned[1].color, None);
    }

    #[test]
    fn unknown_scene_spawns_nothing() {
        let project = project_with_scene();
        let mut world = GameWorld::new();
        let spawned = spawn_scene_by_id(&mut world, &project, "missing");
        assert!(spawned.is_empty());
    }

    #[test]
    fn spawns_tilemap_components() {
        let mut project = project_with_scene();
        let mut map = kiln_core::TileMap::default();
        map.id = "map1".to_string();
        let mut layer = kiln_core::TileMapLayer {
            tileset: "terrain".to_string(),
            ..Default::default()
        };
        layer.set_tile_at(0, 0, 1);
        map.layers.push(layer);
        project.tilemaps.insert("map1".to_string(), map);
        project.tilesets.insert(
            "terrain".to_string(),
            kiln_core::TileSet {
                texture: "tiles/terrain.png".to_string(),
                tile_size: [16, 16],
                columns: 2,
                rows: 2,
                solid: Vec::new(),
            },
        );

        let scene = project.scenes.get_mut("main").expect("main");
        scene.entities.push(Entity {
            id: "e3".to_string(),
            name: "Map".to_string(),
            components: vec![Component::Tilemap {
                map: "map1".to_string(),
            }],
        });

        let mut world = GameWorld::new();
        let textures = TextureLibrary::default();
        let scene = project.scenes.get("main").expect("main");
        let spawned = spawn_tilemaps(
            &mut world,
            &project,
            scene,
            kiln_core::SceneOffset::default(),
            &textures,
        );
        assert_eq!(spawned, vec!["map1".to_string()]);
    }

    #[test]
    fn unknown_tilemap_is_skipped() {
        let mut project = project_with_scene();
        let scene = project.scenes.get_mut("main").expect("main");
        scene.entities.push(Entity {
            id: "e3".to_string(),
            name: "Map".to_string(),
            components: vec![Component::Tilemap {
                map: "missing".to_string(),
            }],
        });

        let mut world = GameWorld::new();
        let textures = TextureLibrary::default();
        let scene = project.scenes.get("main").expect("main");
        let spawned = spawn_tilemaps(
            &mut world,
            &project,
            scene,
            kiln_core::SceneOffset::default(),
            &textures,
        );
        assert!(spawned.is_empty());
    }

    #[test]
    fn nested_scene_offset_is_applied() {
        let mut project = project_with_scene();
        let child = Scene {
            id: "child".to_string(),
            name: "Child".to_string(),
            entities: vec![Entity {
                id: "c1".to_string(),
                name: "Nested".to_string(),
                components: vec![Component::Transform {
                    x: 1.0,
                    y: 2.0,
                    rotation: 0.0,
                    scale: 1.0,
                }],
            }],
            nested_scenes: Vec::new(),
        };
        project.scenes.insert("child".to_string(), child);

        let main = project.scenes.get_mut("main").expect("main");
        main.nested_scenes.push(kiln_core::NestedScene {
            scene_id: "child".to_string(),
            additive: false,
            offset: SceneOffset { x: 100.0, y: 200.0 },
        });

        let mut world = GameWorld::new();
        let spawned = spawn_scene_by_id(&mut world, &project, "main");

        assert_eq!(spawned.len(), 3);
        assert_eq!(spawned[2].position, [101.0, 202.0]);
        assert_eq!(count_scene_entities(&project, "main"), 3);
    }
}
