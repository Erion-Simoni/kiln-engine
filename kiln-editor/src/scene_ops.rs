//! Pure scene editing operations for the editor.
//!
//! The editor keeps the scene in two places at once: the runtime world
//! (which the renderer draws) and the project data (which is written to
//! `kiln.toml`). These helpers move data between the two and provide the
//! small, well-defined edits the inspector and the scene panels perform.
//!
//! Everything here is free of `egui` and of the GPU, so it can be tested
//! without a window or an adapter.

use kiln_core::project::{Component, Entity};
use kiln_core::Transform;
use kiln_runtime::{GameWorld, Role, SourceId, Sprite};
use std::collections::HashMap;

/// The parts of an entity the editor tracks besides its components.
///
/// The runtime world stores a texture id and a role, but the project data
/// stores an asset path and a role name, so the editor keeps the mapping
/// between the two.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityExtras {
    /// Asset path of the sprite texture, if the entity has one.
    pub texture: Option<String>,
    /// Gameplay role of the entity, if it has one.
    pub role: Option<String>,
}

/// Everything the editor knows about one scene entity.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEntity {
    /// The stable id of the entity, used as its `SourceId`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Position, rotation and scale.
    pub transform: Transform,
    /// Tint of the sprite, when the entity has one.
    pub color: Option<[f32; 4]>,
    /// Size of the sprite in pixels.
    pub size: [f32; 2],
    /// Texture path and role.
    pub extras: EntityExtras,
}

impl SceneEntity {
    /// A new entity with a white sprite at the origin.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transform: Transform::default(),
            color: Some([1.0, 1.0, 1.0, 1.0]),
            size: [64.0, 64.0],
            extras: EntityExtras::default(),
        }
    }

    /// Convert to the project's entity representation.
    pub fn to_project_entity(&self) -> Entity {
        let mut components = vec![Component::Transform {
            x: self.transform.position.x,
            y: self.transform.position.y,
            rotation: self.transform.rotation,
            scale: self.transform.scale.x,
        }];

        if let Some(color) = self.color {
            components.push(Component::Sprite {
                color,
                texture: self.extras.texture.clone(),
            });
        }
        if let Some(role) = &self.extras.role {
            components.push(Component::Role { role: role.clone() });
        }

        Entity {
            id: self.id.clone(),
            name: self.name.clone(),
            components,
        }
    }

    /// Read an entity from the project's entity representation.
    pub fn from_project_entity(entity: &Entity) -> Self {
        let mut transform = Transform::default();
        let mut color = None;
        let mut texture = None;
        let mut role = None;

        for component in &entity.components {
            match component {
                Component::Transform {
                    x,
                    y,
                    rotation,
                    scale,
                } => {
                    transform = Transform::new(*x, *y)
                        .with_rotation(*rotation)
                        .with_scale(kiln_core::Vec2::splat(*scale));
                }
                Component::Sprite {
                    color: c,
                    texture: t,
                } => {
                    color = Some(*c);
                    texture = t.clone();
                }
                Component::Role { role: r } => role = Some(r.clone()),
                Component::Tilemap { .. } => {}
            }
        }

        Self {
            id: entity.id.clone(),
            name: entity.name.clone(),
            transform,
            color,
            size: [64.0, 64.0],
            extras: EntityExtras { texture, role },
        }
    }
}

/// The editable scene: a list of entities plus the per-entity extras.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// The entities of the scene, in order.
    pub entities: Vec<SceneEntity>,
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up an entity by its id.
    pub fn entity(&self, id: &str) -> Option<&SceneEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Look up an entity by its id, for editing.
    pub fn entity_mut(&mut self, id: &str) -> Option<&mut SceneEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Add an entity and return its id.
    ///
    /// The id is made unique by appending a counter when the name is taken.
    pub fn add_entity(&mut self, mut entity: SceneEntity) -> String {
        if self.entity(&entity.id).is_some() {
            let base = entity.id.clone();
            let mut counter = 1;
            while self.entity(&format!("{base}_{counter}")).is_some() {
                counter += 1;
            }
            entity.id = format!("{base}_{counter}");
        }
        let id = entity.id.clone();
        self.entities.push(entity);
        id
    }

    /// Remove an entity by its id; returns whether it existed.
    pub fn remove_entity(&mut self, id: &str) -> bool {
        let before = self.entities.len();
        self.entities.retain(|e| e.id != id);
        self.entities.len() != before
    }

    /// Add a component to an entity; returns whether the entity exists.
    ///
    /// Adding a component that is already there replaces its value, which is
    /// what the inspector's "add component" menu does.
    pub fn add_component(&mut self, id: &str, component: Component) -> bool {
        let Some(entity) = self.entity_mut(id) else {
            return false;
        };
        match &component {
            Component::Transform {
                x,
                y,
                rotation,
                scale,
            } => {
                entity.transform = Transform::new(*x, *y)
                    .with_rotation(*rotation)
                    .with_scale(kiln_core::Vec2::splat(*scale));
            }
            Component::Sprite { color, texture } => {
                entity.color = Some(*color);
                if texture.is_some() {
                    entity.extras.texture = texture.clone();
                }
            }
            Component::Role { role } => {
                entity.extras.role = Some(role.clone());
            }
            Component::Tilemap { .. } => {}
        }
        true
    }

    /// Remove a component from an entity; returns whether it was there.
    pub fn remove_component(&mut self, id: &str, kind: ComponentKind) -> bool {
        let Some(entity) = self.entity_mut(id) else {
            return false;
        };
        match kind {
            ComponentKind::Transform => {
                entity.transform = Transform::default();
                true
            }
            ComponentKind::Sprite => {
                let had = entity.color.is_some();
                entity.color = None;
                entity.extras.texture = None;
                had
            }
            ComponentKind::Role => {
                let had = entity.extras.role.is_some();
                entity.extras.role = None;
                had
            }
        }
    }

    /// Convert the scene to the project's entity list.
    pub fn to_project_entities(&self) -> Vec<Entity> {
        self.entities
            .iter()
            .map(SceneEntity::to_project_entity)
            .collect()
    }

    /// Build a scene from the project's entity list.
    pub fn from_project_entities(entities: &[Entity]) -> Self {
        Self {
            entities: entities
                .iter()
                .map(SceneEntity::from_project_entity)
                .collect(),
        }
    }
}

