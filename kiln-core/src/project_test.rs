#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_roundtrip() {
        let mut project = Project::default();
        project.name = "Test Project".to_string();

        let toml_str = project.to_toml().unwrap();
        let parsed = Project::from_toml(&toml_str).unwrap();

        assert_eq!(project.id, parsed.id);
        assert_eq!(project.name, parsed.name);
    }

    #[test]
    fn test_scene_serialization() {
        let scene = Scene {
            id: "scene-1".to_string(),
            name: "Main Scene".to_string(),
            entities: vec![Entity {
                id: "entity-1".to_string(),
                name: "Player".to_string(),
                components: vec![Component::Transform {
                    x: 100.0,
                    y: 200.0,
                    rotation: 0.0,
                    scale: 1.0,
                }],
            }],
        };

        let toml_str = toml::to_string_pretty(&scene).unwrap();
        let parsed: Scene = toml::from_str(&toml_str).unwrap();

        assert_eq!(scene.id, parsed.id);
        assert_eq!(scene.entities.len(), parsed.entities.len());
    }

    #[test]
    fn test_component_types() {
        let transform = Component::Transform {
            x: 10.0,
            y: 20.0,
            rotation: 45.0,
            scale: 1.5,
        };

        let sprite = Component::Sprite {
            color: [1.0, 0.0, 0.0, 1.0],
            texture: None,
        };

        let toml_str = toml::to_string_pretty(&transform).unwrap();
        let parsed: Component = toml::from_str(&toml_str).unwrap();

        match parsed {
            Component::Transform { x, .. } => assert_eq!(x, 10.0),
            _ => panic!("Expected Transform component"),
        }

        // A sprite without an explicit texture still round-trips.
        let sprite_toml = toml::to_string_pretty(&sprite).unwrap();
        let parsed_sprite: Component = toml::from_str(&sprite_toml).unwrap();
        match parsed_sprite {
            Component::Sprite { color, texture } => {
                assert_eq!(color, [1.0, 0.0, 0.0, 1.0]);
                assert!(texture.is_none());
            }
            _ => panic!("Expected Sprite component"),
        }
    }

    #[test]
    fn test_sprite_texture_path_round_trips() {
        let sprite = Component::Sprite {
            color: [1.0, 1.0, 1.0, 1.0],
            texture: Some("sprites/hero.png".to_string()),
        };

        let toml_str = toml::to_string_pretty(&sprite).unwrap();
        let parsed: Component = toml::from_str(&toml_str).unwrap();

        match parsed {
            Component::Sprite { texture, .. } => {
                assert_eq!(texture.as_deref(), Some("sprites/hero.png"));
            }
            _ => panic!("Expected Sprite component"),
        }
    }
}
