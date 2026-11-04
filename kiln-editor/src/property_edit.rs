//! Validated authored edits for Inspector component properties.

use kiln_core::{Component, Scene};
use kiln_runtime::{GameWorld, SourceId, Sprite};

/// Update the authored Sprite tint and sorting layer for one root entity.
pub struct SetSpriteVisual<'a> {
    /// Scene document identity.
    pub scene_id: &'a str,
    /// Root entity identity.
    pub entity_id: &'a str,
    /// Authored tint channels in the range 0 through 1.
    pub color: [f32; 4],
    /// Painter sorting layer.
    pub layer: f32,
}

impl SetSpriteVisual<'_> {
    /// Validate and update authored data and its runtime projection atomically.
    pub fn apply(&self, scene: &mut Scene, world: &mut GameWorld) -> Result<bool, String> {
        if scene.id != self.scene_id {
            return Err("Sprite edit targets another scene".into());
        }
        if !self
            .color
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            || !self.layer.is_finite()
        {
            return Err("Sprite tint must be between 0 and 1 and layer must be finite".into());
        }
        let entities: Vec<_> = scene
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.id == self.entity_id)
            .map(|(index, _)| index)
            .collect();
        let [index] = entities.as_slice() else {
            return Err("Select one root-scene entity to edit its Sprite".into());
        };
        let runtime: Vec<_> = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .filter(|(_, id)| id.0 == self.entity_id)
            .map(|(entity, _)| entity)
            .collect();
        let [entity] = runtime.as_slice() else {
            return Err("Sprite entity identity is missing or ambiguous".into());
        };
        let mut sprite = world
            .world_mut()
            .get::<&mut Sprite>(*entity)
            .map_err(|_| "Entity has no runtime Sprite")?;
        let authored = scene.entities[*index]
            .components
            .iter_mut()
            .filter(|component| matches!(component, Component::Sprite { .. }));
        let mut authored = authored;
        let Some(component) = authored.next() else {
            return Err("Entity has no authored Sprite".into());
        };
        if authored.next().is_some() {
            return Err("Entity has ambiguous duplicate Sprite components".into());
        }
        let Component::Sprite { color, layer, .. } = component else {
            unreachable!()
        };
        let changed = *color != self.color || *layer != self.layer;
        *color = self.color;
        *layer = self.layer;
        sprite.color = self.color;
        sprite.layer = self.layer;
        Ok(changed)
    }
}
