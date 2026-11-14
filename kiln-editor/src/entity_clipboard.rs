//! Reference-safe, project-scoped entity clipboard transactions.
use kiln_core::{Component, Entity, Project, Scene};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub(crate) const PREFIX: &str = "KILN_ENTITIES_V1\n";
const LIMIT: usize = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Clipboard {
    project: String,
    entities: Vec<Entity>,
}

/// Capture selected subtrees; detached roots retain their world transform.
pub(crate) fn copy(
    project: &Project,
    scene: &Scene,
    selected: &[String],
) -> Result<String, String> {
    let mut ids: HashSet<_> = selected.iter().cloned().collect();
    if ids.is_empty()
        || ids
            .iter()
            .any(|id| !scene.entities.iter().any(|e| &e.id == id))
    {
        return Err("Select existing scene entities to copy".into());
    }
    loop {
        let before = ids.len();
        for entity in &scene.entities {
            if entity
                .components
                .iter()
                .any(|c| matches!(c, Component::Parent {parent} if ids.contains(parent)))
            {
                ids.insert(entity.id.clone());
            }
        }
        if before == ids.len() {
            break;
        }
    }
    let world = kiln_core::hierarchy_transform::world_transforms(scene)?;
    let mut entities: Vec<_> = scene
        .entities
        .iter()
        .filter(|e| ids.contains(&e.id))
        .cloned()
        .collect();
    for entity in &mut entities {
        let detached = entity
            .components
            .iter()
            .any(|c| matches!(c, Component::Parent {parent} if !ids.contains(parent)));
        if detached {
            entity
                .components
                .retain(|c| !matches!(c, Component::Parent { .. }));
            let transform = world[&entity.id];
            crate::entity_command::set_local_transform(entity, transform)?;
        }
    }
    let text = PREFIX.to_owned()
        + &serde_json::to_string(&Clipboard {
            project: project.id.clone(),
            entities,
        })
        .map_err(|e| e.to_string())?;
    if text.len() > LIMIT {
        return Err("Entity clipboard exceeds 8 MiB".into());
    }
    Ok(text)
}

