//! Validated, document-first commands for hierarchy edits.

pub use crate::scene_controls::Alignment;
use kiln_core::hierarchy_transform::{local_from_world, world_transforms};
use kiln_core::{Component, Entity, Project, Scene, Transform};

use crate::{editing::next_entity_id, history::SceneHistory};

/// A single authored entity or hierarchy operation.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityEdit {
    /// Change editor-only flags, including unlocking a protected entity.
    SetEditorFlags {
        entity_id: String,
        locked: bool,
        hidden: bool,
    },
    /// Align independent selected world positions as one transaction.
    Align {
        entity_ids: Vec<String>,
        alignment: crate::scene_controls::Alignment,
    },
    /// Paste a validated project-scoped selection as one undo transaction.
    Paste { document: String },
    /// Rename an entity without changing its stable identity.
    Rename { entity_id: String, name: String },
    /// Create a default visible entity at a world position.
    Create { name: String, position: [f32; 2] },
    /// Drop a decoded project image into the scene as a fully authored sprite.
    CreateSprite {
        texture: String,
        position: [f32; 2],
        size: [u32; 2],
    },
    /// Instantiate a project prefab as a new root entity at a scene position.
    InstantiatePrefab {
        prefab_id: String,
        position: [f32; 2],
    },
    /// Restore the linked prefab definition and clear instance overrides.
    RevertPrefabOverrides { entity_id: String },
    /// Delete an entity and detach its direct children.
    Delete { entity_id: String },
    /// Duplicate an entity with its complete authored component list.
    Duplicate { entity_id: String },
    /// Set an entity's parent, or make it a root when `None`.
    Reparent {
        entity_id: String,
        parent_id: Option<String>,
    },
}

/// A hierarchy edit scoped to one authored scene.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityCommand {
    /// Scene identity this command was created for.
    pub scene_id: String,
    /// Requested edit.
    pub edit: EntityEdit,
}

impl EntityCommand {
    /// Apply the edit atomically and record exactly one history entry.
    /// Returns the affected entity ID for create and duplicate operations.
    pub fn execute(
        &self,
        scene: &mut Scene,
        project: &Project,
        history: &mut SceneHistory,
    ) -> Result<Option<String>, String> {
        if scene.id != self.scene_id || !project.scenes.contains_key(&self.scene_id) {
            return Err("Hierarchy edit targets another or missing scene".into());
        }
        let mut edited = scene.clone();
        let result = apply(&mut edited, project, &self.edit)?;
        if edited.entities.len() != scene.entities.len()
            || serde_json::to_value(&edited).ok() != serde_json::to_value(&*scene).ok()
        {
            history.record(scene.clone());
            *scene = edited;
        }
        Ok(result)
    }
}

