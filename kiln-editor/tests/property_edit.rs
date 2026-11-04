use kiln_core::{Component, Entity, Project, Scene};
use kiln_editor::{
    project_files, property_edit::SetSpriteVisual, transform_edit::TransformTransaction,
    SceneHistory,
};
use kiln_runtime::{GameWorld, SourceId, Sprite};

fn fixture() -> Scene {
    Scene {
        id: "main".into(),
        name: "Main".into(),
        script: None,
        nested_scenes: vec![],
        entities: vec![Entity {
            id: "hero".into(),
            name: "Hero".into(),
            components: vec![
                Component::Transform {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
                Component::Sprite {
                    color: [0.2, 0.4, 0.6, 1.0],
                    width: 32.0,
                    height: 32.0,
                    texture: Some("hero.png".into()),
                    layer: 2.0,
                    material: Some("ink".into()),
                    animation_clip: Some("idle".into()),
                },
            ],
        }],
    }
}

fn world_for(scene: &Scene) -> GameWorld {
    let mut project = Project::default();
    project.scenes.insert(scene.id.clone(), scene.clone());
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(
        &mut world,
        &project,
        "main",
        &mut |_: &Entity, c, _: Option<&str>| Sprite::new(c[0], c[1], c[2], c[3]),
    );
    world
}

fn runtime_sprite(world: &GameWorld) -> Sprite {
    let entity = world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, id)| id.0 == "hero")
        .unwrap()
        .0;
    (*world.world_ref().get::<&Sprite>(entity).unwrap()).clone()
}

fn edit(
    scene: &mut Scene,
    world: &mut GameWorld,
    color: [f32; 4],
    layer: f32,
    size: [f32; 2],
) -> Result<bool, String> {
    SetSpriteVisual {
        scene_id: "main",
        entity_id: "hero",
        color,
        layer,
        size,
    }
    .apply(scene, world)
}

#[test]
fn sprite_visual_properties_update_document_and_runtime_without_losing_metadata() {
    let mut scene = fixture();
    let mut world = world_for(&scene);
    assert!(edit(
        &mut scene,
        &mut world,
        [0.8, 0.3, 0.1, 0.7],
        9.0,
        [48.0, 24.0]
    )
    .unwrap());
    let Component::Sprite {
        color,
        layer,
        width,
        height,
        texture,
        material,
        animation_clip,
    } = &scene.entities[0].components[1]
    else {
        panic!("sprite")
    };
    assert_eq!(*color, [0.8, 0.3, 0.1, 0.7]);
    assert_eq!(*layer, 9.0);
    assert_eq!([*width, *height], [48.0, 24.0]);
    assert_eq!(texture.as_deref(), Some("hero.png"));
    assert_eq!(material.as_deref(), Some("ink"));
    assert_eq!(animation_clip.as_deref(), Some("idle"));
    let expected_color = *color;
    let expected_layer = *layer;
    let runtime = runtime_sprite(&world);
    assert_eq!(runtime.color, expected_color);
    assert_eq!(runtime.layer, expected_layer);
    assert_eq!(runtime.size, [48.0, 24.0]);
    assert!(!edit(
        &mut scene,
        &mut world,
        expected_color,
        expected_layer,
        [48.0, 24.0]
    )
    .unwrap());
}

#[test]
fn repeated_sprite_previews_commit_as_one_undo_step() {
    let mut scene = fixture();
    let before = scene.clone();
    let mut world = world_for(&scene);
    let mut transaction = TransformTransaction::new(before.clone(), "hero".into());
    for (value, layer, size) in [
        ([0.4, 0.4, 0.6, 1.0], 2.0, [40.0, 36.0]),
        ([0.8, 0.4, 0.6, 1.0], 4.0, [48.0, 32.0]),
        ([0.8, 0.3, 0.6, 1.0], 4.0, [64.0, 48.0]),
    ] {
        edit(&mut scene, &mut world, value, layer, size).unwrap();
        transaction.update(scene.clone());
    }
    let mut history = SceneHistory::new();
    transaction.commit(&mut history);
    assert_eq!(history.undo_depth(), 1);
    let mut project = Project::default();
    project.scenes.insert("main".into(), scene.clone());
    let directory = tempfile::tempdir().unwrap();
    project_files::save(&project, &directory.path().join("kiln.toml")).unwrap();
    let reopened = project_files::load(directory.path()).unwrap().project;
    assert_eq!(
        serde_json::to_value(&reopened.scenes["main"]).unwrap(),
        serde_json::to_value(&scene).unwrap()
    );
    let undone = history.undo(scene).unwrap();
    assert_eq!(
        serde_json::to_value(undone).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert!(history.can_redo());
}

#[test]
fn invalid_sprite_visual_values_do_not_change_authored_or_runtime_state() {
    let mut scene = fixture();
    let before = serde_json::to_value(&scene).unwrap();
    let mut world = world_for(&scene);
    let runtime_before = runtime_sprite(&world);
    for (color, layer, size) in [
        ([1.2, 0.0, 0.0, 1.0], 0.0, [32.0, 32.0]),
        ([f32::NAN, 0.0, 0.0, 1.0], 0.0, [32.0, 32.0]),
        ([0.0, 0.0, 0.0, 1.0], f32::INFINITY, [32.0, 32.0]),
        ([0.0, 0.0, 0.0, 1.0], 0.0, [0.0, 32.0]),
        ([0.0, 0.0, 0.0, 1.0], 0.0, [f32::NAN, 32.0]),
    ] {
        assert!(edit(&mut scene, &mut world, color, layer, size).is_err());
        assert_eq!(serde_json::to_value(&scene).unwrap(), before);
        let runtime = runtime_sprite(&world);
        assert_eq!(runtime.color, runtime_before.color);
        assert_eq!(runtime.layer, runtime_before.layer);
    }
}
