//! Validated, document-first commands for hierarchy edits.

use kiln_core::{Component, Entity, Project, Scene};

use crate::{editing::next_entity_id, history::SceneHistory};

/// A single authored entity or hierarchy operation.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityEdit {
    /// Create a default visible entity at a world position.
    Create { name: String, position: [f32; 2] },
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
        EntityEdit::Delete { entity_id } => {
            let index = unique_entity_index(scene, entity_id)?;
            scene.entities.remove(index);
            for entity in &mut scene.entities {
                entity.components.retain(|component| !matches!(component, Component::Parent { parent } if parent == entity_id));
            }
            Ok(None)
        }
        EntityEdit::Duplicate { entity_id } => {
            let index = unique_entity_index(scene, entity_id)?;
            let mut copy = scene.entities[index].clone();
            let id = next_id(project, scene);
            copy.id = id.clone();
            copy.name = format!("{} Copy", copy.name);
            scene.entities.push(copy);
            Ok(Some(id))
        }
        EntityEdit::Reparent {
            entity_id,
            parent_id,
        } => {
            let child_index = unique_entity_index(scene, entity_id)?;
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
            Ok(None)
        }
    }
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
