//! Saved editor-only locks/visibility and atomic world-position alignment.
use kiln_core::{Component, Scene};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Flags {
    pub locked: bool,
    pub hidden: bool,
}

pub(crate) fn own_flags(scene: &Scene, id: &str) -> Flags {
    scene
        .entities
        .iter()
        .find(|e| e.id == id)
        .map(entity_flags)
        .unwrap_or_default()
}
fn entity_flags(entity: &kiln_core::Entity) -> Flags {
    let mut flags = Flags::default();
    for component in &entity.components {
        if let Component::EditorFlags { locked, hidden } = component {
            flags.locked |= locked;
            flags.hidden |= hidden;
        }
    }
    flags
}

/// Resolve inherited flags once for the scene, tolerating malformed cycles conservatively.
pub(crate) fn resolved_flags(scene: &Scene) -> HashMap<String, Flags> {
    let own: HashMap<_, _> = scene
        .entities
        .iter()
        .map(|e| (e.id.as_str(), entity_flags(e)))
        .collect();
    let parents: HashMap<_, _> = scene
        .entities
        .iter()
        .filter_map(|e| {
            e.components.iter().find_map(|c| match c {
                Component::Parent { parent } => Some((e.id.as_str(), parent.as_str())),
                _ => None,
            })
        })
        .collect();
    let mut resolved = HashMap::<String, Flags>::new();
    for entity in &scene.entities {
        let mut current = entity.id.as_str();
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut flags = loop {
            if let Some(flags) = resolved.get(current) {
                break *flags;
            }
            if !seen.insert(current) {
                break Flags {
                    locked: true,
                    hidden: true,
                };
            }
            let Some(flags) = own.get(current) else {
                break Flags::default();
            };
            chain.push((current, *flags));
            match parents.get(current) {
                Some(parent) => current = parent,
                None => break Flags::default(),
            }
        };
        for (id, value) in chain.into_iter().rev() {
            flags.locked |= value.locked;
            flags.hidden |= value.hidden;
            resolved.insert(id.to_owned(), flags);
        }
    }
    resolved
}