/// The kinds of component the inspector can add or remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// Position, rotation and scale.
    Transform,
    /// Tint and texture.
    Sprite,
    /// Gameplay role.
    Role,
}

impl ComponentKind {
    /// The label shown in the inspector.
    pub fn label(self) -> &'static str {
        match self {
            ComponentKind::Transform => "Transform",
            ComponentKind::Sprite => "Sprite",
            ComponentKind::Role => "Role",
        }
    }
}

/// Read the editable scene out of the runtime world.
///
/// Entities without a `SourceId` are skipped: they are not part of the
/// project's scene and would not survive a save.
pub fn scene_from_world(world: &GameWorld) -> Scene {
    let mut scene = Scene::new();
    let world = world.world_ref();

    for (entity, source_id, transform) in world
        .query::<(hecs::Entity, &SourceId, &Transform)>()
        .iter()
    {
        let sprite = world.get::<&Sprite>(entity).ok();
        let color = sprite.as_ref().map(|s| s.color);
        let size = sprite.as_ref().map(|s| s.size);
        let role = world.get::<&Role>(entity).ok().map(|r| r.0.clone());

        scene.entities.push(SceneEntity {
            id: source_id.0.clone(),
            name: source_id.0.clone(),
            transform: *transform,
            color,
            size: size.unwrap_or([64.0, 64.0]),
            extras: EntityExtras {
                texture: None,
                role,
            },
        });
    }

    scene
}

/// Apply the texture paths of the editor to a scene read from the world.
///
/// The runtime world does not know asset paths, so they are filled in from
/// the editor's own mapping.
pub fn apply_textures(scene: &mut Scene, textures: &HashMap<String, String>) {
    for entity in &mut scene.entities {
        if let Some(texture) = textures.get(&entity.id) {
            entity.extras.texture = Some(texture.clone());
        }
    }
}

