use kiln_core::{Component, Project};
use kiln_render::TextureId;
use kiln_runtime::{AnimationPlayer, GameWorld, Sprite};
fn project() -> Project {
    Project::from_toml(include_str!("../../samples/advanced_animation/kiln.toml")).unwrap()
}
fn mesh(p: &Project) -> kiln_core::skin::SkinMesh {
    p.scenes["main"]
        .entities
        .iter()
        .flat_map(|e| &e.components)
        .find_map(|c| match c {
            Component::SkinnedMesh { mesh } => Some(mesh.clone()),
            _ => None,
        })
        .unwrap()
}
fn player() -> AnimationPlayer {
    let p = project();
    let mut a = AnimationPlayer::new();
    for (id, clip) in &p.animation_clips {
        a.add_bound_clip(
            clip.clone(),
            if id == "red" {
                TextureId(1)
            } else {
                TextureId(2)
            },
            [1, 1],
        );
    }
    a
}
#[test]
fn skin_reconstructs_bind_and_blends_bone_deformations_per_vertex() {
    let p = project();
    let mesh = mesh(&p);
    mesh.validate_scene(&p.scenes["main"]).unwrap();
    let mut poses = kiln_core::hierarchy_transform::world_transforms(&p.scenes["main"]).unwrap();
    let rest = mesh.deform(&poses).unwrap();
    assert_eq!(rest[0], [80.0, 104.0]);
    assert_eq!(rest[4], [240.0, 104.0]);
    poses.get_mut("lower").unwrap().position.x += 32.0;
    let deformed = mesh.deform(&poses).unwrap();
    assert_eq!(deformed[0], rest[0]);
    assert_eq!(deformed[2][0], rest[2][0] + 16.0);
    assert_eq!(deformed[4][0], rest[4][0] + 32.0);
    poses.get_mut("lower").unwrap().rotation = std::f32::consts::FRAC_PI_2;
    let bent = mesh.deform(&poses).unwrap();
    assert_ne!(bent[2], deformed[2]);
    assert_ne!(bent[4], deformed[4]);
    let mut bad = mesh.clone();
    bad.vertices[0].influences[0].weight = 0.5;
    assert!(bad.validate().is_err());
    bad = mesh.clone();
    bad.triangles[0][0] = 999;
    assert!(bad.validate().is_err());
    poses.remove("lower");
    assert!(mesh.deform(&poses).is_err());
}
#[test]
fn crossfade_has_exact_endpoints_pause_seek_and_saved_resume() {
    let mut a = player();
    a.play("red");
    a.crossfade("green", 1.0).unwrap();
    let mut sprite = Sprite::new(1.0, 1.0, 1.0, 1.0);
    a.apply_to_sprite(&mut sprite);
    assert_eq!(sprite.blend.as_ref().unwrap().weight, 1.0);
    a.update(0.5);
    a.apply_to_sprite(&mut sprite);
    assert_eq!(sprite.texture, TextureId(2));
    assert_eq!(sprite.blend.as_ref().unwrap().weight, 0.5);
    a.update(0.0);
    let saved = a.snapshot().unwrap();
    let mut restored = AnimationPlayer::restore_snapshot(&saved, |path| {
        Some((
            if path.contains("red") {
                TextureId(1)
            } else {
                TextureId(2)
            },
            [1, 1],
        ))
    })
    .unwrap();
    restored.apply_to_sprite(&mut sprite);
    assert_eq!(sprite.blend.as_ref().unwrap().weight, 0.5);
    restored.update(0.5);
    restored.apply_to_sprite(&mut sprite);
    assert!(sprite.blend.is_none());
    assert_eq!(sprite.texture, TextureId(2));
    a.seek(0.0);
    a.apply_to_sprite(&mut sprite);
    assert!(sprite.blend.is_none());
    let mut corrupt = saved;
    corrupt.fade.as_mut().unwrap().elapsed = 1.0;
    assert!(AnimationPlayer::restore_snapshot(&corrupt, |_| Some((TextureId(1), [1, 1]))).is_err());
}
#[test]
fn automatic_tree_selects_adjacent_textures_without_restarting_clock() {
    let p = project();
    let graph = p.scenes["main"]
        .entities
        .iter()
        .find(|e| e.id == "tree")
        .unwrap()
        .components
        .iter()
        .find_map(|c| match c {
            Component::AnimationGraph { graph } => Some(graph.clone()),
            _ => None,
        })
        .unwrap();
    let mut a = player();
    a.bind_graph(graph.clone()).unwrap();
    a.update(0.25);
    let mut sprite = Sprite::new(1.0, 1.0, 1.0, 1.0);
    a.set_graph_parameter("speed", 0.5).unwrap();
    a.apply_to_sprite(&mut sprite);
    assert_eq!(sprite.texture, TextureId(1));
    assert_eq!(sprite.blend.as_ref().unwrap().texture, TextureId(2));
    assert_eq!(sprite.blend.as_ref().unwrap().weight, 0.5);
    assert_eq!(a.elapsed(), 0.25);
    a.set_graph_parameter("speed", 2.0).unwrap();
    a.apply_to_sprite(&mut sprite);
    assert_eq!(sprite.texture, TextureId(2));
    assert!(sprite.blend.is_none());
    let saved = a.snapshot().unwrap();
    let b = AnimationPlayer::restore_snapshot(&saved, |path| {
        Some((
            if path.contains("red") {
                TextureId(1)
            } else {
                TextureId(2)
            },
            [1, 1],
        ))
    })
    .unwrap();
    assert_eq!(b.graph_parameters().unwrap()["speed"], 2.0);
    let mut bad = graph;
    bad.blend_trees.get_mut("move").unwrap().points[1].threshold = 0.0;
    assert!(bad.validate(&p.animation_clips).is_err());
}
#[test]
fn skin_editor_apply_undo_and_save_reopen_keep_bindings() {
    use kiln_editor::{
        component_command::{ComponentCommand, ComponentEdit},
        history::SceneHistory,
    };
    let p = project();
    let mut scene = p.scenes["main"].clone();
    let entity = scene.entities.iter().find(|e| e.id == "skin").unwrap();
    let generated =
        kiln_editor::skin_editor::grid(&scene, entity, ["upper".into(), "lower".into()], 5, 2)
            .unwrap();
    let mut history = SceneHistory::new();
    let mut edit = generated.clone();
    edit.vertices[2].influences[0].weight = 0.25;
    edit.vertices[2].influences[1].weight = 0.75;
    ComponentCommand {
        scene_id: "main".into(),
        entity_id: "skin".into(),
        edit: ComponentEdit::SetVisual(Component::SkinnedMesh { mesh: edit.clone() }),
    }
    .execute(&mut scene, &p, &mut history)
    .unwrap();
    let source = serde_json::to_string(&scene).unwrap();
    let reopened: kiln_core::Scene = serde_json::from_str(&source).unwrap();
    let saved = reopened
        .entities
        .iter()
        .flat_map(|e| &e.components)
        .find_map(|c| match c {
            Component::SkinnedMesh { mesh } => Some(mesh),
            _ => None,
        })
        .unwrap();
    assert_eq!(saved, &edit);
    let before = history.undo(scene).unwrap();
    assert_ne!(serde_json::to_string(&before).unwrap(), source);
}
#[test]
fn mesh_and_fade_survive_logical_game_save() {
    let p = project();
    let source = include_str!("../../samples/advanced_animation/scripts/main.ks");
    let mut s =
        kiln_runtime::SliceSession::with_scene_sources(&p, "main", |_| Ok(source.into())).unwrap();
    let mut w = GameWorld::new();
    kiln_runtime::spawn_scene(&mut w, &p, "main", &mut |_, color, path| {
        let mut sprite = Sprite::new(color[0], color[1], color[2], color[3]);
        sprite.texture = if path.is_some_and(|p| p.contains("red")) {
            TextureId(1)
        } else {
            TextureId(2)
        };
        sprite.with_size(1.0, 1.0)
    });
    let h = kiln_runtime::tweens::handle(&w, "fade").unwrap();
    w.world_mut()
        .get::<&mut AnimationPlayer>(h)
        .unwrap()
        .crossfade("green", 1.0)
        .unwrap();
    s.step(&mut w, (0.0, 0.0), 0.5).unwrap();
    let saved = s.snapshot(&w).unwrap();
    let decoded = kiln_runtime::GameSave::from_json(&saved.to_json().unwrap()).unwrap();
    let (_, other) = decoded
        .restore(
            &p,
            |_| Ok(source.into()),
            |path| {
                Some((
                    if path.contains("red") {
                        TextureId(1)
                    } else {
                        TextureId(2)
                    },
                    [1, 1],
                ))
            },
        )
        .unwrap();
    let h = kiln_runtime::tweens::handle(&other, "skin").unwrap();
    assert_eq!(
        *other
            .world_ref()
            .get::<&kiln_core::skin::SkinMesh>(h)
            .unwrap(),
        mesh(&p)
    );
    let h = kiln_runtime::tweens::handle(&other, "fade").unwrap();
    assert!(other
        .world_ref()
        .get::<&AnimationPlayer>(h)
        .unwrap()
        .snapshot()
        .unwrap()
        .fade
        .is_some());
    assert!(other.world_ref().get::<&Sprite>(h).unwrap().blend.is_some());
}
