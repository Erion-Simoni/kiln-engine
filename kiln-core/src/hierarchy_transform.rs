//! Resolve authored local transforms through scene Parent links.

use crate::project::{Component, Scene};
use crate::{Transform, Vec2};
use std::collections::HashMap;

/// Resolve every entity's authored local transform into world space.
///
/// Parent translation is rotated and scaled by its ancestor; rotation and
/// uniform scale compose additively and multiplicatively respectively.
pub fn world_transforms(scene: &Scene) -> Result<HashMap<String, Transform>, String> {
    let entities: HashMap<_, _> = scene
        .entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect();
    if entities.len() != scene.entities.len() {
        return Err("Scene entity IDs must be unique to resolve transforms".into());
    }
    let mut resolved = HashMap::new();
    let mut visiting = Vec::new();
    for entity in &scene.entities {
        resolve_entity(entity.id.as_str(), &entities, &mut resolved, &mut visiting)?;
    }
    Ok(resolved)
}

/// Convert a world-space transform into an entity's authored local space.
pub fn local_from_world(
    scene: &Scene,
    entity_id: &str,
    world: Transform,
) -> Result<Transform, String> {
    let entity = scene
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .ok_or_else(|| format!("Entity `{entity_id}` is missing from scene"))?;
    let Some(parent_id) = parent_id(entity)? else {
        return Ok(world);
    };
    let transforms = world_transforms(scene)?;
    let parent = transforms
        .get(parent_id)
        .ok_or_else(|| format!("Parent `{parent_id}` is missing from scene"))?;
    if parent.scale.x == 0.0 {
        return Err("Cannot convert world transform through a zero-scale parent".into());
    }
    let relative = (world.position - parent.position).rotate(Vec2::from_angle(-parent.rotation))
        / parent.scale.x;
    Ok(Transform::new(relative.x, relative.y)
        .with_rotation(world.rotation - parent.rotation)
        .with_scale(Vec2::splat(world.scale.x / parent.scale.x)))
}

fn resolve_entity(
    id: &str,
    entities: &HashMap<&str, &crate::project::Entity>,
    resolved: &mut HashMap<String, Transform>,
    visiting: &mut Vec<String>,
) -> Result<Transform, String> {
    if let Some(transform) = resolved.get(id) {
        return Ok(*transform);
    }
    if visiting.iter().any(|visiting_id| visiting_id == id) {
        return Err(format!("Parent cycle contains entity `{id}`"));
    }
    let entity = entities
        .get(id)
        .ok_or_else(|| format!("Parent `{id}` is missing from scene"))?;
    visiting.push(id.to_owned());
    let local = entity_transform(entity);
    let world = if let Some(parent_id) = parent_id(entity)? {
        let parent = resolve_entity(parent_id, entities, resolved, visiting)?;
        compose(parent, local)
    } else {
        local
    };
    visiting.pop();
    if !world.position.is_finite() || !world.rotation.is_finite() || !world.scale.is_finite() {
        return Err(format!("Entity `{id}` resolves to a non-finite transform"));
    }
    resolved.insert(id.to_owned(), world);
    Ok(world)
}

fn parent_id(entity: &crate::project::Entity) -> Result<Option<&str>, String> {
    let mut parents = entity
        .components
        .iter()
        .filter_map(|component| match component {
            Component::Parent { parent } => Some(parent.as_str()),
            _ => None,
        });
    let parent = parents.next();
    if parents.next().is_some() {
        return Err(format!(
            "Entity `{}` has duplicate Parent components",
            entity.id
        ));
    }
    Ok(parent)
}

