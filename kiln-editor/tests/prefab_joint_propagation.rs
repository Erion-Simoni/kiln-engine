//! Prefab Apply/propagation must preserve owner identity and reject broken links atomically.
use kiln_core::{physics_data::Joint2D, Component, Entity, Prefab, PrefabOverride, Project, Scene};
use kiln_editor::editing::{apply_prefab_instance_to_definition, propagate_prefab_in_scenes};

fn joint(owner: &str, target: &str) -> Component {
    Component::Joint {
        joint: Joint2D {
            entity_a: owner.into(),
            entity_b: target.into(),
            joint_type: kiln_core::physics_data::JointType::Distance,
            anchor_a: [0.0; 2],
            anchor_b: [0.0; 2],
            reference_angle: 0.0,
            length: 5.0,
            lower_translation: 0.0,
            upper_translation: 0.0,
            enable_limit: false,
            motor_speed: 0.0,
            max_motor_torque: 0.0,
            enable_motor: false,
        },
    }
}
fn fixture() -> Project {
    let definition = Prefab {
        id: "rig".into(),
        name: "Rig".into(),
        root_entity: Entity {
            id: "template".into(),
            name: "Rig".into(),
            components: vec![],
        },
    };
    let mut project = Project::default();
    project.prefabs.insert("rig".into(), definition);
    for (scene_id, id) in [("main", "rig-a"), ("other", "rig-b")] {
        project.scenes.insert(
            scene_id.into(),
            Scene {
                id: scene_id.into(),
                name: scene_id.into(),
                script: None,
                nested_scenes: vec![],
                entities: vec![
                    Entity {
                        id: id.into(),
                        name: id.into(),
                        components: vec![Component::PrefabInstance {
                            prefab_id: "rig".into(),
                            overrides: PrefabOverride::new(id.into()),
                        }],
                    },
                    Entity {
                        id: "anchor".into(),
                        name: "Anchor".into(),
                        components: vec![],
                    },
                ],
            },
        );
    }
    project
}
fn endpoints(entity: &Entity) -> (&str, &str) {
    entity
        .components
        .iter()
        .find_map(|component| match component {
            Component::Joint { joint } => Some((joint.entity_a.as_str(), joint.entity_b.as_str())),
            _ => None,
        })
        .unwrap()
}
#[test]
fn apply_remaps_instance_owner_to_template_and_each_destination_owner() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(joint("rig-a", "anchor"));
    apply_prefab_instance_to_definition(&mut project, "main", "rig-a").unwrap();
    assert_eq!(
        endpoints(&project.prefabs["rig"].root_entity),
        ("template", "anchor")
    );
    assert_eq!(
        endpoints(&project.scenes["main"].entities[0]),
        ("rig-a", "anchor")
    );
    assert_eq!(
        endpoints(&project.scenes["other"].entities[0]),
        ("rig-b", "anchor")
    );
}
#[test]
fn apply_refuses_missing_destination_endpoint_without_changing_any_document() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(joint("rig-a", "anchor"));
    project.scenes.get_mut("other").unwrap().entities.pop();
    let before = serde_json::to_value(&project).unwrap();
    assert!(apply_prefab_instance_to_definition(&mut project, "main", "rig-a").is_err());
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}
#[test]
fn propagation_remaps_template_owner_in_every_scene() {
    let mut project = fixture();
    let mut updated = project.prefabs["rig"].clone();
    updated
        .root_entity
        .components
        .push(joint("template", "anchor"));
    assert_eq!(
        propagate_prefab_in_scenes(&mut project, &updated).unwrap(),
        2
    );
    assert_eq!(
        endpoints(&project.scenes["main"].entities[0]),
        ("rig-a", "anchor")
    );
    assert_eq!(
        endpoints(&project.scenes["other"].entities[0]),
        ("rig-b", "anchor")
    );
}

#[test]
fn propagation_refuses_missing_or_ambiguous_endpoints_atomically() {
    for ambiguous in [false, true] {
        let mut project = fixture();
        if ambiguous {
            project
                .scenes
                .get_mut("other")
                .unwrap()
                .entities
                .push(Entity {
                    id: "anchor".into(),
                    name: "Duplicate".into(),
                    components: vec![],
                });
        } else {
            project.scenes.get_mut("other").unwrap().entities.pop();
        }
        let mut updated = project.prefabs["rig"].clone();
        updated
            .root_entity
            .components
            .push(joint("template", "anchor"));
        let before = serde_json::to_value(&project).unwrap();
        let error = propagate_prefab_in_scenes(&mut project, &updated).unwrap_err();
        assert!(
            error.contains("anchor") && error.contains("other"),
            "{error}"
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
}

#[test]
fn staged_propagation_resolves_unique_nested_endpoints_and_survives_restart() {
    let mut project = fixture();
    let anchor = project
        .scenes
        .get_mut("other")
        .unwrap()
        .entities
        .pop()
        .unwrap();
    project.scenes.insert(
        "anchors".into(),
        Scene {
            id: "anchors".into(),
            name: "Anchors".into(),
            script: None,
            entities: vec![anchor],
            nested_scenes: vec![],
        },
    );
    project
        .scenes
        .get_mut("other")
        .unwrap()
        .nested_scenes
        .push(kiln_core::project::NestedScene {
            scene_id: "anchors".into(),
            additive: true,
            offset: Default::default(),
        });
    project.scenes.get_mut("main").unwrap().entities[0]
        .components
        .push(joint("rig-a", "anchor"));
    apply_prefab_instance_to_definition(&mut project, "main", "rig-a").unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("kiln.toml");
    kiln_editor::project_files::save(&project, &path).unwrap();
    let reopened = kiln_editor::project_files::load(&path).unwrap().project;
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    let mut world = kiln_runtime::GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        &reopened,
        "other",
        &mut |_: &Entity, color: [f32; 4], _: Option<&str>| {
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3])
        },
    );
    let runtime = world
        .world_ref()
        .query::<(&kiln_runtime::SourceId, &Joint2D)>()
        .iter()
        .find(|(source, _)| source.0 == "rig-b")
        .map(|(_, joint)| joint.clone())
        .unwrap();
    assert_eq!(
        (runtime.entity_a.as_str(), runtime.entity_b.as_str()),
        ("rig-b", "anchor")
    );
}

#[test]
fn unchanged_legacy_joint_does_not_block_unrelated_prefab_propagation() {
    let mut project = fixture();
    project.scenes.get_mut("main").unwrap().entities[1]
        .components
        .push(joint("anchor", "legacy-missing"));
    let prefab = project.prefabs["rig"].clone();
    assert_eq!(
        propagate_prefab_in_scenes(&mut project, &prefab).unwrap(),
        2
    );
    assert_eq!(
        endpoints(&project.scenes["main"].entities[1]),
        ("anchor", "legacy-missing")
    );
}
