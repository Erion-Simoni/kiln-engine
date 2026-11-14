//! Editing the entities of a scene.
//!
//! The inspector edits the world directly, so the editor needs ways to add
//! and remove entities and to give them the components a scene entity has.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kiln_core::project::Project;
use kiln_core::{PrefabInstance, Transform};
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

// Resource maps omit untextured/nonvisual entities. Always reserve live IDs too.
fn next_world_entity_id(world: &GameWorld, existing_ids: &[String]) -> String {
    let mut reserved = existing_ids.to_vec();
    reserved.extend(
        world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .map(|(_, id)| id.0.clone()),
    );
    next_entity_id(&reserved)
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
    let id = next_world_entity_id(world, existing_ids);
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
    world
        .insert_one(
            entity,
            kiln_runtime::Tilemap::new(
                "".to_string(),
                kiln_core::TileMap::default(),
                HashMap::new(),
                HashMap::new(),
            ),
        )
        .is_ok()
}

/// Remove the tilemap of an entity.
///
/// Returns whether the entity had one.
pub fn remove_tilemap(world: &mut GameWorld, entity: hecs::Entity) -> bool {
    world
        .world_mut()
        .remove_one::<kiln_runtime::Tilemap>(entity)
        .is_ok()
}

/// Build the project data of a prefab from an entity of the world.
///
/// The prefab keeps the entity's identity and components, so an instance of
/// it looks like the entity it was made from.
pub fn prefab_from_entity(entity: &kiln_core::project::Entity, name: &str) -> kiln_core::Prefab {
    kiln_core::Prefab::new(name.to_string(), entity.clone())
}

/// Attach runtime prefab links to scene entities carrying authored prefab markers.
pub fn attach_prefab_instances(world: &mut GameWorld, scene: &kiln_core::Scene) -> usize {
    let handles: HashMap<String, hecs::Entity> = world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .map(|(entity, source)| (source.0.clone(), entity))
        .collect();
    let markers: Vec<_> = scene
        .entities
        .iter()
        .filter_map(|entity| {
            entity
                .components
                .iter()
                .find_map(|component| match component {
                    kiln_core::Component::PrefabInstance {
                        prefab_id,
                        overrides,
                    } => Some((entity.id.clone(), prefab_id.clone(), overrides.clone())),
                    _ => None,
                })
        })
        .collect();
    let mut attached = 0;
    for (entity_id, prefab_id, overrides) in markers {
        if let Some(handle) = handles.get(&entity_id) {
            if world
                .world_mut()
                .insert_one(
                    *handle,
                    PrefabInstance {
                        prefab_id,
                        entity_id,
                        overrides,
                    },
                )
                .is_ok()
            {
                attached += 1;
            }
        }
    }
    attached
}

/// Add an instance of a prefab to the world.
///
/// The instance keeps the components of the prefab and gets its own source
/// id, so it is saved with the scene like every other entity.
/// A `PrefabInstance` component is added so that edits to the prefab can be
/// propagated to all instances.
pub fn instantiate_prefab(
    world: &mut GameWorld,
    prefab: &kiln_core::Prefab,
    existing_ids: &[String],
    position: [f32; 2],
) -> Option<String> {
    let instance = prefab.instantiate(None);
    let id = next_world_entity_id(world, existing_ids);

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
            kiln_core::project::Component::Sprite {
                color,
                width,
                height,
                animation_clip,
                ..
            } => {
                sprite =
                    Sprite::new(color[0], color[1], color[2], color[3]).with_size(*width, *height);
                if let Some(clip) = animation_clip {
                    sprite = sprite.with_animation_clip(clip);
                }
            }
            _ => {}
        }
    }

    let prefab_instance = PrefabInstance {
        prefab_id: prefab.id.clone(),
        entity_id: id.clone(),
        overrides: kiln_core::PrefabOverride::new(id.clone()),
    };

    world.spawn((
        transform,
        sprite,
        SourceId::new(id.clone()),
        prefab_instance,
    ));
    Some(id)
}

