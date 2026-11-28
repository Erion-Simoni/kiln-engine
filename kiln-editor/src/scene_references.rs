//! Resolve entity references in a composed scene before publishing edits.
use kiln_core::{physics_data::Joint2D, Component, Entity, Project, Scene};
use std::collections::{HashMap, HashSet};

fn composition<'a>(
    project: &'a Project,
    edited: &'a Scene,
    root: &str,
) -> Result<Vec<&'a Scene>, String> {
    let mut pending = vec![root.to_owned()];
    let mut visited = HashSet::new();
    let mut scenes = Vec::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            return Err(format!("Scene `{id}` is repeated in composition `{root}`; remove the ambiguous nested link first"));
        }
        let scene = if id == edited.id {
            edited
        } else {
            project
                .scenes
                .get(&id)
                .ok_or_else(|| format!("Nested scene `{id}` is missing"))?
        };
        pending.extend(scene.nested_scenes.iter().map(|link| link.scene_id.clone()));
        scenes.push(scene);
    }
    Ok(scenes)
}

pub(crate) fn require_joint_endpoints(
    project: &Project,
    scene: &Scene,
    joint: &Joint2D,
) -> Result<(), String> {
    let mut counts = HashMap::new();
    for entity in composition(project, scene, &scene.id)?
        .iter()
        .flat_map(|scene| &scene.entities)
    {
        *counts.entry(entity.id.as_str()).or_insert(0usize) += 1;
    }
    for endpoint in [&joint.entity_a, &joint.entity_b] {
        if counts.get(endpoint.as_str()).copied() != Some(1) {
            return Err(format!("Joint endpoint `{endpoint}` must identify exactly one entity in scene `{}` and its nested scenes", scene.id));
        }
    }
    Ok(())
}

pub(crate) fn require_unreferenced(
    project: &Project,
    edited: &Scene,
    entity_id: &str,
) -> Result<(), String> {
    // Only containing compositions share this entity identity. An independent
    // scene may reuse the ID without making its camera/joints depend on this one.
    let mut roots: Vec<_> = project
        .scenes
        .keys()
        .filter(|id| crate::editing::scene_reaches(project, id, &edited.id, &mut HashSet::new()))
        .collect();
    roots.sort();
    for root in roots {
        let scenes = composition(project, edited, root)?;
        for scene in scenes {
            for owner in &scene.entities {
                // Components owned by the deleted entity disappear with it.
                if scene.id == edited.id && owner.id == entity_id {
                    continue;
                }
                if owner.components.iter().any(|component| matches!(component, Component::Joint {joint} if joint.entity_a == entity_id || joint.entity_b == entity_id)) {
                    return Err(format!("Entity `{entity_id}` is used by a joint on `{}` in scene `{}`; remove that joint before deleting the entity",owner.id,scene.id));
                }
            }
        }
        if project
            .cameras
            .get(root)
            .is_some_and(|camera| camera.target.as_deref() == Some(entity_id))
        {
            return Err(format!("Entity `{entity_id}` is the follow target of camera `{root}`; change the camera follow target before deleting the entity"));
        }
    }
    Ok(())
}

// Validate only newly published/changed joints; unrelated entity edits must not
// reject an unchanged legacy component that the user still needs to repair.
pub(crate) fn require_changed_joints(
    project: &Project,
    before: &Scene,
    after: &Scene,
) -> Result<(), String> {
    let previous: HashMap<_, _> = before
        .entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect();
    for entity in &after.entities {
        let previous = previous.get(entity.id.as_str());
        for component in &entity.components {
            if let Component::Joint { joint } = component {
                let unchanged = previous.is_some_and(|old| old.components.iter().any(|component| matches!(component,Component::Joint {joint: previous} if previous == joint)));
                if !unchanged {
                    require_joint_endpoints(project, after, joint)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn remap_joint_owner(entity: &mut Entity, original_id: &str) {
    for component in &mut entity.components {
        if let Component::Joint { joint } = component {
            for endpoint in [&mut joint.entity_a, &mut joint.entity_b] {
                if endpoint == original_id {
                    *endpoint = entity.id.clone();
                }
            }
        }
    }
}
