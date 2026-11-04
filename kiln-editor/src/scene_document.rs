//! Reconcile editable runtime components with authored scene metadata.

use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_runtime::{GameWorld, Role, SourceId, Sprite, Tilemap};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Keep authoring metadata absent from ECS and avoid baking nested instances
/// into their parent scene. This bridge is not a complete edit-command model.
pub fn snapshot(
    world: &GameWorld,
    textures: &HashMap<String, String>,
    roles: &HashMap<String, String>,
    project: Option<&Project>,
) -> Scene {
    let id = project.and_then(Project::default_scene_id);
    snapshot_for_scene(world, textures, roles, project, id.as_deref())
}

/// Reconcile a specific editor scene without changing the project's startup scene.
pub fn snapshot_for_scene(
    world: &GameWorld,
    textures: &HashMap<String, String>,
    roles: &HashMap<String, String>,
    project: Option<&Project>,
    scene_id: Option<&str>,
) -> Scene {
    let original = project.and_then(|p| scene_id.and_then(|id| p.scenes.get(id)));
    let mut scene = original.cloned().unwrap_or_else(|| Scene {
        id: "main_scene".into(),
        name: "Main Scene".into(),
        script: None,
        entities: Vec::new(),
        nested_scenes: Vec::new(),
    });
    let root_ids: HashSet<_> = scene.entities.iter().map(|e| e.id.clone()).collect();
    let mut nested_ids = HashSet::new();
    if let Some(project) = project {
        let mut visited = HashSet::from([scene.id.clone()]);
        nested_entity_ids(project, &scene, &mut visited, &mut nested_ids);
    }
    let ecs = world.world_ref();
    let runtime: BTreeMap<_, _> = ecs
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .map(|(entity, id)| (id.0.clone(), entity))
        .collect();
    let mut entities = Vec::new();
    for original in &scene.entities {
        if let Some(&entity) = runtime.get(&original.id) {
            entities.push(reconcile(
                ecs,
                entity,
                &original.id,
                Some(original),
                textures,
                None,
                project,
            ));
        }
    }
    for (id, &entity) in &runtime {
        if !root_ids.contains(id) && !nested_ids.contains(id) {
            entities.push(reconcile(
                ecs,
                entity,
                id,
                None,
                textures,
                if project.is_none() {
                    roles.get(id)
                } else {
                    None
                },
                project,
            ));
        }
    }
    scene.entities = entities;
    scene
}

fn nested_entity_ids(
    project: &Project,
    scene: &Scene,
    visited: &mut HashSet<String>,
    ids: &mut HashSet<String>,
) {
    for nested in &scene.nested_scenes {
        if !visited.insert(nested.scene_id.clone()) {
            continue;
        }
        if let Some(child) = project.scenes.get(&nested.scene_id) {
            ids.extend(child.entities.iter().map(|e| e.id.clone()));
            nested_entity_ids(project, child, visited, ids);
        }
    }
}

fn reconcile(
    ecs: &hecs::World,
    entity: hecs::Entity,
    id: &str,
    original: Option<&Entity>,
    textures: &HashMap<String, String>,
    fallback_role: Option<&String>,
    project: Option<&Project>,
) -> Entity {
    let transform = ecs
        .get::<&Transform>(entity)
        .ok()
        .map(|t| Component::Transform {
            x: t.position.x,
            y: t.position.y,
            rotation: t.rotation,
            scale: t.scale.x,
        });
    let sprite = ecs.get::<&Sprite>(entity).ok().map(|s| Component::Sprite {
        color: s.color,
        width: s.size[0],
        height: s.size[1],
        texture: textures.get(id).cloned().or_else(|| {
            original.and_then(|e| {
                e.components.iter().find_map(|c| {
                    if let Component::Sprite { texture, .. } = c {
                        texture.clone()
                    } else {
                        None
                    }
                })
            })
        }),
        layer: s.layer,
        material: s.material.clone(),
        animation_clip: s.animation_clip.clone(),
    });
    let role = ecs
        .get::<&Role>(entity)
        .ok()
        .map(|r| r.0.clone())
        .or_else(|| fallback_role.cloned())
        .map(|role| Component::Role { role });
    let tilemap = ecs
        .get::<&Tilemap>(entity)
        .ok()
        .map(|m| Component::Tilemap {
            map: m.map_id.clone(),
        });
    let mut components = Vec::new();
    if let Some(original) = original {
        for component in &original.components {
            let updated = match component {
                Component::Transform { .. } => transform.clone(),
                Component::Sprite { .. } => sprite.clone(),
                Component::Role { .. } => role.clone(),
                Component::Tilemap { map } => tilemap.clone().or_else(|| {
                    // An unresolved reference must not disappear merely on save.
                    project
                        .filter(|p| !p.tilemaps.contains_key(map))
                        .map(|_| component.clone())
                }),
                Component::Parent { .. } => Some(component.clone()),
                Component::PrefabInstance { .. } => Some(component.clone()),
            };
            if let Some(updated) = updated {
                components.push(updated);
            }
        }
    }
    let originals = original
        .map(|e| e.components.as_slice())
        .unwrap_or_default();
    // Scene loading supplies a default transform even for metadata-only entities;
    // do not invent an authored component just because the projection has one.
    if original.is_none() {
        components.extend(transform);
    }
    for component in [sprite, role, tilemap].into_iter().flatten() {
        let exists = originals
            .iter()
            .any(|c| std::mem::discriminant(c) == std::mem::discriminant(&component));
        if !exists {
            components.push(component);
        }
    }
    Entity {
        id: id.into(),
        name: original
            .map(|e| e.name.clone())
            .unwrap_or_else(|| id.into()),
        components,
    }
}
