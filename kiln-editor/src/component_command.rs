//! Authored component presence commands, independent of window/GPU/runtime state.

use crate::history::SceneHistory;
use kiln_core::{Component, Project, Scene};

/// Components exposed by the entity inspector. Parent is edited by the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    SpriteLayout,
    Particles,
    Physics,
    Joint,
    Script,
    Transform,
    Sprite,
    Role,
    Tilemap,
}

impl ComponentKind {
    /// Identify an inspector component, excluding hierarchy links.
    pub fn of(component: &Component) -> Option<Self> {
        match component {
            Component::SpriteLayout { .. } => Some(Self::SpriteLayout),
            Component::Particles { .. } => Some(Self::Particles),
            Component::Physics { .. } => Some(Self::Physics),
            Component::Joint { .. } => Some(Self::Joint),
            Component::Script { .. } => Some(Self::Script),
            Component::Transform { .. } => Some(Self::Transform),
            Component::Sprite { .. } => Some(Self::Sprite),
            Component::Role { .. } => Some(Self::Role),
            Component::Tilemap { .. } => Some(Self::Tilemap),
            Component::Parent { .. } | Component::EditorFlags { .. } => None,
            Component::PrefabInstance { .. } => None,
        }
    }

    /// Defaults for resource-free components; Tilemap needs an explicit map ID.
    pub fn default_component(self) -> Option<Component> {
        match self {
            Self::SpriteLayout => Some(Component::SpriteLayout {
                settings: Default::default(),
            }),
            Self::Particles => Some(Component::Particles {
                effect: Default::default(),
            }),
            Self::Physics => Some(Component::Physics {
                body: Default::default(),
                collider: Default::default(),
            }),
            Self::Joint => None,
            Self::Script => None,
            Self::Transform => Some(Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }),
            Self::Sprite => Some(Component::Sprite {
                color: [1.0; 4],
                width: 32.0,
                height: 32.0,
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
    /// Replace a resource-free component, adding it if absent; still undoable.
    SetVisual(Component),
    SetPhysics {
        body: kiln_core::physics_data::RigidBody2D,
        collider: kiln_core::physics_data::Collider2D,
    },
    SetJoint {
        joint: kiln_core::physics_data::Joint2D,
    },
    SetScript {
        path: String,
        properties: std::collections::HashMap<String, f64>,
    },
    Add(Component),
    Remove(ComponentKind),
    SetRole {
        role: String,
    },
    SetSpriteReferences {
        texture: Option<String>,
        material: Option<String>,
        animation_clip: Option<String>,
    },
    SetTilemapReference {
        map: String,
    },
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
        crate::scene_controls::require_editable(scene, &self.entity_id)?;
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
            ComponentEdit::SetVisual(component) => {
                validate(component, project)?;
                match component {
                    Component::SpriteLayout { .. } => ComponentKind::SpriteLayout,
                    Component::Particles { .. } => ComponentKind::Particles,
                    _ => return Err("SetVisual expects sprite layout or particle data".into()),
                }
            }
            ComponentEdit::SetPhysics { .. } => ComponentKind::Physics,
            ComponentEdit::SetJoint { .. } => ComponentKind::Joint,
            ComponentEdit::SetScript { .. } => ComponentKind::Script,
            ComponentEdit::Add(component) => {
                validate(component, project)?;
                ComponentKind::of(component).ok_or("Edit parent links through the hierarchy")?
            }
            ComponentEdit::Remove(kind) => *kind,
            ComponentEdit::SetRole { .. } => ComponentKind::Role,
            ComponentEdit::SetSpriteReferences { .. } => ComponentKind::Sprite,
            ComponentEdit::SetTilemapReference { .. } => ComponentKind::Tilemap,
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
            (ComponentEdit::SetVisual(component), index) => {
                if let Some(index) = index {
                    if serde_json::to_value(&components[*index]).ok()
                        == serde_json::to_value(component).ok()
                    {
                        return Ok(false);
                    }
                    components[*index] = component.clone();
                } else {
                    components.push(component.clone());
                }
                Ok(true)
            }
            (ComponentEdit::SetPhysics { body, collider }, Some(index)) => {
                let replacement = Component::Physics {
                    body: *body,
                    collider: *collider,
                };
                validate(&replacement, project)?;
                if serde_json::to_value(&components[*index]).ok()
                    == serde_json::to_value(&replacement).ok()
                {
                    return Ok(false);
                }
                components[*index] = replacement;
                Ok(true)
            }
            (ComponentEdit::SetJoint { joint }, Some(index)) => {
                let replacement = Component::Joint {
                    joint: joint.clone(),
                };
                validate(&replacement, project)?;
                if serde_json::to_value(&components[*index]).ok()
                    == serde_json::to_value(&replacement).ok()
                {
                    return Ok(false);
                }
                components[*index] = replacement;
                Ok(true)
            }
            (ComponentEdit::SetScript { path, properties }, Some(index)) => {
                let replacement = Component::Script {
                    path: path.clone(),
                    properties: properties.clone(),
                };
                validate(&replacement, project)?;
                if serde_json::to_value(&components[*index]).ok()
                    == serde_json::to_value(&replacement).ok()
                {
                    return Ok(false);
                }
                components[*index] = replacement;
                Ok(true)
            }
            (ComponentEdit::Add(component), None) => {
                components.push(component.clone());
                Ok(true)
            }
            (ComponentEdit::Remove(_), Some(index)) => {
                components.remove(*index);
                Ok(true)
            }
            (ComponentEdit::SetRole { role }, Some(index)) => {
                let role = role.trim();
                let Component::Role { role: current } = &mut components[*index] else {
                    unreachable!("role kind resolved to a non-role component")
                };
                if current == role {
                    Ok(false)
                } else {
                    *current = role.to_owned();
                    Ok(true)
                }
            }
            (
                ComponentEdit::SetSpriteReferences {
                    texture,
                    material,
                    animation_clip,
                },
                Some(index),
            ) => {
                let normalized = |value: &Option<String>| {
                    value
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                };
                let texture = normalized(texture);
                let material = normalized(material);
                let animation_clip = normalized(animation_clip);
                let Component::Sprite {
                    texture: current_texture,
                    material: current_material,
                    animation_clip: current_clip,
                    ..
                } = &mut components[*index]
                else {
                    unreachable!("sprite kind resolved to a non-sprite component")
                };
                let changed = *current_texture != texture
                    || *current_material != material
                    || *current_clip != animation_clip;
                if changed {
                    *current_texture = texture;
                    *current_material = material;
                    *current_clip = animation_clip;
                }
                Ok(changed)
            }
            (ComponentEdit::SetTilemapReference { map }, Some(index)) => {
                let map = map.trim();
                if map.is_empty() || !project.tilemaps.contains_key(map) {
                    return Err("Choose an existing project tilemap".into());
                }
                let Component::Tilemap { map: current } = &mut components[*index] else {
                    unreachable!("tilemap kind resolved to a non-tilemap component")
                };
                if current == map {
                    Ok(false)
                } else {
                    *current = map.to_owned();
                    Ok(true)
                }
            }
            _ => Ok(false),
        }
    }
}

fn validate(component: &Component, project: &Project) -> Result<(), String> {
    match component {
        Component::SpriteLayout { settings } => settings.validate(),
        Component::Particles { effect } => effect.validate(),
        Component::Physics { body, collider } => {
            kiln_core::physics_data::validate_body(body, collider)
        }
        Component::Joint { joint } => kiln_core::physics_data::validate_joint(joint),
        Component::Script { path, properties }
            if path.trim().is_empty()
                || std::path::Path::new(path).is_absolute()
                || std::path::Path::new(path)
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
                || !properties.values().all(|value| value.is_finite()) =>
        {
            Err("Script path must be project-relative and properties must be finite".into())
        }
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
        Component::PrefabInstance { .. } => {
            Err("Edit prefab links through the Prefab Editor".into())
        }
        _ => Ok(()),
    }
}