fn entity_transform(entity: &crate::project::Entity) -> Transform {
    entity
        .components
        .iter()
        .find_map(|component| match component {
            Component::Transform {
                x,
                y,
                rotation,
                scale,
            } => Some(
                Transform::new(*x, *y)
                    .with_rotation(*rotation)
                    .with_scale(Vec2::splat(*scale)),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// Compose a local transform with its parent world transform.
pub fn compose(parent: Transform, local: Transform) -> Transform {
    let translated = local.position * parent.scale.x;
    let position = parent.position + translated.rotate(Vec2::from_angle(parent.rotation));
    Transform::new(position.x, position.y)
        .with_rotation(parent.rotation + local.rotation)
        .with_scale(Vec2::splat(parent.scale.x * local.scale.x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Entity;

    fn entity(id: &str, parent: Option<&str>, x: f32, y: f32, rotation: f32, scale: f32) -> Entity {
        let mut components = vec![Component::Transform {
            x,
            y,
            rotation,
            scale,
        }];
        if let Some(parent) = parent {
            components.push(Component::Parent {
                parent: parent.into(),
            });
        }
        Entity {
            id: id.into(),
            name: id.into(),
            components,
        }
    }

    fn scene(entities: Vec<Entity>) -> Scene {
        Scene {
            id: "main".into(),
            name: "Main".into(),
            entities,
            nested_scenes: vec![],
            script: None,
        }
    }

    #[test]
    fn resolves_nested_translation_rotation_and_uniform_scale() {
        let scene = scene(vec![
            entity("root", None, 10.0, 20.0, std::f32::consts::FRAC_PI_2, 2.0),
            entity("child", Some("root"), 3.0, 0.0, 0.25, 0.5),
            entity("grandchild", Some("child"), 4.0, 0.0, 0.5, 3.0),
        ]);
        let transforms = world_transforms(&scene).unwrap();
        let child = transforms["child"];
        assert!((child.position.x - 10.0).abs() < 0.0001);
        assert!((child.position.y - 26.0).abs() < 0.0001);
        assert!((child.rotation - (std::f32::consts::FRAC_PI_2 + 0.25)).abs() < 0.0001);
        assert_eq!(child.scale.x, 1.0);
        let grandchild = transforms["grandchild"];
        assert!(
            (grandchild.position.x - (10.0 + 4.0 * (std::f32::consts::FRAC_PI_2 + 0.25).cos()))
                .abs()
                < 0.0001
        );
        assert!(
            (grandchild.position.y - (26.0 + 4.0 * (std::f32::consts::FRAC_PI_2 + 0.25).sin()))
                .abs()
                < 0.0001
        );
        assert_eq!(grandchild.scale.x, 3.0);
    }

    #[test]
    fn rejects_cycles_missing_parents_and_duplicate_ids() {
        assert!(world_transforms(&scene(vec![
            entity("a", Some("b"), 0.0, 0.0, 0.0, 1.0),
            entity("b", Some("a"), 0.0, 0.0, 0.0, 1.0)
        ]))
        .is_err());
        assert!(
            world_transforms(&scene(vec![entity("a", Some("gone"), 0.0, 0.0, 0.0, 1.0)])).is_err()
        );
        assert!(world_transforms(&scene(vec![
            entity("a", None, 0.0, 0.0, 0.0, 1.0),
            entity("a", None, 1.0, 0.0, 0.0, 1.0)
        ]))
        .is_err());
    }

    #[test]
    fn converts_world_transform_back_to_local_under_parent() {
        let scene = scene(vec![
            entity("root", None, 5.0, 8.0, std::f32::consts::FRAC_PI_2, 2.0),
            entity("child", Some("root"), 1.0, 2.0, 0.5, 3.0),
        ]);
        let mut changed = scene.clone();
        let world = world_transforms(&scene).unwrap()["child"];
        let local = local_from_world(&scene, "child", world).unwrap();
        assert!((local.position.x - 1.0).abs() < 0.0001);
        assert!((local.position.y - 2.0).abs() < 0.0001);
        changed.entities[1].components[0] = Component::Transform {
            x: local.position.x,
            y: local.position.y,
            rotation: local.rotation,
            scale: local.scale.x,
        };
        assert_eq!(world_transforms(&changed).unwrap()["child"], world);
    }
}
