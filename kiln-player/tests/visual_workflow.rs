//! Editor authoring -> disk -> real KAR archive -> native player loader/runtime.
use kiln_core::{
    Component, Entity, FrameEvent, Project, Scene, TileMap, TileMapLayer, TileSet, Transform,
};
use kiln_editor::{
    component_command::{ComponentCommand, ComponentEdit},
    entity_command::{EntityCommand, EntityEdit},
    project_files, SceneHistory,
};
use kiln_render::TextureId;
use kiln_runtime::{
    AnimationPlayer, GameWorld, ParticleSystemComponent, SliceSession, SourceId, Sprite,
};
use std::{collections::HashMap, path::Path};

fn write_image(root: &Path, path: &str, width: u32, height: u32) {
    let mut pixels = vec![0; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            let color = if path.contains("hero") {
                let px = x % 16;
                if (2..14).contains(&px)
                    && (1..15).contains(&y)
                    && !(y > 11 && (7..9).contains(&px))
                {
                    if y == 4 && (px == 5 || px == 10) {
                        [10, 20, 40, 255]
                    } else {
                        [40 + (x / 16) as u8 * 20, 110, 240, 255]
                    }
                } else {
                    [0, 0, 0, 0]
                }
            } else {
                let tile = (x / 16) + (y / 16) * 4;
                [90 + tile as u8 * 5, 100 + tile as u8 * 4, 30, 255]
            };
            pixels[i..i + 4].copy_from_slice(&color);
        }
    }
    std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    std::fs::write(
        root.join(path),
        kiln_assets::encode_png_rgba(width, height, &pixels).unwrap(),
    )
    .unwrap();
}
fn author(root: &Path) -> Project {
    write_image(root, "art/hero.png", 32, 16);
    write_image(root, "tiles/terrain.png", 64, 64);
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(
        root.join("scripts/main.ks"),
        "move_player(get_input_x() * 60.0 * get_delta_time(), 0.0);",
    )
    .unwrap();
    std::fs::write(
        root.join("scripts/hero.ks"),
        r#"if script_phase() == "animation_event" {
state_set("events", state_get("events", 0) + 1);
state_set("last_event", animation_event_name());
state_set("payload", animation_event_value());
}"#,
    )
    .unwrap();
    let mut p = Project {
        id: "visual-authoring-sample".into(),
        name: "Visual Authoring 2D".into(),
        ..Default::default()
    };
    p.window.width = 320;
    p.window.height = 180;
    let mut scene = Scene {
        id: "main".into(),
        name: "Visual Playground".into(),
        entities: vec![],
        script: Some("scripts/main.ks".into()),
        nested_scenes: vec![],
    };
    p.scenes.insert(scene.id.clone(), scene.clone());
    let mut history = SceneHistory::new();
    let hero = EntityCommand {
        scene_id: "main".into(),
        edit: EntityEdit::CreateSprite {
            texture: "art/hero.png".into(),
            position: [24.0, 40.0],
            size: [32, 16],
        },
    }
    .execute(&mut scene, &p, &mut history)
    .unwrap()
    .unwrap();
    let mut walk = kiln_editor::sprite_editor::grid_clip(
        "walk",
        "art/hero.png",
        [32, 16],
        [16, 16],
        [0, 0],
        [0, 0],
    )
    .unwrap();
    walk.loops = false;
    walk.next = Some("idle".into());
    walk.frames[0].events.push(FrameEvent {
        name: "start".into(),
        payload: None,
    });
    walk.frames[1].events.push(FrameEvent {
        name: "footstep".into(),
        payload: Some("stone".into()),
    });
    p.animation_clips.insert("walk".into(), walk.clone());
    let mut idle = walk.clone();
    idle.name = "idle".into();
    idle.frames.truncate(1);
    idle.frames[0].events.clear();
    idle.loops = true;
    idle.next = None;
    p.animation_clips.insert("idle".into(), idle);
    for edit in [
        ComponentEdit::SetSpriteReferences {
            texture: Some("art/hero.png".into()),
            material: None,
            animation_clip: Some("walk".into()),
        },
        ComponentEdit::Add(Component::Role {
            role: "player".into(),
        }),
        ComponentEdit::Add(Component::Script {
            path: "scripts/hero.ks".into(),
            properties: HashMap::new(),
        }),
        ComponentEdit::SetVisual(Component::Particles {
            effect: kiln_core::visual::ParticleEffect {
                max_particles: 120,
                layer: 3.0,
                emitters: vec![kiln_core::vfx::EmitterConfig {
                    rate: 60.0,
                    speed_range: [1.0, 2.0],
                    lifetime_range: [1.5, 2.0],
                    size_range: [2.0, 3.0],
                    gravity: [0.0; 2],
                    color: [1.0, 0.5, 0.2, 0.4],
                    ..Default::default()
                }],
                ..Default::default()
            },
        }),
    ] {
        ComponentCommand {
            scene_id: "main".into(),
            entity_id: hero.clone(),
            edit,
        }
        .execute(&mut scene, &p, &mut history)
        .unwrap();
    }
    if let Some(Component::Sprite { width, .. }) = scene.entities[0]
        .components
        .iter_mut()
        .find(|c| matches!(c, Component::Sprite { .. }))
    {
        *width = 16.0;
    }
    let before = scene.clone();
    ComponentCommand {
        scene_id: "main".into(),
        entity_id: hero.clone(),
        edit: ComponentEdit::SetVisual(Component::SpriteLayout {
            settings: kiln_core::visual::SpriteLayout {
                flip_x: true,
                pixels_per_unit: 2.0,
                ..Default::default()
            },
        }),
    }
    .execute(&mut scene, &p, &mut history)
    .unwrap();
    let edited = scene.clone();
    scene = history.undo(scene).unwrap();
    assert_eq!(serde_value(&scene), serde_value(&before));
    scene = history.redo(scene).unwrap();
    assert_eq!(serde_value(&scene), serde_value(&edited));
    let mut map = TileMap {
        id: "level".into(),
        ..Default::default()
    };
    map.layers.push(TileMapLayer {
        name: "Ground".into(),
        tileset: "terrain".into(),
        z: -1,
        ..Default::default()
    });
    map.layers.push(TileMapLayer {
        name: "Foreground".into(),
        tileset: "terrain".into(),
        z: 5,
        ..Default::default()
    });
    p.tilesets.insert(
        "terrain".into(),
        TileSet {
            texture: "tiles/terrain.png".into(),
            tile_size: [16, 16],
            columns: 4,
            rows: 4,
            solid: vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
        },
    );
    p.tilemaps.insert("level".into(), map);
    let mut brush = kiln_editor::tilemap_editor::TilemapEditorState::new();
    brush.select_map(Some("level".into()));
    brush.begin_stroke(&p);
    assert_eq!(brush.fill(&mut p, [0, 0], [0, 0, 8, 1], 0).unwrap(), 8);
    brush.end_stroke(&p);
    brush.undo(&mut p);
    assert_eq!(
        p.tilemaps["level"].layers[0].tile_at(1, 0),
        kiln_core::EMPTY_TILE
    );
    brush.redo(&mut p);
    // A solid wall blocks movement; the decorative layer has a different Z.
    for y in 1..6 {
        brush.paint(&mut p, 5, y, 0);
    }
    brush.selected_layer = 1;
    brush.paint(&mut p, 3, 3, 1);
    scene.entities.push(Entity {
        id: "map".into(),
        name: "Level".into(),
        components: vec![
            Component::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
                scale: 1.0,
            },
            Component::Tilemap {
                map: "level".into(),
            },
        ],
    });
    p.scenes.insert("main".into(), scene);
    p.asset_ids
        .insert("art/hero.png".into(), "hero-image".into());
    p.asset_import_settings.insert(
        "hero-image".into(),
        kiln_core::project::AssetImportSettings {
            nearest_filter: true,
            is_sprite: true,
            generate_mipmaps: false,
        },
    );
    project_files::save(&p, &root.join("kiln.toml")).unwrap();
    p
}
fn serde_value(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}
#[derive(Debug, PartialEq)]
struct Simulation {
    poses: Vec<(String, [f32; 2])>,
    geometry: Vec<([f32; 2], [f32; 4])>,
    events: i64,
}
fn simulate(game: &kiln_player::loader::LoadedGame, fps: usize) -> Simulation {
    let mut paths = kiln_player::textures::texture_paths(&game.project);
    paths.sort();
    let bindings: HashMap<_, _> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let image = kiln_assets::decode_png(&game.assets[path]).unwrap();
            (
                path.clone(),
                (TextureId(index as u32 + 1), [image.width, image.height]),
            )
        })
        .collect();
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &game.project, "main", &mut |_, c, path| {
        let mut sprite = Sprite::new(c[0], c[1], c[2], c[3]);
        if let Some((texture, size)) = path.and_then(|path| bindings.get(path)) {
            sprite.texture = *texture;
            sprite.size = [size[0] as f32, size[1] as f32];
        }
        sprite
    });
    kiln_runtime::spawn_tilemaps(
        &mut world,
        &game.project,
        &game.project.scenes["main"],
        Default::default(),
        &mut |_| Some(bindings["tiles/terrain.png"].0),
    );
    let mut session = SliceSession::with_scene_sources(&game.project, "main", |path| {
        String::from_utf8(game.assets[path].clone()).map_err(|e| e.to_string())
    })
    .unwrap();
    let mut clock = kiln_runtime::play_clock::PlayClock::default();
    for _ in 0..fps * 2 {
        for _ in 0..clock.advance(1.0 / fps as f64) {
            session.step(&mut world, (1.0, 0.0), 1.0 / 60.0).unwrap();
        }
    }
    let mut poses: Vec<_> = world
        .world_ref()
        .query::<(&SourceId, &Transform)>()
        .iter()
        .map(|(id, pose)| (id.0.clone(), pose.position.to_array()))
        .collect();
    poses.sort_by(|a, b| a.0.cmp(&b.0));
    let hero = &game.project.scenes["main"].entities[0].id;
    let hero_pose = poses.iter().find(|(id, _)| id == hero).unwrap().1;
    assert!(
        hero_pose[0] <= 76.0 && hero_pose[0] >= 74.0,
        "{hero_pose:?}"
    );
    assert_eq!(
        world
            .world_ref()
            .query::<&AnimationPlayer>()
            .iter()
            .next()
            .unwrap()
            .current_clip(),
        Some("idle")
    );
    assert_eq!(
        session.entity_script_state(hero, "last_event"),
        Some(&kiln_ir::Value::Str("footstep".into()))
    );
    assert_eq!(
        session.entity_script_state(hero, "payload"),
        Some(&kiln_ir::Value::Str("stone".into()))
    );
    let events = session
        .entity_script_state(hero, "events")
        .unwrap()
        .as_i64()
        .unwrap();
    assert_eq!(events, 2);
    assert!(
        world
            .world_ref()
            .query::<&ParticleSystemComponent>()
            .iter()
            .next()
            .unwrap()
            .particle_count()
            > 60
    );
    let draws = kiln_runtime::visual::collect_scene_draws(&world, [0.0, 0.0, 320.0, 180.0]);
    let geometry = draws
        .iter()
        .map(|(_, draw)| (draw.position, draw.uv_rect))
        .collect();
    assert_eq!(draws.last().unwrap().1.position, [56.0, 56.0]);
    Simulation {
        poses,
        geometry,
        events,
    }
}
#[test]
fn authored_visuals_survive_restart_archive_and_player_runtime() {
    let root = tempfile::tempdir().unwrap();
    let p = author(root.path());
    let source = kiln_player::loader::load(root.path(), Some("main")).unwrap();
    assert_eq!(serde_value(&p), serde_value(&source.project));
    let output = tempfile::tempdir().unwrap();
    let archive_path = output.path().join("visuals.kar");
    let files = [
        "kiln.toml",
        "art/hero.png",
        "tiles/terrain.png",
        "scripts/main.ks",
        "scripts/hero.ks",
    ]
    .map(|path| root.path().join(path));
    kiln_assets::Archive::new(archive_path.clone())
        .pack(&files)
        .unwrap();
    let exported = kiln_player::loader::load(&archive_path, Some("main")).unwrap();
    assert_eq!(simulate(&source, 30), simulate(&exported, 144));
    assert_eq!(
        serde_value(&p),
        serde_value(&project_files::load(root.path()).unwrap().project)
    );
    if let Some(destination) = std::env::var_os("KILN_TASK6_SAMPLE_OUT") {
        let destination = Path::new(&destination);
        std::fs::create_dir_all(destination).unwrap();
        for path in [
            "kiln.toml",
            "art/hero.png",
            "tiles/terrain.png",
            "scripts/main.ks",
            "scripts/hero.ks",
        ] {
            let target = destination.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(root.path().join(path), target).unwrap();
        }
    }
}