/// Re-apply a prefab to all ECS instances that reference it.
///
/// This finds all entities with a `PrefabInstance` component whose
/// `prefab_id` matches the given prefab, and rebuilds their transform and
/// sprite from the prefab data while preserving instance-level overrides.
pub fn update_prefab_instances(world: &mut GameWorld, prefab: &kiln_core::Prefab) -> usize {
    let mut updated = 0;

    // Collect entities that are instances of this prefab
    let instance_entities: Vec<hecs::Entity> = world
        .world_ref()
        .query::<(hecs::Entity, &PrefabInstance)>()
        .iter()
        .filter(|(_, prefab_instance)| prefab_instance.prefab_id == prefab.id)
        .map(|(entity, _)| entity)
        .collect();

    for entity in instance_entities {
        // Get the source id to find the overrides
        let _source_id = world
            .world_ref()
            .get::<&SourceId>(entity)
            .map(|sid| sid.0.clone())
            .unwrap_or_default();

        // Rebuild transform and sprite from prefab
        let mut transform = Transform::new(0.0, 0.0);
        let mut sprite = Sprite::new(1.0, 1.0, 1.0, 1.0).with_size(32.0, 32.0);

        for component in &prefab.root_entity.components {
            match component {
                kiln_core::project::Component::Transform {
                    x,
                    y,
                    rotation,
                    scale,
                    ..
                } => {
                    transform = Transform::new(*x, *y)
                        .with_rotation(*rotation)
                        .with_scale(kiln_core::Vec2::splat(*scale));
                }
                kiln_core::project::Component::Sprite {
                    color,
                    texture,
                    layer,
                    width,
                    height,
                    ..
                } => {
                    sprite = Sprite::new(color[0], color[1], color[2], color[3])
                        .with_size(*width, *height)
                        .with_layer(*layer);
                    if let Some(path) = texture {
                        // Texture override would need asset loading; skip for now
                        let _ = path;
                    }
                }
                _ => {}
            }
        }

        // Apply instance overrides
        if let Ok(prefab_inst) = world.world_mut().get::<&mut PrefabInstance>(entity) {
            for (comp_name, comp_override) in &prefab_inst.overrides.overrides {
                match comp_name.as_str() {
                    "transform" | "Transform" => {
                        if let kiln_core::prefab::ComponentOverride::Transform {
                            x,
                            y,
                            rotation,
                            scale,
                        } = comp_override
                        {
                            if let Some(v) = x {
                                transform.position.x = *v;
                            }
                            if let Some(v) = y {
                                transform.position.y = *v;
                            }
                            if let Some(v) = rotation {
                                transform.rotation = *v;
                            }
                            if let Some(v) = scale {
                                transform.scale = kiln_core::Vec2::splat(*v);
                            }
                        }
                    }
                    "sprite" | "Sprite" => {
                        if let kiln_core::prefab::ComponentOverride::Sprite { color, texture } =
                            comp_override
                        {
                            if let Some(c) = color {
                                sprite.color = *c;
                            }
                            if let Some(t) = texture {
                                let _ = t; // Texture override handled by asset system
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Write back to the entity
        let _ = world.world_mut().insert_one(entity, transform);
        let _ = world.world_mut().insert_one(entity, sprite);
        updated += 1;
    }

    updated
}

/// Propagate a changed prefab into every saved scene while preserving each
/// instance's identity, authored name, hierarchy parent, and overrides.
pub fn propagate_prefab_in_scenes(
    project: &mut kiln_core::Project,
    prefab: &kiln_core::Prefab,
) -> usize {
    let mut updated = 0;
    for scene in project.scenes.values_mut() {
        for entity in &mut scene.entities {
            let link = entity
                .components
                .iter()
                .find_map(|component| match component {
                    kiln_core::Component::PrefabInstance {
                        prefab_id,
                        overrides,
                    } if prefab_id == &prefab.id => Some((prefab_id.clone(), overrides.clone())),
                    _ => None,
                });
            let Some((prefab_id, overrides)) = link else {
                continue;
            };
            let parent = entity
                .components
                .iter()
                .find_map(|component| match component {
                    kiln_core::Component::Parent { parent } => Some(parent.clone()),
                    _ => None,
                });
            let mut propagated = prefab.root_entity.clone();
            propagated.id = entity.id.clone();
            propagated.name = entity.name.clone();
            propagated.components.retain(|component| {
                !matches!(
                    component,
                    kiln_core::Component::Parent { .. }
                        | kiln_core::Component::PrefabInstance { .. }
                )
            });
            prefab.apply_overrides(&mut propagated, &overrides);
            if let Some(parent) = parent {
                propagated
                    .components
                    .push(kiln_core::Component::Parent { parent });
            }
            propagated
                .components
                .push(kiln_core::Component::PrefabInstance {
                    prefab_id,
                    overrides,
                });
            crate::scene_controls::preserve_flags(entity, &mut propagated);
            *entity = propagated;
            updated += 1;
        }
    }
    updated
}

/// Apply one linked scene instance's supported authored components to its
/// prefab definition, clear that instance's overrides, and propagate the new
/// definition into every saved instance in the project.
pub fn apply_prefab_instance_to_definition(
    project: &mut kiln_core::Project,
    scene_id: &str,
    entity_id: &str,
) -> Result<kiln_core::Prefab, String> {
    let scene = project.scenes.get_mut(scene_id).ok_or("Scene is missing")?;
    let entities: Vec<_> = scene
        .entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| entity.id == entity_id)
        .map(|(index, _)| index)
        .collect();
    let [index] = entities.as_slice() else {
        return Err("Prefab instance identity is missing or ambiguous".into());
    };
    let prefab_id = scene.entities[*index]
        .components
        .iter()
        .find_map(|component| match component {
            kiln_core::Component::PrefabInstance { prefab_id, .. } => Some(prefab_id.clone()),
            _ => None,
        })
        .ok_or("Selected entity is not a prefab instance")?;
    let definition = project
        .prefabs
        .get_mut(&prefab_id)
        .ok_or("Prefab definition is missing")?;
    let mut root = scene.entities[*index].clone();
    root.id = definition.root_entity.id.clone();
    root.name = definition.root_entity.name.clone();
    root.components.retain(|component| {
        !matches!(
            component,
            kiln_core::Component::Parent { .. } | kiln_core::Component::PrefabInstance { .. }
        )
    });
    definition.root_entity = root;
    let updated = definition.clone();
    if let Some(kiln_core::Component::PrefabInstance { overrides, .. }) = scene.entities[*index]
        .components
        .iter_mut()
        .find(|component| matches!(component, kiln_core::Component::PrefabInstance { .. }))
    {
        *overrides = kiln_core::PrefabOverride::new(entity_id.to_owned());
    }
    let scenes = &mut project.scenes;
    for saved_scene in scenes.values_mut() {
        for instance in &mut saved_scene.entities {
            let linked = instance.components.iter().any(|component| {
                matches!(component, kiln_core::Component::PrefabInstance { prefab_id: id, .. } if id == &updated.id)
            });
            if !linked || (saved_scene.id == scene_id && instance.id == entity_id) {
                continue;
            }
            let mut propagated = updated.root_entity.clone();
            propagated.id = instance.id.clone();
            propagated.name = instance.name.clone();
            let parent = instance
                .components
                .iter()
                .find_map(|component| match component {
                    kiln_core::Component::Parent { parent } => Some(parent.clone()),
                    _ => None,
                });
            let overrides = instance
                .components
                .iter()
                .find_map(|component| match component {
                    kiln_core::Component::PrefabInstance { overrides, .. } => {
                        Some(overrides.clone())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| kiln_core::PrefabOverride::new(instance.id.clone()));
            updated.apply_overrides(&mut propagated, &overrides);
            if let Some(parent) = parent {
                propagated
                    .components
                    .push(kiln_core::Component::Parent { parent });
            }
            propagated
                .components
                .push(kiln_core::Component::PrefabInstance {
                    prefab_id: updated.id.clone(),
                    overrides,
                });
            crate::scene_controls::preserve_flags(instance, &mut propagated);
            *instance = propagated;
        }
    }
    Ok(updated)
}

/// Update authored Sprite, audio and UI font paths after a project-relative asset move.
pub fn rewrite_asset_references(project: &mut Project, old: &str, new: &str) -> usize {
    project.rewrite_resource_path(old, new)
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
    if scene_reaches(project, child_id, parent_id, &mut HashSet::new()) {
        return Err("Nesting this scene would create a scene cycle".into());
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

/// Remove a nested-scene link without deleting either scene document.
pub fn unnest_scene(
    project: &mut Project,
    parent_id: &str,
    child_id: &str,
) -> Result<bool, String> {
    let parent = project
        .scenes
        .get_mut(parent_id)
        .ok_or("Parent scene is missing")?;
    let before = parent.nested_scenes.len();
    parent
        .nested_scenes
        .retain(|nested| nested.scene_id != child_id);
    Ok(parent.nested_scenes.len() != before)
}

/// Update a nested scene's offset and additive loading mode.
pub fn configure_nested_scene(
    project: &mut Project,
    parent_id: &str,
    child_id: &str,
    offset: [f32; 2],
    additive: bool,
) -> Result<(), String> {
    if !offset.iter().all(|value| value.is_finite()) {
        return Err("Nested scene offset must be finite".into());
    }
    let parent = project
        .scenes
        .get_mut(parent_id)
        .ok_or("Parent scene is missing")?;
    let nested = parent
        .nested_scenes
        .iter_mut()
        .find(|nested| nested.scene_id == child_id)
        .ok_or("Nested scene link is missing")?;
    nested.offset = kiln_core::project::SceneOffset {
        x: offset[0],
        y: offset[1],
    };
    nested.additive = additive;
    Ok(())
}

fn scene_reaches(
    project: &Project,
    from: &str,
    target: &str,
    visited: &mut HashSet<String>,
) -> bool {
    if from == target {
        return true;
    }
    if !visited.insert(from.to_owned()) {
        return false;
    }
    project.scenes.get(from).is_some_and(|scene| {
        scene
            .nested_scenes
            .iter()
            .any(|nested| scene_reaches(project, &nested.scene_id, target, visited))
    })
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

    fn empty_scene(id: &str) -> kiln_core::Scene {
        kiln_core::Scene {
            id: id.into(),
            name: id.into(),
            entities: Vec::new(),
            script: None,
            nested_scenes: Vec::new(),
        }
    }

    #[test]
    fn nested_scenes_reject_cycles_and_support_configuration_and_removal() {
        let mut project = Project::default();
        for id in ["a", "b", "c"] {
            project.scenes.insert(id.into(), empty_scene(id));
        }
        nest_scene(&mut project, "a", "b").unwrap();
        nest_scene(&mut project, "b", "c").unwrap();
        assert!(nest_scene(&mut project, "c", "a").is_err());
        configure_nested_scene(&mut project, "a", "b", [15.0, -3.0], true).unwrap();
        let link = &project.scenes["a"].nested_scenes[0];
        assert_eq!([link.offset.x, link.offset.y], [15.0, -3.0]);
        assert!(link.additive);
        assert!(configure_nested_scene(&mut project, "a", "b", [f32::NAN, 0.0], false).is_err());
        assert!(unnest_scene(&mut project, "a", "b").unwrap());
        assert!(project.scenes["a"].nested_scenes.is_empty());
    }

    #[test]
    fn moved_asset_paths_update_scenes_and_prefab_definitions() {
        let mut project = Project::default();
        let sprite_entity = |id: &str| kiln_core::Entity {
            id: id.into(),
            name: id.into(),
            components: vec![kiln_core::Component::Sprite {
                color: [1.0; 4],
                width: 16.0,
                height: 16.0,
                texture: Some("art/old.png".into()),
                layer: 0.0,
                material: None,
                animation_clip: None,
            }],
        };
        let mut scene = empty_scene("main");
        scene.entities.push(sprite_entity("scene-sprite"));
        project.scenes.insert(scene.id.clone(), scene);
        project.prefabs.insert(
            "prefab".into(),
            kiln_core::Prefab {
                id: "prefab".into(),
                name: "P".into(),
                root_entity: sprite_entity("template"),
            },
        );
        assert_eq!(
            rewrite_asset_references(&mut project, "art/old.png", "art/new.png"),
            2
        );
        assert!(
            matches!(&project.scenes["main"].entities[0].components[0], kiln_core::Component::Sprite { texture: Some(path), .. } if path == "art/new.png")
        );
        assert!(
            matches!(&project.prefabs["prefab"].root_entity.components[0], kiln_core::Component::Sprite { texture: Some(path), .. } if path == "art/new.png")
        );
    }

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