/// Rebuild the runtime world from an editable scene.
///
/// The world is emptied first, so the result contains exactly the entities of
/// the scene. The returned map holds the texture path of every entity, which
/// the editor keeps so that saving the scene does not lose it.
pub fn world_from_scene(
    world: &mut GameWorld,
    scene: &Scene,
    sizes: &HashMap<String, (u32, u32)>,
) -> HashMap<String, String> {
    world.world_mut().clear();

    let mut textures = HashMap::new();
    let mut entities = Vec::new();

    for entity in &scene.entities {
        if let Some(texture) = &entity.extras.texture {
            textures.insert(entity.id.clone(), texture.clone());
        }
        entities.push(entity.to_project_entity());
    }

    let mut make_sprite = |_entity: &Entity, color: [f32; 4], texture: Option<&str>| {
        let size = texture
            .and_then(|path| sizes.get(path))
            .copied()
            .unwrap_or((64, 64));
        Sprite::new(color[0], color[1], color[2], color[3]).with_size(size.0 as f32, size.1 as f32)
    };

    kiln_runtime::spawn_entities(
        world,
        &entities,
        kiln_core::project::SceneOffset::default(),
        &mut make_sprite,
    );

    textures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_entity_has_a_white_sprite() {
        let entity = SceneEntity::new("hero", "Hero");
        assert_eq!(entity.color, Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(entity.transform.position, kiln_core::Vec2::ZERO);
    }

    #[test]
    fn project_entities_round_trip() {
        let mut entity = SceneEntity::new("hero", "Hero");
        entity.transform = Transform::new(10.0, -20.0).with_rotation(1.5);
        entity.color = Some([0.1, 0.2, 0.3, 0.4]);
        entity.extras.texture = Some("sprites/hero.png".to_string());
        entity.extras.role = Some("player".to_string());

        let project_entity = entity.to_project_entity();
        let back = SceneEntity::from_project_entity(&project_entity);

        assert_eq!(back.id, "hero");
        assert_eq!(back.name, "Hero");
        assert_eq!(back.transform.position.x, 10.0);
        assert_eq!(back.transform.position.y, -20.0);
        assert_eq!(back.transform.rotation, 1.5);
        assert_eq!(back.color, Some([0.1, 0.2, 0.3, 0.4]));
        assert_eq!(back.extras.texture.as_deref(), Some("sprites/hero.png"));
        assert_eq!(back.extras.role.as_deref(), Some("player"));
    }

    #[test]
    fn adding_an_entity_keeps_ids_unique() {
        let mut scene = Scene::new();
        scene.add_entity(SceneEntity::new("hero", "Hero"));
        let second = scene.add_entity(SceneEntity::new("hero", "Hero 2"));

        assert_eq!(second, "hero_1");
        assert_eq!(scene.entities.len(), 2);
    }

    #[test]
    fn removing_an_entity_reports_whether_it_existed() {
        let mut scene = Scene::new();
        scene.add_entity(SceneEntity::new("hero", "Hero"));

        assert!(scene.remove_entity("hero"));
        assert!(!scene.remove_entity("hero"));
        assert!(scene.entities.is_empty());
    }

    #[test]
    fn adding_a_component_sets_its_value() {
        let mut scene = Scene::new();
        scene.add_entity(SceneEntity::new("hero", "Hero"));

        assert!(scene.add_component(
            "hero",
            Component::Role {
                role: "player".to_string()
            }
        ));
        assert_eq!(
            scene.entity("hero").unwrap().extras.role.as_deref(),
            Some("player")
        );
    }

    #[test]
    fn adding_a_component_to_a_missing_entity_fails() {
        let mut scene = Scene::new();
        assert!(!scene.add_component(
            "ghost",
            Component::Role {
                role: "player".to_string()
            }
        ));
    }

    #[test]
    fn removing_the_sprite_component_clears_its_texture() {
        let mut scene = Scene::new();
        let mut entity = SceneEntity::new("hero", "Hero");
        entity.extras.texture = Some("sprites/hero.png".to_string());
        scene.add_entity(entity);

        assert!(scene.remove_component("hero", ComponentKind::Sprite));
        let entity = scene.entity("hero").unwrap();
        assert_eq!(entity.color, None);
        assert_eq!(entity.extras.texture, None);
        // The role was not touched.
        assert!(!scene.remove_component("hero", ComponentKind::Role));
    }

    #[test]
    fn a_scene_reads_back_from_the_world() {
        let mut world = GameWorld::new();
        world.spawn((
            Transform::new(5.0, 6.0),
            Sprite::new(1.0, 0.5, 0.25, 1.0),
            Role::new("player"),
            SourceId::new("hero"),
        ));
        world.spawn((Transform::new(1.0, 1.0), Sprite::new(1.0, 1.0, 1.0, 1.0)));

        let scene = scene_from_world(&world);

        // The entity without a SourceId is not part of the scene.
        assert_eq!(scene.entities.len(), 1);
        let hero = scene.entity("hero").expect("hero is in the scene");
        assert_eq!(hero.transform.position.x, 5.0);
        assert_eq!(hero.color, Some([1.0, 0.5, 0.25, 1.0]));
        assert_eq!(hero.extras.role.as_deref(), Some("player"));
    }

    #[test]
    fn a_world_is_rebuilt_from_a_scene() {
        let mut world = GameWorld::new();
        world.spawn((
            Transform::new(5.0, 6.0),
            Sprite::new(1.0, 1.0, 1.0, 1.0),
            SourceId::new("stale"),
        ));

        let mut scene = Scene::new();
        let mut hero = SceneEntity::new("hero", "Hero");
        hero.transform = Transform::new(1.0, 2.0);
        hero.extras.texture = Some("sprites/hero.png".to_string());
        scene.add_entity(hero);

        let sizes = HashMap::from([("sprites/hero.png".to_string(), (32, 48))]);
        let textures = world_from_scene(&mut world, &scene, &sizes);

        assert_eq!(world.world_ref().len(), 1);
        assert_eq!(
            textures.get("hero").map(String::as_str),
            Some("sprites/hero.png")
        );

        let world_ref = world.world_ref();
        let mut query = world_ref.query::<(&SourceId, &Transform, &Sprite)>();
        let (id, transform, sprite) = query.iter().next().expect("the hero is in the world");
        assert_eq!(id.0, "hero");
        assert_eq!(transform.position.x, 1.0);
        // The sprite is as big as its texture.
        assert_eq!(sprite.size, [32.0, 48.0]);
    }

    #[test]
    fn textures_are_applied_to_a_scene_read_from_the_world() {
        let mut world = GameWorld::new();
        world.spawn((
            Transform::default(),
            Sprite::new(1.0, 1.0, 1.0, 1.0),
            SourceId::new("hero"),
        ));

        let mut scene = scene_from_world(&world);
        let textures = HashMap::from([("hero".to_string(), "sprites/hero.png".to_string())]);
        apply_textures(&mut scene, &textures);

        assert_eq!(
            scene.entity("hero").unwrap().extras.texture.as_deref(),
            Some("sprites/hero.png")
        );
    }
}
