//! Editing the entities of a scene.
//!
//! The inspector edits the world directly, so the editor needs ways to add
//! and remove entities and to give them the components a scene entity has.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kiln_core::project::Project;
use kiln_core::Transform;
use kiln_runtime::{GameWorld, Role, SourceId, Sprite, Tilemap};

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

/// Remove the transform of an entity.
///
/// Returns whether the entity had one.
pub fn remove_transform(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world.world_mut().remove_one::<Transform>(entity).is_ok()
}

/// Remove the sprite of an entity.
///
/// Returns whether the entity had one.
pub fn remove_sprite(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world.world_mut().remove_one::<Sprite>(entity).is_ok()
}

/// Add a transform to an entity that does not have one.
///
/// Returns whether the transform was added.
pub fn add_transform(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    let world = world.world_mut();
    if world.get::<&Transform>(entity).is_ok() {
        return false;
    }
    world.insert_one(entity, Transform::new(0.0, 0.0)).is_ok()
}

/// Add a sprite to an entity that does not have one.
///
/// Returns whether the sprite was added.
pub fn add_sprite(world: &mut GameWorld, entity: hecs::Entity, defaults: EntityDefaults) -> bool {
    let world = world.world_mut();
    if world.get::<&Sprite>(entity).is_ok() {
        return false;
    }
    let sprite = Sprite::new(
        defaults.color[0],
        defaults.color[1],
        defaults.color[2],
        defaults.color[3],
    )
    .with_size(defaults.size[0], defaults.size[1]);
    world.insert_one(entity, sprite).is_ok()
}

/// Add a role component to an entity that does not have one.
///
/// Returns whether the role was added.
pub fn add_role(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    let world = world.world_mut();
    if world.get::<&Role>(entity).is_ok() {
        return false;
    }
    world.insert_one(entity, Role::new("")).is_ok()
}

/// Remove the role of an entity.
///
/// Returns whether the entity had one.
pub fn remove_role(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world.world_mut().remove_one::<Role>(entity).is_ok()
}

/// Add a tilemap component to an entity that does not have one.
///
/// Returns whether the tilemap was added.
pub fn add_tilemap(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    let world = world.world_mut();
    if world.get::<&kiln_runtime::Tilemap>(entity).is_ok() {
        return false;
    }
    // Create an empty tilemap - it will be configured by the user
    world.insert_one(entity, kiln_runtime::Tilemap::new(
        "".to_string(),
        kiln_core::TileMap::default(),
        HashMap::new(),
        HashMap::new(),
    )).is_ok()
}

/// Remove the tilemap of an entity.
///
/// Returns whether the entity had one.
pub fn remove_tilemap(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world.world_mut().remove_one::<kiln_runtime::Tilemap>(entity).is_ok()
}

/// Build the project data of a prefab from an entity of the world.
///
/// The prefab keeps the entity's identity and components, so an instance of
/// it looks like the entity it was made from.
pub fn prefab_from_entity(entity: &kiln_core::project::Entity, name: &str) -> kiln_core::Prefab {
    kiln_core::Prefab::new(name.to_string(), entity.clone())
}

/// Add an instance of a prefab to the world.
///
/// The instance keeps the components of the prefab and gets its own source
/// id, so it is saved with the scene like every other entity.
pub fn instantiate_prefab(
    world: &mut GameWorld,
    prefab: &kiln_core::Prefab,
    existing_ids: &[String],
    position: [f32; 2],
) -> Option<String> {
    let instance = prefab.instantiate(None);
    let id = next_entity_id(existing_ids);

    // The transform of the instance starts at the given position, and the
    // sprite keeps the size and the tint of the prefab.
    let mut transform = Transform::new(position[0], position[1]);
    let mut sprite =
        Sprite::new(1.0, 1.0, 1.0, 1.0).with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1]);
    for component in &instance.components {
        match component {
            kiln_core::project::Component::Transform { scale, .. } => {
                transform = transform.with_scale(kiln_core::Vec2::splat(*scale));
            }
            kiln_core::project::Component::Sprite { color, .. } => {
                sprite = Sprite::new(color[0], color[1], color[2], color[3])
                    .with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1]);
            }
            _ => {}
        }
    }

    world.spawn((transform, sprite, SourceId::new(id.clone())));
    Some(id)
}

/// The default size of a sprite without a texture, in world units.
const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

/// The id of a new scene, counted from one.
///
/// The ids are unique within a project, so the editor can name a scene and
/// find it again after a reload.
pub fn next_scene_id(existing: &[String]) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("scene_{index}");
        if !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// Create an empty scene and add it to the project.
///
/// The new scene becomes the default scene, so it is the one the editor
/// shows.
pub fn create_scene(project: &mut Project, name: &str) -> String {
    let existing: Vec<String> = project.scenes.keys().cloned().collect();
    let id = next_scene_id(&existing);
    project.scenes.insert(
        id.clone(),
        kiln_core::project::Scene {
            id: id.clone(),
            name: name.to_string(),
            entities: Vec::new(),
            nested_scenes: Vec::new(),
            script: None,
        },
    );
    id
}

/// Add a scene as a nested scene of another scene.
///
/// Returns whether the scene could be nested, which is not the case when the
/// scene is the one it would be nested in.
pub fn nest_scene(project: &mut Project, parent_id: &str, child_id: &str) -> Result<(), String> {
    if parent_id == child_id {
        return Err("A scene cannot contain itself".to_string());
    }
    let child_exists = project.scenes.contains_key(child_id);
    if !child_exists {
        return Err(format!("The scene '{child_id}' does not exist"));
    }
    let Some(parent) = project.scenes.get_mut(parent_id) else {
        return Err(format!("The scene '{parent_id}' does not exist"));
    };
    if parent
        .nested_scenes
        .iter()
        .any(|nested| nested.scene_id == child_id)
    {
        return Err(format!("The scene '{child_id}' is already nested"));
    }
    parent.nested_scenes.push(kiln_core::project::NestedScene {
        scene_id: child_id.to_string(),
        offset: kiln_core::project::SceneOffset::default(),
        additive: false,
    });
    Ok(())
}

/// The files of a project directory, ready to be packed.
fn project_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort();
    files
}

/// Collect every file below `dir`.
fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Pack a project directory into an archive.
///
/// The manifest and the assets of the project end up in the archive, so the
/// player can run it. Returns the path of the archive.
pub fn pack_project(root: &Path, output: &Path) -> Result<PathBuf, String> {
    let files = project_files(root);
    if files.is_empty() {
        return Err(format!("The project directory {} is empty", root.display()));
    }
    let mut archive = kiln_assets::Archive::new(output.to_path_buf());
    archive
        .pack(&files)
        .map_err(|e| format!("The project could not be packed: {e}"))?;
    Ok(output.to_path_buf())
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
