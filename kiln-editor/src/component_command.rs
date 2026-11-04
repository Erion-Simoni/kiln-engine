//! Authored component presence commands, independent of window/GPU/runtime state.

use crate::history::SceneHistory;
use kiln_core::{Component, Project, Scene};

/// Components exposed by the entity inspector. Parent is edited by the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Transform,
    Sprite,
    Role,
    Tilemap,
}

impl ComponentKind {
    /// Identify an inspector component, excluding hierarchy links.
    pub fn of(component: &Component) -> Option<Self> {
        match component {
            Component::Transform { .. } => Some(Self::Transform),
            Component::Sprite { .. } => Some(Self::Sprite),
            Component::Role { .. } => Some(Self::Role),
            Component::Tilemap { .. } => Some(Self::Tilemap),
            Component::Parent { .. } => None,
        }
    }

    /// Defaults for resource-free components; Tilemap needs an explicit map ID.
    pub fn default_component(self) -> Option<Component> {
        match self {
            Self::Transform => Some(Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }),
            Self::Sprite => Some(Component::Sprite {
                color: [1.0; 4],
                texture: None,
                layer: 0.0,
                material: None,
                animation_clip: None,
            }),
            Self::Role => Some(Component::Role {
                role: String::new(),
            }),
            Self::Tilemap => None,
        }
    }
}

/// One component-presence edit. Add never overwrites an existing component.
#[derive(Debug, Clone)]
pub enum ComponentEdit {
    Add(Component),
    Remove(ComponentKind),
}

/// A command targeting one root entity in one scene document.
#[derive(Debug, Clone)]
pub struct ComponentCommand {
    /// Authored scene identity.
    pub scene_id: String,
    /// Stable root entity identity.
    pub entity_id: String,
    /// Requested component change.
    pub edit: ComponentEdit,
}

impl ComponentCommand {
    /// Apply atomically to the document and record exactly one actual edit.
    /// Rebuild the runtime projection only when this returns true.
    pub fn execute(
        &self,
        scene: &mut Scene,
        project: &Project,
        history: &mut SceneHistory,
    ) -> Result<bool, String> {
        let before = scene.clone();
        let changed = self.apply(scene, project)?;
        if changed {
            history.record(before);
        }
        Ok(changed)
    }

    fn apply(&self, scene: &mut Scene, project: &Project) -> Result<bool, String> {
        if scene.id != self.scene_id || !project.scenes.contains_key(&self.scene_id) {
            return Err("Component edit targets another or missing scene".into());
        }
        let targets: Vec<_> = scene
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.id == self.entity_id)
            .map(|(i, _)| i)
            .collect();
        let [index] = targets.as_slice() else {
            return Err("Select one root-scene entity to edit its components".into());
        };
        let kind = match &self.edit {
            ComponentEdit::Add(component) => {
                validate(component, project)?;
                ComponentKind::of(component).ok_or("Edit parent links through the hierarchy")?
            }
            ComponentEdit::Remove(kind) => *kind,
        };
        let components = &mut scene.entities[*index].components;
        let matches: Vec<_> = components
            .iter()
            .enumerate()
            .filter(|(_, c)| ComponentKind::of(c) == Some(kind))
            .map(|(i, _)| i)
            .collect();
        if matches.len() > 1 {
            return Err("Entity has ambiguous duplicate components".into());
        }
        match (&self.edit, matches.first()) {
            (ComponentEdit::Add(component), None) => {
                components.push(component.clone());
                Ok(true)
            }
            (ComponentEdit::Remove(_), Some(index)) => {
                components.remove(*index);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn validate(component: &Component, project: &Project) -> Result<(), String> {
    match component {
        Component::Transform {
            x,
            y,
            rotation,
            scale,
        } if ![x, y, rotation, scale].iter().all(|v| v.is_finite()) => {
            Err("Transform values must be finite".into())
        }
        Component::Sprite { color, layer, .. }
            if !color.iter().all(|v| v.is_finite()) || !layer.is_finite() =>
        {
            Err("Sprite values must be finite".into())
        }
        Component::Tilemap { map } if map.is_empty() || !project.tilemaps.contains_key(map) => {
            Err("Choose an existing project tilemap".into())
        }
        Component::Parent { .. } => Err("Edit parent links through the hierarchy".into()),
        _ => Ok(()),
    }
}
