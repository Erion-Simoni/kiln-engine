//! Spawning project scenes into a runtime world.
//!
//! Both the standalone player and the editor build their world from the same
//! project data, so a scene looks the same in both. Textures are resolved by
//! the caller through [`SpriteFactory`], because only the player has a GPU to
//! upload them to.

use kiln_core::hierarchy_transform::world_transforms;
use kiln_core::project::{Component, Entity, Project, Scene, SceneOffset};
use kiln_core::Transform;

use crate::sprite::AnimationPlayer;
use crate::world::{GameWorld, GlobalTransform, ParentLink, Role, SourceId, WorldOffset};

/// Default sprite size in pixels, used when an entity has no texture.
pub const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

/// Creates the runtime sprite for a scene entity.
///
/// The factory receives the entity's tint and the asset path of its texture
/// (`None` when the entity has no texture) and returns the sprite to spawn.
pub type SpriteFactory<'a> =
    dyn FnMut(&Entity, [f32; 4], Option<&str>) -> crate::sprite::Sprite + 'a;

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

/// Recompute world transforms from the live local Transform components and
/// runtime Parent links. Call after gameplay updates and before rendering.
pub fn update_global_transforms(world: &mut GameWorld) -> Result<(), String> {
    let locals: std::collections::HashMap<_, _> = world
        .world_ref()
        .query::<(hecs::Entity, &Transform)>()
        .iter()
        .map(|(entity, local)| (entity, *local))
        .collect();
    let parents: std::collections::HashMap<_, _> = world
        .world_ref()
        .query::<(hecs::Entity, &ParentLink)>()
        .iter()
        .map(|(entity, parent)| (entity, parent.0))
        .collect();
    let offsets: std::collections::HashMap<_, _> = world
        .world_ref()
        .query::<(hecs::Entity, &WorldOffset)>()
        .iter()
        .map(|(entity, offset)| (entity, offset.0))
        .collect();
    let mut resolved = std::collections::HashMap::new();
    let mut visiting = Vec::new();
    for entity in locals.keys().copied() {
        resolve_runtime_transform(
            entity,
            &locals,
            &parents,
            &offsets,
            &mut resolved,
            &mut visiting,
        )?;
    }
    for (entity, transform) in resolved {
        if world.world_ref().get::<&GlobalTransform>(entity).is_ok() {
            world
                .world_mut()
                .get::<&mut GlobalTransform>(entity)
                .map_err(|_| "Could not update runtime global transform")?
                .0 = transform;
        } else {
            world
                .world_mut()
                .insert_one(entity, GlobalTransform(transform))
                .map_err(|_| "Could not add global transform to runtime entity")?;
        }
    }
    Ok(())
}

pub(crate) fn resolve_runtime_transform(
    entity: hecs::Entity,
    locals: &std::collections::HashMap<hecs::Entity, Transform>,
    parents: &std::collections::HashMap<hecs::Entity, hecs::Entity>,
    offsets: &std::collections::HashMap<hecs::Entity, [f32; 2]>,
    resolved: &mut std::collections::HashMap<hecs::Entity, Transform>,
    visiting: &mut Vec<hecs::Entity>,
) -> Result<Transform, String> {
    if let Some(transform) = resolved.get(&entity) {
        return Ok(*transform);
    }
    if visiting.contains(&entity) {
        return Err("Runtime transform hierarchy contains a cycle".into());
    }
    let local = *locals
        .get(&entity)
        .ok_or_else(|| "Runtime parent has no local transform".to_owned())?;
    visiting.push(entity);
    let world = if let Some(parent) = parents.get(&entity) {
        let parent_world =
            resolve_runtime_transform(*parent, locals, parents, offsets, resolved, visiting)?;
        kiln_core::hierarchy_transform::compose(parent_world, local)
    } else {
        let offset = offsets.get(&entity).copied().unwrap_or_default();
        let mut world = local;
        world.position += kiln_core::Vec2::new(offset[0], offset[1]);
        world
    };
    visiting.pop();
    if !world.position.is_finite() || !world.rotation.is_finite() || !world.scale.is_finite() {
        return Err("Runtime hierarchy produced a non-finite transform".into());
    }
    resolved.insert(entity, world);
    Ok(world)
}

