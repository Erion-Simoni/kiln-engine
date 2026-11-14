//! Typed resource dependencies and reference-safe path rewriting.
use crate::{Component, ComponentOverride, Entity, MaterialParam, Project};
use std::collections::BTreeMap;

fn entity_paths(entity: &mut Entity, owner: &str, visit: &mut impl FnMut(&mut String, &str)) {
    for component in &mut entity.components {
        match component {
            Component::Sprite {
                texture: Some(path),
                ..
            }
            | Component::Script { path, .. } => visit(path, owner),
            Component::PrefabInstance { overrides, .. } => {
                for value in overrides.overrides.values_mut() {
                    if let ComponentOverride::Sprite {
                        texture: Some(Some(path)),
                        ..
                    } = value
                    {
                        visit(path, owner);
                    }
                }
            }
            _ => {}
        }
    }
}
impl Project {
    fn visit_resource_paths(&mut self, mut visit: impl FnMut(&mut String, &str)) {
        for (scene_id, scene) in &mut self.scenes {
            if let Some(path) = &mut scene.script {
                visit(path, &format!("scene:{scene_id}"));
            }
            for entity in &mut scene.entities {
                entity_paths(
                    entity,
                    &format!("scene:{scene_id}/entity:{}", entity.id),
                    &mut visit,
                );
            }
        }
        for (id, prefab) in &mut self.prefabs {
            entity_paths(&mut prefab.root_entity, &format!("prefab:{id}"), &mut visit);
        }
        for (id, clip) in &mut self.animation_clips {
            visit(&mut clip.texture, &format!("clip:{id}"));
        }
        for (id, tileset) in &mut self.tilesets {
            visit(&mut tileset.texture, &format!("tileset:{id}"));
        }
        for (id, font) in &mut self.ui.fonts {
            visit(&mut font.path, &format!("ui-font:{id}"));
        }
        for (id, font) in &mut self.sdf_fonts {
            visit(&mut font.texture_path, &format!("sdf-font:{id}"));
        }
        for audio in &mut self.audio_assets {
            visit(&mut audio.path, &format!("audio:{}", audio.name));
        }
        for (id, shader) in &mut self.shaders {
            for (key, value) in &mut shader.parameters {
                if let MaterialParam::Texture(path) = value {
                    visit(path, &format!("shader:{id}/{key}"));
                }
            }
        }
        for (id, material) in &mut self.materials {
            for (key, value) in &mut material.parameters {
                if let MaterialParam::Texture(path) = value {
                    visit(path, &format!("material:{id}/{key}"));
                }
            }
        }
    }

    /// Resource path to sorted unique authored dependents. Does not modify this project.
    pub fn resource_dependencies(&self) -> BTreeMap<String, Vec<String>> {
        let mut snapshot = self.clone();
        let mut result = BTreeMap::<String, Vec<String>>::new();
        snapshot.visit_resource_paths(|path, owner| {
            if !path.is_empty() {
                result.entry(path.clone()).or_default().push(owner.into());
            }
        });
        for owners in result.values_mut() {
            owners.sort();
            owners.dedup();
        }
        result
    }

    /// Rewrite supported path references and move the asset's stable identity.
    /// The caller owns filesystem move/transaction and script-import source edits.
    pub fn rewrite_resource_path(&mut self, old: &str, new: &str) -> usize {
        if old.is_empty() || old == new {
            return 0;
        }
        let mut count = 0;
        self.visit_resource_paths(|path, _| {
            if path == old {
                *path = new.into();
                count += 1;
            }
        });
        if let Some(id) = self.asset_ids.remove(old) {
            self.asset_ids.insert(new.into(), id);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_overrides_fall_back_preserve_ids_and_survive_roundtrip() {
        let mut project = Project::default();
        project
            .asset_ids
            .insert("art/hero.png".into(), "stable".into());
        project.asset_import_settings.insert(
            "stable".into(),
            crate::project::AssetImportSettings {
                nearest_filter: true,
                generate_mipmaps: false,
                is_sprite: true,
            },
        );
        project.asset_platform_import_settings.insert(
            "stable".into(),
            std::collections::HashMap::from([(
                "web".into(),
                crate::project::AssetImportSettings {
                    nearest_filter: false,
                    generate_mipmaps: true,
                    is_sprite: true,
                },
            )]),
        );
        assert_eq!(
            project.texture_options_for_target("art/hero.png", "web"),
            [false, true]
        );
        assert_eq!(
            project.texture_options_for_target("art/hero.png", "windows"),
            [true, false]
        );
        project.rewrite_resource_path("art/hero.png", "art/renamed.png");
        assert_eq!(project.asset_ids["art/renamed.png"], "stable");
        let reopened = Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert_eq!(
            reopened.texture_options_for_target("art/renamed.png", "web"),
            [false, true]
        );
        assert!(!Project::default()
            .to_toml()
            .unwrap()
            .contains("asset_platform_import_settings"));
    }

    #[test]
    fn resource_moves_include_clips_tilesets_materials_scripts_and_prefab_overrides() {
        let old = "art/old.png";
        let new = "art/new.png";
        let mut project = Project::default();
        let mut override_value = crate::PrefabOverride::new("actor".into());
        override_value.overrides.insert(
            "sprite".into(),
            ComponentOverride::Sprite {
                color: None,
                texture: Some(Some(old.into())),
            },
        );
        let entity = Entity {
            id: "actor".into(),
            name: old.into(),
            components: vec![
                Component::Sprite {
                    color: [1.0; 4],
                    width: 16.0,
                    height: 16.0,
                    texture: Some(old.into()),
                    layer: 0.0,
                    material: None,
                    animation_clip: None,
                },
                Component::PrefabInstance {
                    prefab_id: "prefab".into(),
                    overrides: override_value,
                },
                Component::Script {
                    path: old.into(),
                    properties: Default::default(),
                },
            ],
        };
        project.scenes.insert(
            "main".into(),
            crate::Scene {
                id: "main".into(),
                name: old.into(),
                script: Some(old.into()),
                entities: vec![entity.clone()],
                nested_scenes: vec![],
            },
        );
        project
            .prefabs
            .insert("prefab".into(), crate::Prefab::new("Actor".into(), entity));
        project
            .animation_clips
            .insert("walk".into(), crate::Clip::new("walk".into(), old.into()));
        project.tilesets.insert(
            "tiles".into(),
            crate::TileSet {
                texture: old.into(),
                ..Default::default()
            },
        );
        project.materials.insert(
            "material".into(),
            crate::SpriteMaterial {
                parameters: std::collections::HashMap::from([(
                    "albedo".into(),
                    MaterialParam::Texture(old.into()),
                )]),
                ..Default::default()
            },
        );
        project.asset_ids.insert(old.into(), "stable".into());
        let before = project.resource_dependencies();
        assert_eq!(before[old].len(), 6);
        assert_eq!(project.rewrite_resource_path(old, new), 10);
        assert!(!project.resource_dependencies().contains_key(old));
        assert_eq!(project.asset_ids[new], "stable");
        assert_eq!(project.scenes["main"].entities[0].name, old);
        assert_eq!(project.scenes["main"].name, old);
        let reopened = Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert_eq!(
            reopened.resource_dependencies(),
            project.resource_dependencies()
        );
    }
}
