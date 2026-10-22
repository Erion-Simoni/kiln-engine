//! Editing the entities of a scene.
//!
//! The inspector edits the world directly, so the editor needs ways to add
//! and remove entities and to give them the components a scene entity has.

use kiln_core::Transform;
use kiln_runtime::{GameWorld, Role, SourceId, Sprite};

/// The components a new scene entity gets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityDefaults {
    /// The size of a new sprite, in world units.
    pub size: [f32; 2],
    /// The tint of a new sprite.
    pub color: [f32; 4],
}

impl Default for EntityDefaults {
    fn default() -> Self {
        Self {
            size: [32.0, 32.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// The id of the next entity, counted from one.
///
/// The ids are unique within a scene, so the editor can name an entity and
/// find it again after a reload.
pub fn next_entity_id(existing: &[String]) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("entity_{index}");
        if !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// Add an entity to the world and return its source id and handle.
///
/// The entity gets a transform, a sprite and a source id, so it is saved with
/// the scene like every other entity.
pub fn add_entity(
    world: &mut GameWorld,
    existing_ids: &[String],
    position: [f32; 2],
    defaults: EntityDefaults,
) -> (String, hecs::Entity) {
    let id = next_entity_id(existing_ids);
    let entity = world.world_mut().spawn((
        Transform::new(position[0], position[1]),
        Sprite::new(
            defaults.color[0],
            defaults.color[1],
            defaults.color[2],
            defaults.color[3],
        )
        .with_size(defaults.size[0], defaults.size[1]),
        SourceId::new(id.clone()),
    ));
    (id, entity)
}

/// Remove an entity from the world.
///
/// Returns whether the entity was there.
pub fn remove_entity(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world.world_mut().despawn(entity).is_ok()
}

/// Set the gameplay role of an entity.
///
/// Returns whether the entity was there.
pub fn set_role(world: &mut GameWorld, entity: hecs::Entity, role: &str) -> bool {
    let world = world.world_mut();
    if world.get::<&SourceId>(entity).is_err() {
        return false;
    }
    // A role is a component, so an entity that already has one is rebuilt.
    let has_role = world.get::<&Role>(entity).is_ok();
    if has_role {
        let _ = world.remove_one::<Role>(entity);
    }
    world
        .insert_one(entity, Role::new(role.to_string()))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_entity_id_skips_the_ones_that_exist() {
        let existing = vec!["entity_1".to_string(), "entity_2".to_string()];
        assert_eq!(next_entity_id(&existing), "entity_3");
    }

    #[test]
    fn the_first_entity_id_is_one() {
        assert_eq!(next_entity_id(&[]), "entity_1");
    }

    #[test]
    fn an_added_entity_gets_a_transform_and_a_sprite() {
        let mut world = GameWorld::new();
        let (id, _) = add_entity(&mut world, &[], [10.0, 20.0], EntityDefaults::default());
        assert_eq!(id, "entity_1");

        let world = world.world_ref();
        let mut query = world.query::<(&SourceId, &Transform, &Sprite)>();
        let count = query.iter().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_removed_entity_is_gone() {
        let mut world = GameWorld::new();
        let (_, added_entity) = add_entity(&mut world, &[], [0.0, 0.0], EntityDefaults::default());
        let entity = added_entity;
        assert!(remove_entity(&mut world, entity));
        assert_eq!(world.world_ref().len(), 0);
    }

    #[test]
    fn a_role_can_be_set_on_an_entity() {
        let mut world = GameWorld::new();
        let (_, added_entity) = add_entity(&mut world, &[], [0.0, 0.0], EntityDefaults::default());
        let entity = added_entity;
        assert!(set_role(&mut world, entity, "player"));

        let world = world.world_ref();
        let mut query = world.query::<&Role>();
        let role = query.iter().next().expect("the role exists");
        assert_eq!(role.as_str(), "player");
    }
}
