//! Geometry and preview/gameplay parity at the shared runtime boundary.
use kiln_core::{Component, Entity, Project, Scene, Transform, Vec2};
use kiln_runtime::{GameWorld, GlobalTransform, ParticleSystemComponent, SourceId, Sprite};
#[test]
fn pivot_scale_flips_and_rotation_define_actual_draw_bounds() {
    let sprite = Sprite::default()
        .with_size(20.0, 10.0)
        .with_pivot(0.0, 1.0)
        .with_flip_x(true);
    let mut sprite = sprite;
    sprite.pixels_per_unit = 2.0;
    let pose = Transform::new(10.0, 20.0)
        .with_rotation(std::f32::consts::FRAC_PI_2)
        .with_scale(Vec2::new(-2.0, 3.0));
    let draw = sprite.draw_instance(pose);
    assert!((draw.position[0] - 2.5).abs() < 0.001 && (draw.position[1] - 10.0).abs() < 0.001);
    assert_eq!(draw.uv_rect, [1.0, 0.0, 0.0, 1.0]);
    let bounds = sprite.world_bounds(pose);
    for (value, expected) in bounds.into_iter().zip([-5.0, 0.0, 10.0, 20.0]) {
        assert!((value - expected).abs() < 0.001);
    }
}
#[test]
fn saved_effect_preview_matches_gameplay_randomness_gravity_fading_and_pause() {
    let effect = kiln_core::visual::ParticleEffect::default();
    let mut preview = ParticleSystemComponent::from_effect(&effect).unwrap();
    let mut world = GameWorld::new();
    let entity = world.world_mut().spawn((
        ParticleSystemComponent::from_effect(&effect).unwrap(),
        GlobalTransform(Transform::default()),
    ));
    for _ in 0..90 {
        preview.system_mut().update(1.0 / 60.0);
        kiln_runtime::sprite::update_particle_systems(world.world_mut(), 1.0 / 60.0);
    }
    let runtime = world
        .world_ref()
        .get::<&ParticleSystemComponent>(entity)
        .unwrap();
    let snapshot = |effect: &ParticleSystemComponent| {
        effect
            .system()
            .particles()
            .iter()
            .map(|p| (p.position, p.velocity, p.color, p.age, p.lifetime, p.size))
            .collect::<Vec<_>>()
    };
    assert!(!runtime.system().particles().is_empty());
    assert_eq!(snapshot(&preview), snapshot(&runtime));
    let before = runtime.particle_count();
    drop(runtime);
    world
        .world_mut()
        .get::<&mut ParticleSystemComponent>(entity)
        .unwrap()
        .system_mut()
        .emitting = false;
    kiln_runtime::sprite::update_particle_systems(world.world_mut(), 1.0 / 60.0);
    assert!(
        world
            .world_ref()
            .get::<&ParticleSystemComponent>(entity)
            .unwrap()
            .particle_count()
            <= before
    );
}
#[test]
fn tilemap_attachment_preserves_local_parent_and_nested_offsets() {
    let mut p = Project::default();
    let mut map = kiln_core::TileMap {
        id: "tiles".into(),
        ..Default::default()
    };
    let mut layer = kiln_core::TileMapLayer {
        tileset: "set".into(),
        offset: [300.0, 0.0],
        ..Default::default()
    };
    layer.set_tile_at(0, 0, 0);
    map.layers.push(layer);
    p.tilemaps.insert("tiles".into(), map);
    p.tilesets.insert(
        "set".into(),
        kiln_core::TileSet {
            solid: vec![true],
            ..Default::default()
        },
    );
    let transform = |x| Component::Transform {
        x,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
    };
    let nested = Scene {
        id: "nested".into(),
        name: "Nested".into(),
        script: None,
        nested_scenes: vec![],
        entities: vec![
            Entity {
                id: "parent".into(),
                name: "Parent".into(),
                components: vec![transform(10.0)],
            },
            Entity {
                id: "map".into(),
                name: "Map".into(),
                components: vec![
                    transform(5.0),
                    Component::Parent {
                        parent: "parent".into(),
                    },
                    Component::Tilemap {
                        map: "tiles".into(),
                    },
                ],
            },
        ],
    };
    let main = Scene {
        id: "main".into(),
        name: "Main".into(),
        script: None,
        entities: vec![],
        nested_scenes: vec![kiln_core::NestedScene {
            scene_id: "nested".into(),
            additive: true,
            offset: kiln_core::SceneOffset { x: 100.0, y: 0.0 },
        }],
    };
    p.scenes.insert("main".into(), main);
    p.scenes.insert("nested".into(), nested.clone());
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &p, "main", &mut |_, _, _| Sprite::default());
    kiln_runtime::spawn_tilemaps(
        &mut world,
        &p,
        &nested,
        kiln_core::SceneOffset { x: 100.0, y: 0.0 },
        &mut |_| Some(kiln_render::TextureId(1)),
    );
    kiln_runtime::update_global_transforms(&mut world).unwrap();
    let mut query = world
        .world_ref()
        .query::<(&SourceId, &Transform, &kiln_runtime::Tilemap)>();
    let (_, local, map) = query.iter().next().unwrap();
    assert_eq!(local.position.x, 5.0);
    assert_eq!(map.offset, [115.0, 0.0]);
    assert_eq!(map.world_bounds(), [415.0, 0.0, 671.0, 256.0]);
    assert_eq!(
        map.first_solid_hit([415.0, 0.0, 416.0, 1.0]),
        Some([415.0, 0.0])
    );
    assert_eq!(map.collect_visible([415.0, 0.0, 431.0, 16.0]).len(), 1);
}

#[test]
fn same_layer_painter_order_follows_authored_hierarchy_across_archetypes() {
    let mut p = Project::default();
    let sprite = |color| Component::Sprite {
        color,
        width: 16.0,
        height: 16.0,
        texture: None,
        layer: 0.0,
        material: None,
        animation_clip: None,
    };
    p.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![
                Entity {
                    id: "z_background".into(),
                    name: "Background".into(),
                    components: vec![sprite([1.0, 0.0, 0.0, 1.0])],
                },
                Entity {
                    id: "a_foreground".into(),
                    name: "Foreground".into(),
                    components: vec![
                        sprite([0.0, 1.0, 0.0, 1.0]),
                        Component::Role {
                            role: "decoration".into(),
                        },
                    ],
                },
            ],
        },
    );
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &p, "main", &mut |_, color, _| {
        Sprite::new(color[0], color[1], color[2], color[3])
    });
    let colors: Vec<_> =
        kiln_runtime::visual::collect_scene_draws(&world, [-10.0, -10.0, 10.0, 10.0])
            .iter()
            .map(|(_, draw)| draw.color)
            .collect();
    assert_eq!(colors, [[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]);
}