pub(crate) fn require_editable(scene: &Scene, id: &str) -> Result<(), String> {
    match resolved_flags(scene).get(id) {
        Some(flags) if flags.locked => Err(format!(
            "Entity `{id}` is locked in the editor; unlock it in the hierarchy"
        )),
        Some(_) => Ok(()),
        None => Err(format!("Entity `{id}` is missing")),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Alignment of independent entity world positions, without changing their dimensions.
pub enum Alignment {
    Left,
    HorizontalCenter,
    Right,
    Bottom,
    VerticalCenter,
    Top,
}
impl Alignment {
    pub(crate) const ALL: [(Self, &'static str); 6] = [
        (Self::Left, "Leftmost X"),
        (Self::HorizontalCenter, "Center X"),
        (Self::Right, "Rightmost X"),
        (Self::Bottom, "Bottom Y"),
        (Self::VerticalCenter, "Center Y"),
        (Self::Top, "Top Y"),
    ];
}

pub(crate) fn align(scene: &mut Scene, ids: &[String], mode: Alignment) -> Result<(), String> {
    let selected: HashSet<_> = ids.iter().map(String::as_str).collect();
    let parents: HashMap<_, _> = scene
        .entities
        .iter()
        .filter_map(|e| {
            e.components.iter().find_map(|c| match c {
                Component::Parent { parent } => Some((e.id.as_str(), parent.as_str())),
                _ => None,
            })
        })
        .collect();
    let world = kiln_core::hierarchy_transform::world_transforms(scene)?;
    let mut roots = Vec::new();
    for id in &selected {
        require_editable(scene, id)?;
        let mut current = *id;
        let mut ancestor_selected = false;
        while let Some(parent) = parents.get(current) {
            if selected.contains(parent) {
                ancestor_selected = true;
                break;
            }
            current = parent;
        }
        if !ancestor_selected {
            roots.push((*id).to_owned());
        }
    }
    if roots.len() < 2 {
        return Err(
            "Select at least two independent entities to align their world positions".into(),
        );
    }
    roots.sort();
    let axis = if matches!(
        mode,
        Alignment::Left | Alignment::HorizontalCenter | Alignment::Right
    ) {
        0
    } else {
        1
    };
    let position = |id: &str| {
        let p = world[id].position;
        if axis == 0 {
            p.x
        } else {
            p.y
        }
    };
    let low = roots
        .iter()
        .map(|id| position(id))
        .fold(f32::INFINITY, f32::min);
    let high = roots
        .iter()
        .map(|id| position(id))
        .fold(f32::NEG_INFINITY, f32::max);
    let target = match mode {
        Alignment::Left | Alignment::Bottom => low,
        Alignment::Right | Alignment::Top => high,
        _ => low / 2.0 + high / 2.0,
    };
    for id in roots {
        let mut pose = world[&id];
        if axis == 0 {
            pose.position.x = target;
        } else {
            pose.position.y = target;
        }
        let local = kiln_core::hierarchy_transform::local_from_world(scene, &id, pose)?;
        let entity = scene
            .entities
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("Missing aligned entity")?;
        crate::entity_command::set_local_transform(entity, local)?;
    }
    Ok(())
}

/// Definition propagation must not erase an instance's editor workspace flags.
pub(crate) fn preserve_flags(from: &kiln_core::Entity, to: &mut kiln_core::Entity) {
    to.components
        .retain(|c| !matches!(c, Component::EditorFlags { .. }));
    to.components.extend(
        from.components
            .iter()
            .filter(|c| matches!(c, Component::EditorFlags { .. }))
            .cloned(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity_command::{EntityCommand, EntityEdit},
        history::SceneHistory,
    };
    fn entity(id: &str, x: f32, parent: Option<&str>) -> kiln_core::Entity {
        let mut components = vec![Component::Transform {
            x,
            y: 10.0,
            rotation: 0.0,
            scale: 1.0,
        }];
        if let Some(parent) = parent {
            components.push(Component::Parent {
                parent: parent.into(),
            });
        }
        kiln_core::Entity {
            id: id.into(),
            name: id.into(),
            components,
        }
    }
    fn fixture() -> (kiln_core::Project, Scene) {
        let scene = Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                entity("a", 20.0, None),
                entity("b", 80.0, None),
                entity("child", 5.0, Some("a")),
            ],
        };
        let mut project = kiln_core::Project::default();
        project.scenes.insert(scene.id.clone(), scene.clone());
        (project, scene)
    }
    fn command(edit: EntityEdit) -> EntityCommand {
        EntityCommand {
            scene_id: "main".into(),
            edit,
        }
    }
    #[test]
    fn inherited_locks_reject_direct_edits_and_unlock_is_undoable() {
        let (mut project, mut scene) = fixture();
        let mut history = SceneHistory::new();
        command(EntityEdit::SetEditorFlags {
            entity_id: "a".into(),
            locked: true,
            hidden: true,
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        let flags = resolved_flags(&scene);
        assert_eq!(
            flags["child"],
            Flags {
                locked: true,
                hidden: true
            }
        );
        assert_eq!(flags["b"], Flags::default());
        let before = serde_json::to_value(&scene).unwrap();
        for edit in [
            EntityEdit::Rename {
                entity_id: "child".into(),
                name: "bad".into(),
            },
            EntityEdit::Delete {
                entity_id: "a".into(),
            },
            EntityEdit::Reparent {
                entity_id: "child".into(),
                parent_id: None,
            },
            EntityEdit::Align {
                entity_ids: vec!["a".into(), "b".into()],
                alignment: Alignment::Left,
            },
        ] {
            assert!(command(edit)
                .execute(&mut scene, &project, &mut history)
                .unwrap_err()
                .contains("locked"));
            assert_eq!(serde_json::to_value(&scene).unwrap(), before);
        }
        project.scenes.insert("main".into(), scene.clone());
        let reopened = kiln_core::Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert!(resolved_flags(&reopened.scenes["main"])["child"].hidden);
        command(EntityEdit::SetEditorFlags {
            entity_id: "a".into(),
            locked: false,
            hidden: false,
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        require_editable(&scene, "child").unwrap();
        let undone = history.undo(scene).unwrap();
        assert!(resolved_flags(&undone)["child"].locked);
    }
    #[test]
    fn prefab_propagation_and_revert_preserve_instance_editor_flags() {
        let (mut project, mut scene) = fixture();
        let prefab = kiln_core::Prefab::new("Actor".into(), scene.entities[0].clone());
        let prefab_id = prefab.id.clone();
        project.prefabs.insert(prefab_id.clone(), prefab.clone());
        let mut history = SceneHistory::new();
        let instance = command(EntityEdit::InstantiatePrefab {
            prefab_id: prefab_id.clone(),
            position: [200.0, 50.0],
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap()
        .unwrap();
        command(EntityEdit::SetEditorFlags {
            entity_id: instance.clone(),
            locked: false,
            hidden: true,
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        project.scenes.insert("main".into(), scene.clone());
        crate::editing::propagate_prefab_in_scenes(&mut project, &prefab);
        scene = project.scenes["main"].clone();
        assert!(own_flags(&scene, &instance).hidden);
        command(EntityEdit::RevertPrefabOverrides {
            entity_id: instance.clone(),
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        assert!(own_flags(&scene, &instance).hidden);
    }

    #[test]
    fn alignment_is_one_history_step_and_selected_children_do_not_move_twice() {
        let (project, mut scene) = fixture();
        let before = serde_json::to_value(&scene).unwrap();
        let mut history = SceneHistory::new();
        command(EntityEdit::Align {
            entity_ids: vec!["a".into(), "b".into(), "child".into()],
            alignment: Alignment::HorizontalCenter,
        })
        .execute(&mut scene, &project, &mut history)
        .unwrap();
        let world = kiln_core::hierarchy_transform::world_transforms(&scene).unwrap();
        assert_eq!(world["a"].position.x, 50.0);
        assert_eq!(world["b"].position.x, 50.0);
        assert_eq!(world["child"].position.x, 55.0);
        let undone = history.undo(scene.clone()).unwrap();
        assert_eq!(serde_json::to_value(&undone).unwrap(), before);
        assert!(!history.can_undo());
        assert_eq!(
            serde_json::to_value(history.redo(undone).unwrap()).unwrap(),
            serde_json::to_value(scene).unwrap()
        );
    }
    #[test]
    fn editor_visibility_does_not_change_player_draws_or_disappear_on_snapshot() {
        let (mut project, mut scene) = fixture();
        scene.entities[0].components.push(Component::EditorFlags {
            locked: false,
            hidden: true,
        });
        scene.entities[0].components.push(Component::Sprite {
            color: [1.0; 4],
            width: 20.0,
            height: 20.0,
            texture: None,
            layer: 0.0,
            material: None,
            animation_clip: None,
        });
        project.scenes.insert("main".into(), scene.clone());
        let mut world = kiln_runtime::GameWorld::new();
        kiln_runtime::scene::spawn_scene(&mut world, &project, "main", &mut |_, _, _| {
            kiln_runtime::Sprite::new(1.0, 1.0, 1.0, 1.0)
        });
        assert_eq!(
            kiln_runtime::visual::collect_scene_draws(&world, [0.0, 0.0, 100.0, 100.0]).len(),
            1
        );
        let flags = resolved_flags(&scene);
        assert!(kiln_runtime::visual::collect_scene_draws_filtered(
            &world,
            [0.0, 0.0, 100.0, 100.0],
            |_| None,
            |id| !id.and_then(|id| flags.get(id)).is_some_and(|f| f.hidden)
        )
        .is_empty());
        let saved = crate::scene_document::snapshot_for_scene(
            &world,
            &Default::default(),
            &Default::default(),
            Some(&project),
            Some("main"),
        );
        assert!(own_flags(&saved, "a").hidden);
    }
}
