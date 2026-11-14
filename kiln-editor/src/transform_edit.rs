//! Transform commands shared by inspector and viewport authoring.

use crate::history::SceneHistory;
use kiln_core::hierarchy_transform::world_transforms;
use kiln_core::{Component, Scene, Transform};
use kiln_runtime::{GameWorld, GlobalTransform, SourceId};

/// An explicit transform change to a root entity in a particular scene.
pub struct SetTransform<'a> {
    /// Scene document identity.
    pub scene_id: &'a str,
    /// Root entity identity, independent of its runtime handle.
    pub entity_id: &'a str,
    /// Complete uniform transform to apply.
    pub value: Transform,
}

impl SetTransform<'_> {
    /// Validate and update both the document and its existing runtime projection.
    /// Failure leaves both unchanged; nested entities cannot edit their parent scene.
    pub fn apply(&self, scene: &mut Scene, world: &mut GameWorld) -> Result<bool, String> {
        crate::scene_controls::require_editable(scene, self.entity_id)?;
        let value = self.value;
        if !value.position.is_finite()
            || !value.rotation.is_finite()
            || !value.scale.is_finite()
            || value.scale.x != value.scale.y
        {
            return Err("Scene transforms require finite values and uniform scale".into());
        }
        if scene.id != self.scene_id {
            return Err("Transform targets another scene".into());
        }
        let authored: Vec<_> = scene
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.id == self.entity_id)
            .map(|(i, _)| i)
            .collect();
        let [index] = authored.as_slice() else {
            return Err("Select one root-scene entity to edit its transform".into());
        };
        let runtime: Vec<_> = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId, &Transform)>()
            .iter()
            .filter(|(_, id, _)| id.0 == self.entity_id)
            .map(|(entity, _, _)| entity)
            .collect();
        let [_entity] = runtime.as_slice() else {
            return Err("Transform entity identity is missing or ambiguous".into());
        };
        let mut updated_scene = scene.clone();
        let components = &mut updated_scene.entities[*index].components;
        let updated = Component::Transform {
            x: value.position.x,
            y: value.position.y,
            rotation: value.rotation,
            scale: value.scale.x,
        };
        let slot = components
            .iter()
            .position(|c| matches!(c, Component::Transform { .. }));
        let unchanged = slot.is_some_and(|i| match &components[i] {
            Component::Transform {
                x,
                y,
                rotation,
                scale,
            } => {
                *x == value.position.x
                    && *y == value.position.y
                    && *rotation == value.rotation
                    && *scale == value.scale.x
            }
            _ => false,
        });
        if let Some(index) = slot {
            components[index] = updated;
        } else {
            components.push(updated);
        }
        let resolved = world_transforms(&updated_scene)?;
        let runtime_entities: Vec<_> = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId, &Transform)>()
            .iter()
            .map(|(entity, source, _)| (source.0.clone(), entity))
            .collect();
        for source_id in resolved.keys() {
            let matches: Vec<_> = runtime_entities
                .iter()
                .filter(|(id, _)| id == source_id)
                .map(|(_, entity)| *entity)
                .collect();
            if matches.len() > 1 {
                return Err(format!(
                    "Transform entity `{source_id}` is ambiguous in runtime"
                ));
            }
        }
        *scene = updated_scene;
        for (source_id, transform) in resolved {
            if let Some((_, entity)) = runtime_entities.iter().find(|(id, _)| *id == source_id) {
                if source_id == self.entity_id {
                    *world
                        .world_mut()
                        .get::<&mut Transform>(*entity)
                        .map_err(|_| "Entity has no runtime transform")? = value;
                }
                if world.world_ref().get::<&GlobalTransform>(*entity).is_ok() {
                    world
                        .world_mut()
                        .get::<&mut GlobalTransform>(*entity)
                        .map_err(|_| "Could not update runtime global transform")?
                        .0 = transform;
                } else {
                    world
                        .world_mut()
                        .insert_one(*entity, GlobalTransform(transform))
                        .map_err(|_| "Could not update runtime global transform")?;
                }
            }
        }
        Ok(!unchanged)
    }
}

/// One Inspector edit or viewport gesture, including all preview frames.
pub struct SceneTransaction {
    before: Scene,
    after: Scene,
    entity_id: String,
    widget: Option<egui::Id>,
}

impl SceneTransaction {
    /// Capture the document before the first preview mutation.
    pub fn new(before: Scene, entity_id: String) -> Self {
        Self {
            after: before.clone(),
            before,
            entity_id,
            widget: None,
        }
    }
    /// Check whether an update belongs to this gesture.
    pub fn targets(&self, scene_id: &str, entity_id: &str) -> bool {
        self.before.id == scene_id && self.entity_id == entity_id
    }
    /// Which inspector field owns the current gesture, if it came from a field.
    pub fn owns_widget(&self, id: egui::Id) -> bool {
        self.widget == Some(id)
    }
    /// Associate an inspector field so separate fields form separate undo steps.
    pub fn set_widget(&mut self, id: egui::Id) {
        self.widget = Some(id);
    }
    /// Retain the latest successfully applied preview document.
    pub fn update(&mut self, after: Scene) {
        self.after = after;
    }
    /// Record once; returning to the original scene preserves redo history.
    pub fn commit(self, history: &mut SceneHistory) {
        if serde_json::to_value(&self.before).ok() != serde_json::to_value(&self.after).ok() {
            history.record(self.before);
        }
    }
    /// Restore this scene through the editor's normal scene loader.
    pub fn cancel(self) -> Scene {
        self.before
    }
}

/// Backwards-compatible name for transform gesture callers.
pub type TransformTransaction = SceneTransaction;

/// Translation evaluated from the press position, independent of frame count.
#[derive(Debug, Clone)]
pub struct TranslationDrag {
    /// Logical pointer position at button press.
    pub origin: egui::Pos2,
    /// Transform at button press, never incrementally accumulated.
    pub initial: Transform,
}

impl TranslationDrag {
    /// Evaluate absolute displacement and optional world-space grid snapping.
    pub fn at(
        &self,
        pointer: egui::Pos2,
        units_per_point: [f32; 2],
        grid: Option<f32>,
    ) -> Transform {
        let mut value = self.initial;
        value.position.x += (pointer.x - self.origin.x) * units_per_point[0];
        value.position.y -= (pointer.y - self.origin.y) * units_per_point[1];
        if let Some(grid) = grid.filter(|g| g.is_finite() && *g > 0.0) {
            value.position.x = (value.position.x / grid).round() * grid;
            value.position.y = (value.position.y / grid).round() * grid;
        }
        value
    }
}