fn apply(
    scene: &mut Scene,
    project: &Project,
    edit: &EntityEdit,
) -> Result<Option<String>, String> {
    match edit {
        EntityEdit::Rename { entity_id, .. }
        | EntityEdit::Delete { entity_id }
        | EntityEdit::Duplicate { entity_id }
        | EntityEdit::RevertPrefabOverrides { entity_id } => {
            crate::scene_controls::require_editable(scene, entity_id)?
        }
        EntityEdit::Reparent {
            entity_id,
            parent_id,
        } => {
            crate::scene_controls::require_editable(scene, entity_id)?;
            if let Some(parent) = parent_id {
                crate::scene_controls::require_editable(scene, parent)?;
            }
        }
        _ => {}
    }
    match edit {
        EntityEdit::SetEditorFlags {
            entity_id,
            locked,
            hidden,
        } => {
            let index = unique_entity_index(scene, entity_id)?;
            scene.entities[index]
                .components
                .retain(|c| !matches!(c, Component::EditorFlags { .. }));
            if *locked || *hidden {
                scene.entities[index]
                    .components
                    .push(Component::EditorFlags {
                        locked: *locked,
                        hidden: *hidden,
                    });
            }
            Ok(None)
        }
        EntityEdit::Align {
            entity_ids,
            alignment,
        } => {
            crate::scene_controls::align(scene, entity_ids, *alignment)?;
            Ok(None)
        }
        EntityEdit::Paste { document } => crate::entity_clipboard::paste(scene, project, document),
        EntityEdit::CreateSprite {
            texture,
            position,
            size,
        } => {
            if !position.iter().all(|value| value.is_finite())
                || size.contains(&0)
                || std::path::Path::new(texture).is_absolute()
                || texture.is_empty()
                || std::path::Path::new(texture)
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(
                    "Sprite drop needs a project image, finite position and valid dimensions"
                        .into(),
                );
            }
            let id = next_id(project, scene);
            scene.entities.push(Entity {
                id: id.clone(),
                name: std::path::Path::new(texture)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Sprite")
                    .into(),
                components: vec![
                    Component::Transform {
                        x: position[0],
                        y: position[1],
                        rotation: 0.0,
                        scale: 1.0,
                    },
                    Component::Sprite {
                        color: [1.0; 4],
                        width: size[0] as f32,
                        height: size[1] as f32,
                        texture: Some(texture.clone()),
                        layer: 0.0,
                        material: None,
                        animation_clip: None,
                    },
                    Component::SpriteLayout {
                        settings: Default::default(),
                    },
                ],
            });
            Ok(Some(id))
        }
        EntityEdit::Rename { entity_id, name } => {
            let index = unique_entity_index(scene, entity_id)?;
            let name = name.trim();
            if name.is_empty() {
                return Err("Entity name cannot be empty".into());
            }
            if scene.entities[index].name == name {
                return Ok(None);
            }
            scene.entities[index].name = name.to_owned();
            Ok(None)
        }
        EntityEdit::Create { name, position } => {
            if name.trim().is_empty() || !position.iter().all(|value| value.is_finite()) {
                return Err("Entity name and position must be valid".into());
            }
            let id = next_id(project, scene);
            scene.entities.push(Entity {
                id: id.clone(),
                name: name.trim().to_owned(),
                components: vec![
                    Component::Transform {
                        x: position[0],
                        y: position[1],
                        rotation: 0.0,
                        scale: 1.0,
                    },
                    Component::Sprite {
                        color: [1.0; 4],
                        width: 32.0,
                        height: 32.0,
                        texture: None,
                        layer: 0.0,
                        material: None,
                        animation_clip: None,
                    },
                    Component::Role {
                        role: String::new(),
                    },
                ],
            });
            Ok(Some(id))
        }
        EntityEdit::InstantiatePrefab {
            prefab_id,
            position,
        } => {
            if !position.iter().all(|value| value.is_finite()) {
                return Err("Prefab instance position must be finite".into());
            }
            let prefab = project
                .prefabs
                .get(prefab_id)
                .filter(|prefab| prefab.id == *prefab_id)
                .ok_or("Choose an existing project prefab")?;
            let id = next_id(project, scene);
            let mut instance = prefab.root_entity.clone();
            instance.id = id.clone();
            instance.name = format!("{} Instance", prefab.name);
            instance.components.retain(|component| {
                !matches!(
                    component,
                    Component::Parent { .. } | Component::PrefabInstance { .. }
                )
            });
            let mut has_transform = false;
            let mut instance_overrides = kiln_core::PrefabOverride::new(id.clone());
            for component in &mut instance.components {
                if let Component::Transform { x, y, .. } = component {
                    let override_x = (*x != position[0]).then_some(position[0]);
                    let override_y = (*y != position[1]).then_some(position[1]);
                    *x = position[0];
                    *y = position[1];
                    if override_x.is_some() || override_y.is_some() {
                        instance_overrides.add_override(
                            "transform".into(),
                            kiln_core::ComponentOverride::Transform {
                                x: override_x,
                                y: override_y,
                                rotation: None,
                                scale: None,
                            },
                        );
                    }
                    has_transform = true;
                    break;
                }
            }
            if !has_transform {
                instance.components.push(Component::Transform {
                    x: position[0],
                    y: position[1],
                    rotation: 0.0,
                    scale: 1.0,
                });
                instance_overrides.add_override(
                    "transform".into(),
                    kiln_core::ComponentOverride::Transform {
                        x: Some(position[0]),
                        y: Some(position[1]),
                        rotation: None,
                        scale: None,
                    },
                );
            }
            instance.components.push(Component::PrefabInstance {
                prefab_id: prefab.id.clone(),
                overrides: instance_overrides,
            });
            scene.entities.push(instance);
            Ok(Some(id))
        }
        EntityEdit::RevertPrefabOverrides { entity_id } => {
            let index = unique_entity_index(scene, entity_id)?;
            let prefab_id = scene.entities[index]
                .components
                .iter()
                .find_map(|component| match component {
                    Component::PrefabInstance { prefab_id, .. } => Some(prefab_id.clone()),
                    _ => None,
                })
                .ok_or("Selected entity is not a prefab instance")?;
            let parent = scene.entities[index]
                .components
                .iter()
                .find_map(|component| match component {
                    Component::Parent { parent } => Some(parent.clone()),
                    _ => None,
                });
            let prefab = project
                .prefabs
                .get(&prefab_id)
                .filter(|prefab| prefab.id == prefab_id)
                .ok_or("Prefab definition is missing")?;
            let mut restored = prefab.root_entity.clone();
            restored.id = entity_id.clone();
            restored.name = scene.entities[index].name.clone();
            restored.components.retain(|component| {
                !matches!(
                    component,
                    Component::Parent { .. } | Component::PrefabInstance { .. }
                )
            });
            if let Some(parent) = parent {
                restored.components.push(Component::Parent { parent });
            }
            restored.components.push(Component::PrefabInstance {
                prefab_id,
                overrides: kiln_core::PrefabOverride::new(entity_id.clone()),
            });
            crate::scene_controls::preserve_flags(&scene.entities[index], &mut restored);
            scene.entities[index] = restored;
            Ok(None)
        }
        EntityEdit::Delete { entity_id } => {
            let flags = crate::scene_controls::resolved_flags(scene);
            if scene.entities.iter().any(|child| {
                child
                    .components
                    .iter()
                    .any(|c| matches!(c,Component::Parent{parent} if parent==entity_id))
                    && flags.get(&child.id).is_some_and(|f| f.locked)
            }) {
                return Err(
                    "Deleting this entity would reparent a locked child; unlock the child first"
                        .into(),
                );
            }
            let index = unique_entity_index(scene, entity_id)?;
            let world = world_transforms(scene)?;
            let children: Vec<_> = scene
                .entities
                .iter()
                .filter(|entity| entity.components.iter().any(|component| matches!(component, Component::Parent { parent } if parent == entity_id)))
                .map(|entity| (entity.id.clone(), world[&entity.id]))
                .collect();
            scene.entities.remove(index);
            for entity in &mut scene.entities {
                entity.components.retain(|component| !matches!(component, Component::Parent { parent } if parent == entity_id));
            }
            for (child_id, child_world) in children {
                let child_index = unique_entity_index(scene, &child_id)?;
                let child = &mut scene.entities[child_index];
                set_local_transform(child, child_world)?;
            }
            Ok(None)
        }
        EntityEdit::Duplicate { entity_id } => {
            let index = unique_entity_index(scene, entity_id)?;
            let mut copy = scene.entities[index].clone();
            let id = next_id(project, scene);
            copy.id = id.clone();
            copy.name = format!("{} Copy", copy.name);
            for component in &mut copy.components {
                if let Component::Joint { joint } = component {
                    if joint.entity_a == *entity_id {
                        joint.entity_a = id.clone();
                    }
                    if joint.entity_b == *entity_id {
                        joint.entity_b = id.clone();
                    }
                }
                if let Component::PrefabInstance { overrides, .. } = component {
                    overrides.entity_id = id.clone();
                }
            }
            scene.entities.push(copy);
            Ok(Some(id))
        }
        EntityEdit::Reparent {
            entity_id,
            parent_id,
        } => {
            let child_index = unique_entity_index(scene, entity_id)?;
            let world_before = world_transforms(scene)?
                .get(entity_id)
                .copied()
                .ok_or_else(|| format!("Entity `{entity_id}` has no resolved transform"))?;
            if let Some(parent) = parent_id {
                unique_entity_index(scene, parent)?;
                if parent == entity_id || is_descendant(scene, parent, entity_id)? {
                    return Err("Reparenting would create a hierarchy cycle".into());
                }
            }
            let child = &mut scene.entities[child_index];
            let parents: Vec<_> = child
                .components
                .iter()
                .enumerate()
                .filter_map(|(i, c)| matches!(c, Component::Parent { .. }).then_some(i))
                .collect();
            if parents.len() > 1 {
                return Err("Entity has ambiguous duplicate parent links".into());
            }
            let current_parent =
                parents
                    .first()
                    .and_then(|index| match &child.components[*index] {
                        Component::Parent { parent } => Some(parent.as_str()),
                        _ => None,
                    });
            if current_parent == parent_id.as_deref() {
                return Ok(None);
            }
            child
                .components
                .retain(|component| !matches!(component, Component::Parent { .. }));
            if let Some(parent) = parent_id {
                child.components.push(Component::Parent {
                    parent: parent.clone(),
                });
            }
            let local = local_from_world(scene, entity_id, world_before)?;
            set_local_transform(&mut scene.entities[child_index], local)?;
            Ok(None)
        }
    }
}