/// Apply to a staged scene. The surrounding command owns atomicity and undo.
pub(crate) fn paste(
    scene: &mut Scene,
    project: &Project,
    text: &str,
) -> Result<Option<String>, String> {
    if text.len() > LIMIT {
        return Err("Entity clipboard exceeds 8 MiB".into());
    }
    let body = text
        .strip_prefix(PREFIX)
        .ok_or("Clipboard is not a Kiln entity selection")?;
    let data: Clipboard = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if data.project != project.id || data.entities.is_empty() {
        return Err("Copy entities from the current project before pasting".into());
    }
    let mut used: Vec<_> = project
        .scenes
        .values()
        .flat_map(|s| s.entities.iter().map(|e| e.id.clone()))
        .collect();
    used.extend(scene.entities.iter().map(|e| e.id.clone()));
    let mut remap = HashMap::new();
    for entity in &data.entities {
        if entity.id.is_empty() || remap.contains_key(&entity.id) {
            return Err("Clipboard has ambiguous entity identities".into());
        }
        let id = crate::editing::next_entity_id(&used);
        used.push(id.clone());
        remap.insert(entity.id.clone(), id);
    }
    let first = data.entities.first().map(|e| remap[&e.id].clone());
    for mut entity in data.entities {
        entity.id = remap[&entity.id].clone();
        entity.name.push_str(" Copy");
        let root = !entity
            .components
            .iter()
            .any(|c| matches!(c, Component::Parent { .. }));
        if root
            && !entity
                .components
                .iter()
                .any(|c| matches!(c, Component::Transform { .. }))
        {
            entity.components.push(Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            });
        }
        for component in &mut entity.components {
            match component {
                Component::Parent { parent } => {
                    *parent = remap
                        .get(parent)
                        .cloned()
                        .ok_or("Clipboard parent is outside its selection")?
                }
                Component::Transform { x, y, .. } if root => {
                    *x += 16.0;
                    *y += 16.0;
                }
                Component::Joint { joint } => {
                    for endpoint in [&mut joint.entity_a, &mut joint.entity_b] {
                        if let Some(id) = remap.get(endpoint) {
                            *endpoint = id.clone();
                        } else if !scene.entities.iter().any(|e| &e.id == endpoint) {
                            return Err(
                                "Clipboard joint references an entity outside this scene".into()
                            );
                        }
                    }
                }
                Component::PrefabInstance {
                    prefab_id,
                    overrides,
                } => {
                    if !project.prefabs.contains_key(prefab_id) {
                        return Err("Clipboard prefab definition is missing".into());
                    }
                    overrides.entity_id = entity.id.clone();
                }
                _ => {}
            }
        }
        scene.entities.push(entity);
    }
    kiln_core::hierarchy_transform::world_transforms(scene)?;
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity_command::{EntityCommand, EntityEdit},
        history::SceneHistory,
    };
    fn fixture() -> (Project, Scene) {
        let scene = Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                Entity {
                    id: "parent".into(),
                    name: "Parent".into(),
                    components: vec![Component::Transform {
                        x: 100.0,
                        y: 50.0,
                        rotation: 0.0,
                        scale: 2.0,
                    }],
                },
                Entity {
                    id: "child".into(),
                    name: "Child".into(),
                    components: vec![
                        Component::Parent {
                            parent: "parent".into(),
                        },
                        Component::Transform {
                            x: 5.0,
                            y: 3.0,
                            rotation: 0.0,
                            scale: 1.0,
                        },
                        Component::Role {
                            role: "enemy".into(),
                        },
                    ],
                },
            ],
        };
        let mut project = Project::default();
        project.scenes.insert(scene.id.clone(), scene.clone());
        (project, scene)
    }
    #[test]
    fn subtree_paste_remaps_parents_preserves_components_and_is_one_undo() {
        let (project, mut scene) = fixture();
        let original = serde_json::to_value(&scene).unwrap();
        let text = copy(&project, &scene, &["parent".into()]).unwrap();
        let mut history = SceneHistory::new();
        let id = EntityCommand {
            scene_id: scene.id.clone(),
            edit: EntityEdit::Paste { document: text },
        }
        .execute(&mut scene, &project, &mut history)
        .unwrap()
        .unwrap();
        assert_eq!(scene.entities.len(), 4);
        let pasted_child = &scene.entities[3];
        assert_ne!(pasted_child.id, "child");
        assert!(pasted_child
            .components
            .iter()
            .any(|c| matches!(c,Component::Parent {parent} if parent==&id)));
        assert!(pasted_child
            .components
            .iter()
            .any(|c| matches!(c,Component::Role {role} if role=="enemy")));
        let world = kiln_core::hierarchy_transform::world_transforms(&scene).unwrap();
        assert_eq!(
            world[&pasted_child.id].position,
            kiln_core::Vec2::new(126.0, 72.0)
        );
        let restored = history.undo(scene.clone()).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), original);
        let redone = history.redo(restored).unwrap();
        assert_eq!(redone.entities.len(), 4);
    }
    #[test]
    fn child_only_copy_detaches_with_world_pose_and_pastes_into_another_scene() {
        let (mut project, scene) = fixture();
        let text = copy(&project, &scene, &["child".into()]).unwrap();
        let mut other = Scene {
            id: "other".into(),
            name: "Other".into(),
            entities: vec![],
            script: None,
            nested_scenes: vec![],
        };
        project.scenes.insert(other.id.clone(), other.clone());
        EntityCommand {
            scene_id: other.id.clone(),
            edit: EntityEdit::Paste { document: text },
        }
        .execute(&mut other, &project, &mut SceneHistory::new())
        .unwrap();
        assert_eq!(other.entities.len(), 1);
        assert!(!other.entities[0]
            .components
            .iter()
            .any(|c| matches!(c, Component::Parent { .. })));
        let world = kiln_core::hierarchy_transform::world_transforms(&other).unwrap();
        assert_eq!(
            world[&other.entities[0].id].position,
            kiln_core::Vec2::new(126.0, 72.0)
        );
    }
    #[test]
    fn a_child_without_a_transform_keeps_its_inherited_world_pose() {
        let (project, mut scene) = fixture();
        scene.entities[1]
            .components
            .retain(|c| !matches!(c, Component::Transform { .. }));
        let text = copy(&project, &scene, &["child".into()]).unwrap();
        let id = paste(&mut scene, &project, &text).unwrap().unwrap();
        let world = kiln_core::hierarchy_transform::world_transforms(&scene).unwrap();
        assert_eq!(world[&id].position, kiln_core::Vec2::new(116.0, 66.0));
        assert_eq!(world[&id].scale, kiln_core::Vec2::new(2.0, 2.0));
    }
    #[test]
    fn invalid_clipboard_and_cross_project_paste_leave_scene_and_history_untouched() {
        let (project, mut scene) = fixture();
        let text = copy(&project, &scene, &["parent".into()]).unwrap();
        let mut foreign = project.clone();
        foreign.id.push_str("different");
        let original = serde_json::to_value(&scene).unwrap();
        let mut history = SceneHistory::new();
        for document in ["text from another app".to_string(), text] {
            assert!(EntityCommand {
                scene_id: scene.id.clone(),
                edit: EntityEdit::Paste { document }
            }
            .execute(&mut scene, &foreign, &mut history)
            .is_err());
            assert_eq!(serde_json::to_value(&scene).unwrap(), original);
            assert!(!history.can_undo());
        }
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    use crate::{
        entity_command::{EntityCommand, EntityEdit},
        history::SceneHistory,
    };
    #[test]
    fn prefab_and_joint_identity_remapping_survives_save_and_failed_foreign_links() {
        let mut project = Project::default();
        let mut root = Entity {
            id: "a".into(),
            name: "Actor".into(),
            components: vec![Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            }],
        };
        let prefab = kiln_core::Prefab::new("Actor".into(), root.clone());
        let prefab_id = prefab.id.clone();
        project.prefabs.insert(prefab_id.clone(), prefab);
        root.components.push(Component::PrefabInstance {
            prefab_id,
            overrides: kiln_core::PrefabOverride::new("a".into()),
        });
        let joint=serde_json::from_value(serde_json::json!({"entity_a":"a","entity_b":"a","joint_type":"Distance","anchor_a":[0,0],"anchor_b":[0,0],"reference_angle":0,"length":1,"lower_translation":0,"upper_translation":0,"enable_limit":false,"motor_speed":0,"max_motor_torque":0,"enable_motor":false})).unwrap();
        root.components.push(Component::Joint { joint });
        let mut scene = Scene {
            id: "main".into(),
            name: "Main".into(),
            entities: vec![root],
            script: None,
            nested_scenes: vec![],
        };
        project.scenes.insert(scene.id.clone(), scene.clone());
        let text = copy(&project, &scene, &["a".into()]).unwrap();
        let id = EntityCommand {
            scene_id: scene.id.clone(),
            edit: EntityEdit::Paste { document: text },
        }
        .execute(&mut scene, &project, &mut SceneHistory::new())
        .unwrap()
        .unwrap();
        let pasted = &scene.entities[1];
        assert!(pasted.components.iter().any(
            |c| matches!(c,Component::Joint {joint} if joint.entity_a==id && joint.entity_b==id)
        ));
        assert!(pasted.components.iter().any(
            |c| matches!(c,Component::PrefabInstance {overrides,..} if overrides.entity_id==id)
        ));
        project.scenes.insert(scene.id.clone(), scene.clone());
        let reopened = Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&reopened.scenes["main"]).unwrap(),
            serde_json::to_value(&scene).unwrap()
        );
        let mut bad = copy(&project, &scene, std::slice::from_ref(&id)).unwrap();
        bad = bad.replace(
            &format!("\"entity_b\":\"{id}\""),
            "\"entity_b\":\"missing\"",
        );
        let before = serde_json::to_value(&scene).unwrap();
        let mut history = SceneHistory::new();
        assert!(EntityCommand {
            scene_id: scene.id.clone(),
            edit: EntityEdit::Paste { document: bad }
        }
        .execute(&mut scene, &project, &mut history)
        .is_err());
        assert_eq!(serde_json::to_value(&scene).unwrap(), before);
        assert!(!history.can_undo());
    }
}
