//! Explicit transfer of supported runtime changes to authoring documents.

use kiln_core::{Component, Scene};

/// Copy local transforms for existing authored entities. Runtime spawns,
/// deletions, and resource/property mutations are intentionally not applied.
pub fn apply_transforms(authored: &Scene, runtime: &Scene) -> Result<Scene, String> {
    if authored.id != runtime.id {
        return Err("Runtime changes target another scene".into());
    }
    let mut result = authored.clone();
    for entity in &mut result.entities {
        let matches: Vec<_> = runtime
            .entities
            .iter()
            .filter(|candidate| candidate.id == entity.id)
            .collect();
        let [runtime_entity] = matches.as_slice() else {
            continue;
        };
        let transforms: Vec<_> = runtime_entity
            .components
            .iter()
            .filter(|component| matches!(component, Component::Transform { .. }))
            .collect();
        if let [transform] = transforms.as_slice() {
            if let Component::Transform {
                x,
                y,
                rotation,
                scale,
            } = transform
            {
                if ![x, y, rotation, scale]
                    .iter()
                    .all(|value| value.is_finite())
                    || *scale <= 0.0
                {
                    return Err(format!("Runtime transform for '{}' is invalid", entity.id));
                }
            }
            if let Some(component) = entity
                .components
                .iter_mut()
                .find(|component| matches!(component, Component::Transform { .. }))
            {
                *component = (*transform).clone();
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_transfer_keeps_authored_identity_resources_and_entity_set() {
        let mut project = kiln_core::Project::default();
        let id = crate::editing::create_scene(&mut project, "Test");
        let mut authored = project.scenes[&id].clone();
        authored.entities.push(kiln_core::Entity {
            id: "enemy".into(),
            name: "Enemy".into(),
            components: vec![
                Component::Transform {
                    x: 1.0,
                    y: 2.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Role {
                    role: "enemy".into(),
                },
            ],
        });
        let mut runtime = authored.clone();
        runtime.entities[0].components[0] = Component::Transform {
            x: 7.0,
            y: 9.0,
            rotation: 0.0,
            scale: 1.0,
        };
        runtime.entities[0].name = "Runtime name".into();
        runtime.entities.push(kiln_core::Entity {
            id: "spawn".into(),
            name: "Spawn".into(),
            components: vec![],
        });
        let applied = apply_transforms(&authored, &runtime).unwrap();
        assert_eq!(applied.entities.len(), 1);
        assert_eq!(applied.entities[0].name, "Enemy");
        assert!(matches!(
            applied.entities[0].components[0],
            Component::Transform { x: 7.0, y: 9.0, .. }
        ));
        assert!(
            matches!(&applied.entities[0].components[1], Component::Role { role } if role == "enemy")
        );
    }
}