pub(crate) fn set_local_transform(entity: &mut Entity, transform: Transform) -> Result<(), String> {
    let indices: Vec<_> = entity
        .components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| {
            matches!(component, Component::Transform { .. }).then_some(index)
        })
        .collect();
    if indices.len() > 1 {
        return Err(format!(
            "Entity `{}` has duplicate Transform components",
            entity.id
        ));
    }
    let component = Component::Transform {
        x: transform.position.x,
        y: transform.position.y,
        rotation: transform.rotation,
        scale: transform.scale.x,
    };
    if let Some(index) = indices.first() {
        entity.components[*index] = component;
    } else {
        entity.components.push(component);
    }
    Ok(())
}

fn unique_entity_index(scene: &Scene, id: &str) -> Result<usize, String> {
    let matches: Vec<_> = scene
        .entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| entity.id == id)
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(format!("Entity '{id}' does not exist in this scene")),
        _ => Err(format!("Entity ID '{id}' is ambiguous in this scene")),
    }
}

fn is_descendant(scene: &Scene, possible_descendant: &str, ancestor: &str) -> Result<bool, String> {
    let mut cursor = possible_descendant;
    let mut visited = Vec::new();
    loop {
        if cursor == ancestor {
            return Ok(true);
        }
        if visited.iter().any(|id| id == cursor) {
            return Err("Existing hierarchy contains a cycle".into());
        }
        visited.push(cursor.to_owned());
        let index = unique_entity_index(scene, cursor)?;
        let parent =
            scene.entities[index]
                .components
                .iter()
                .find_map(|component| match component {
                    Component::Parent { parent } => Some(parent.as_str()),
                    _ => None,
                });
        match parent {
            Some(parent) => cursor = parent,
            None => return Ok(false),
        }
    }
}

fn next_id(project: &Project, scene: &Scene) -> String {
    let mut ids: Vec<String> = project
        .scenes
        .values()
        .flat_map(|scene| scene.entities.iter().map(|entity| entity.id.clone()))
        .collect();
    ids.extend(scene.entities.iter().map(|entity| entity.id.clone()));
    next_entity_id(&ids)
}