/// Spawn a list of scene entities into the world.
///
/// This is the same work [`spawn_scene`] does for one scene's entities, but
/// takes the entities directly instead of looking them up in a project.
/// The editor uses it to rebuild a world from an undo snapshot.
pub fn spawn_entities(
    world: &mut GameWorld,
    entities: &[Entity],
    offset: SceneOffset,
    make_sprite: &mut SpriteFactory<'_>,
    project: &kiln_core::Project,
) -> Vec<SpawnedEntity> {
    let mut spawned = Vec::new();
    let hierarchy_scene = Scene {
        id: String::new(),
        name: String::new(),
        entities: entities.to_vec(),
        nested_scenes: Vec::new(),
        script: None,
    };
    let resolved_transforms = world_transforms(&hierarchy_scene).ok();

    for entity in entities {
        let local_transform = entity_transform(entity);
        let transform = resolved_transforms
            .as_ref()
            .and_then(|transforms| transforms.get(&entity.id).copied())
            .unwrap_or(local_transform);
        eprintln!(
            "DEBUG spawn_entities: entity id={} transform.position.x={} transform.position.y={}",
            entity.id, transform.position.x, transform.position.y
        );
        let color = entity_sprite_color(entity);
        let texture_path = entity_sprite_texture(entity);
        let animation_clip = entity_sprite_animation_clip(entity);
        let role_name = entity_role(entity);

        let position = [
            transform.position.x + offset.x,
            transform.position.y + offset.y,
        ];
        let runtime_transform = Transform::new(position[0], position[1])
            .with_rotation(transform.rotation)
            .with_scale(transform.scale);
        let source_id = SourceId::new(entity.id.clone());

        let sprite = color.map(|rgba| {
            let mut s = make_sprite(entity, rgba, texture_path.as_deref());
            s.animation_clip = animation_clip.clone();
            for component in &entity.components {
                if let Component::Sprite {
                    layer,
                    material,
                    width,
                    height,
                    ..
                } = component
                {
                    s.layer = *layer;
                    s.material = material.clone();
                    s.size = [*width, *height];
                }
            }
            s
        });

        eprintln!(
            "DEBUG spawn_entities: entity id={} sprite={:?}",
            entity.id,
            sprite.is_some()
        );

        if let Some(ref s) = sprite {
            eprintln!(
                "DEBUG spawn_entities: entity id={} sprite.texture={:?}",
                entity.id, s.texture
            );
        }

        match &role_name {
            Some(role_name_str) => {
                if let Some(sprite) = sprite {
                    let animation_clip = sprite.animation_clip.clone();
                    let entity = world.world_mut().spawn((
                        local_transform,
                        GlobalTransform(runtime_transform),
                        sprite,
                        Role::new(role_name_str.clone()),
                        source_id,
                    ));
                    // If the sprite references an animation clip, load it
                    if let Some(clip_name) = &animation_clip {
                        if let Some(clip) = project.animation_clips.get(clip_name) {
                            let mut anim = AnimationPlayer::new();
                            anim.add_clip(clip.clone());
                            anim.play(clip_name);
                            let _ = world.world_mut().insert_one(entity, anim);
                        }
                    }
                } else {
                    world.spawn((
                        local_transform,
                        GlobalTransform(runtime_transform),
                        Role::new(role_name_str.clone()),
                        source_id,
                    ));
                }
            }
            None => {
                if let Some(sprite) = sprite {
                    let animation_clip = sprite.animation_clip.clone();
                    let entity = world.world_mut().spawn((
                        local_transform,
                        GlobalTransform(runtime_transform),
                        sprite,
                        source_id,
                    ));
                    // If the sprite references an animation clip, load it
                    if let Some(clip_name) = &animation_clip {
                        if let Some(clip) = project.animation_clips.get(clip_name) {
                            let mut anim = AnimationPlayer::new();
                            anim.add_clip(clip.clone());
                            anim.play(clip_name);
                            let _ = world.world_mut().insert_one(entity, anim);
                        }
                    }
                } else {
                    world.spawn((
                        local_transform,
                        GlobalTransform(runtime_transform),
                        source_id,
                    ));
                }
            }
        }

        spawned.push(SpawnedEntity {
            source_id: entity.id.clone(),
            position,
            rotation: transform.rotation,
            scale: transform.scale,
            color,
            texture: texture_path,
            role: role_name,
        });
    }

    let spawned_handles: std::collections::HashMap<_, _> = world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .filter(|(_, source)| entities.iter().any(|entity| entity.id == source.0))
        .map(|(handle, source)| (source.0.clone(), handle))
        .collect();
    for entity in entities {
        let Some(child) = spawned_handles.get(&entity.id) else {
            continue;
        };
        for component in &entity.components {
            match component {
                Component::Physics { body, collider } => {
                    let _ = world.world_mut().insert(*child, (*body, *collider));
                }
                Component::Joint { joint } => {
                    let _ = world.world_mut().insert_one(*child, joint.clone());
                }
                _ => {}
            }
        }
        let parent_id = entity
            .components
            .iter()
            .find_map(|component| match component {
                Component::Parent { parent } => Some(parent),
                _ => None,
            });
        if let Some(parent) = parent_id.and_then(|parent_id| spawned_handles.get(parent_id)) {
            let _ = world.world_mut().insert_one(*child, ParentLink(*parent));
        } else if parent_id.is_none() {
            let _ = world
                .world_mut()
                .insert_one(*child, WorldOffset([offset.x, offset.y]));
        }
    }

    spawned
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
    let mut spawned = spawn_entities(world, &scene.entities, offset, make_sprite, project);

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

/// Read the animation clip name of a scene entity's sprite, if it has one.
fn entity_sprite_animation_clip(entity: &Entity) -> Option<String> {
    for component in &entity.components {
        if let Component::Sprite { animation_clip, .. } = component {
            return animation_clip.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sprite;

    #[test]
    fn scene_spawn_projects_parent_composed_transforms() {
        let mut project = Project::default();
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![],
                entities: vec![
                    Entity {
                        id: "root".into(),
                        name: "Root".into(),
                        components: vec![Component::Transform {
                            x: 10.0,
                            y: 20.0,
                            rotation: std::f32::consts::FRAC_PI_2,
                            scale: 2.0,
                        }],
                    },
                    Entity {
                        id: "child".into(),
                        name: "Child".into(),
                        components: vec![
                            Component::Parent {
                                parent: "root".into(),
                            },
                            Component::Transform {
                                x: 3.0,
                                y: 0.0,
                                rotation: 0.25,
                                scale: 0.5,
                            },
                            Component::Sprite {
                                color: [1.0; 4],
                                width: 40.0,
                                height: 20.0,
                                texture: None,
                                layer: 0.0,
                                material: None,
                                animation_clip: None,
                            },
                        ],
                    },
                ],
            },
        );
        let mut world = GameWorld::new();
        spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
            Sprite::new(color[0], color[1], color[2], color[3])
        });
        let child = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .find(|(_, id)| id.0 == "child")
            .unwrap()
            .0;
        {
            let transform = world.world_ref().get::<&GlobalTransform>(child).unwrap();
            assert!((transform.position.x - 10.0).abs() < 0.0001);
            assert!((transform.position.y - 26.0).abs() < 0.0001);
            assert!((transform.rotation - (std::f32::consts::FRAC_PI_2 + 0.25)).abs() < 0.0001);
            assert_eq!(transform.scale.x, 1.0);
        }
        let local_position = world.world_ref().get::<&Transform>(child).unwrap().position;
        assert_eq!(local_position, kiln_core::Vec2::new(3.0, 0.0));
        assert_eq!(
            world.world_ref().get::<&Sprite>(child).unwrap().size,
            [40.0, 20.0]
        );
        let root = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .find(|(_, id)| id.0 == "root")
            .unwrap()
            .0;
        *world.world_mut().get::<&mut Transform>(root).unwrap() = Transform::new(50.0, 60.0);
        update_global_transforms(&mut world).unwrap();
        let moved = *world.world_ref().get::<&GlobalTransform>(child).unwrap();
        assert!((moved.position.x - 53.0).abs() < 0.0001);
        assert!((moved.position.y - 60.0).abs() < 0.0001);
        assert!((moved.rotation - 0.25).abs() < 0.0001);
        assert_eq!(moved.scale.x, 0.5);
    }

    #[test]
    fn propagated_nested_scene_roots_keep_their_instance_offset() {
        let mut project = Project::default();
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![kiln_core::NestedScene {
                    scene_id: "child-scene".into(),
                    additive: true,
                    offset: SceneOffset { x: 100.0, y: 200.0 },
                }],
                entities: vec![],
            },
        );
        project.scenes.insert(
            "child-scene".into(),
            Scene {
                id: "child-scene".into(),
                name: "Child".into(),
                script: None,
                nested_scenes: vec![],
                entities: vec![Entity {
                    id: "nested-root".into(),
                    name: "Nested root".into(),
                    components: vec![Component::Transform {
                        x: 1.0,
                        y: 2.0,
                        rotation: 0.0,
                        scale: 1.0,
                    }],
                }],
            },
        );
        let mut world = GameWorld::new();
        spawn_scene(&mut world, &project, "main", &mut |_, color, _| {
            Sprite::new(color[0], color[1], color[2], color[3])
        });
        update_global_transforms(&mut world).unwrap();
        let nested = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId, &GlobalTransform)>()
            .iter()
            .find(|(_, id, _)| id.0 == "nested-root")
            .map(|(_, _, transform)| transform.0)
            .unwrap();
        assert_eq!(nested.position, kiln_core::Vec2::new(101.0, 202.0));
    }
}
