//! Async browser preloading followed by the shared synchronous scene/module loader.
use super::*;
pub(super) type Textures = HashMap<String, (TextureId, [u32; 2])>;

pub(super) async fn scripts(
    project: &kiln_core::Project,
    project_path: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut loaded = HashMap::new();
    let mut ids: Vec<_> = project.scenes.keys().collect();
    ids.sort();
    for _ in 0..128 {
        let missing = std::cell::RefCell::new(std::collections::BTreeSet::new());
        let mut errors = Vec::new();
        for id in &ids {
            if let Err(error) = SliceSession::with_scene_sources(project, id, |path| {
                loaded.get(path).cloned().ok_or_else(|| {
                    missing.borrow_mut().insert(path.to_owned());
                    "Resource not yet fetched".into()
                })
            }) {
                errors.push(error);
            }
        }
        let missing = missing.into_inner();
        if missing.is_empty() {
            if let Some(error) = errors.first() {
                return Err(error.clone().into());
            }
            return Ok(loaded);
        }
        for path in missing {
            if loaded.len() >= 128 {
                return Err("Script module resource limit reached".into());
            }
            let text = load_script(&path, project_path).await?;
            if text.len() > 1_048_576 {
                return Err(format!("Script exceeds 1 MiB: {path}").into());
            }
            loaded.insert(path, text);
        }
    }
    Err("Script module preloading did not converge".into())
}

pub(super) fn session(
    project: &kiln_core::Project,
    scene: &str,
    scripts: &HashMap<String, String>,
    textures: &Textures,
) -> Result<(SliceSession, GameWorld), String> {
    let session = SliceSession::with_scene_sources(project, scene, |path| {
        scripts
            .get(path)
            .cloned()
            .ok_or_else(|| format!("Missing script {path}"))
    })?;
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, project, scene, &mut |_, color, path| {
        let mut sprite = kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3]);
        if let Some((id, size)) = path.and_then(|p| textures.get(p)) {
            sprite.texture = *id;
            sprite.size = [size[0] as f32, size[1] as f32];
        }
        sprite
    });
    if let Some(scene) = project.scenes.get(scene) {
        kiln_runtime::spawn_tilemaps(
            &mut world,
            project,
            scene,
            kiln_core::SceneOffset::default(),
            &mut |id| {
                project
                    .tilesets
                    .get(id)
                    .and_then(|s| textures.get(&s.texture))
                    .map(|t| t.0)
            },
        );
    }
    for animation in world
        .world_mut()
        .query_mut::<&mut kiln_runtime::AnimationPlayer>()
    {
        for clip in project.animation_clips.values() {
            if let Some((id, size)) = textures.get(&clip.texture) {
                animation.add_bound_clip(clip.clone(), *id, *size);
            }
        }
    }
    kiln_runtime::update_global_transforms(&mut world)?;
    Ok((session, world))
}
