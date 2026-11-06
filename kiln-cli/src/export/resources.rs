use super::*;
use kiln_core::{Component, Project};

pub fn read_project(path: &Path) -> Result<Project> {
    let text = fs::read_to_string(path)?;
    let project = if path.extension().is_some_and(|v| v == "ron") {
        ron::from_str::<Project>(&text).map_err(|e| e.to_string())
    } else {
        Project::from_toml(&text)
    }
    .map_err(anyhow::Error::msg)?;
    if project.id.is_empty() || project.scenes.is_empty() {
        bail!("Project needs an ID and at least one scene");
    }
    project.ui.validate().map_err(anyhow::Error::msg)?;
    Ok(project)
}

fn excluded(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "target" | "export" | "dist")
}
pub fn snapshot(
    root: &Path,
    manifest: &Path,
    output: &Path,
    stage: &Path,
    snapshot: &Path,
    marker: &Path,
    project: &Project,
) -> Result<Vec<PathBuf>> {
    let walk = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.path() == root
                || (!excluded(&entry.file_name().to_string_lossy())
                    && !entry.path().starts_with(output)
                    && !entry.path().starts_with(stage))
        });
    let mut files = Vec::new();
    let mut total = 0u64;
    for entry in walk {
        cancelled(marker)?;
        let entry = entry?;
        if entry.file_type().is_symlink() {
            bail!(
                "Project symlinks cannot be exported: {}",
                entry.path().display()
            );
        }
        if !entry.file_type().is_file() || entry.path() == manifest {
            continue;
        }
        let relative = entry.path().strip_prefix(root)?;
        // Source manifests are normalized into one authoritative kiln.toml.
        if relative == Path::new("kiln.toml") || relative == Path::new("kiln.ron") {
            continue;
        }
        total = total.saturating_add(entry.metadata()?.len());
        if total > 2 * 1024 * 1024 * 1024 || files.len() >= 99_999 {
            bail!("Project export exceeds 2 GiB or 100,000 files");
        }
        let target = snapshot.join(relative);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(entry.path(), &target)?;
        files.push(target);
    }
    let manifest = snapshot.join("kiln.toml");
    fs::write(&manifest, project.to_toml().map_err(anyhow::Error::msg)?)?;
    files.push(manifest);
    files.sort();
    Ok(files)
}

fn resource(root: &Path, path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    if path.is_empty()
        || relative
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        bail!("Resource must be project-relative: {path}");
    }
    let file = root.join(relative);
    if !file.is_file() {
        bail!("Required resource is missing or excluded from export: {path}");
    }
    Ok(file)
}
pub fn validate(project: &Project, root: &Path) -> Result<()> {
    let check = |path: &str| resource(root, path).map(|_| ());
    let image = |path: &str| image::open(resource(root, path)?).map_err(anyhow::Error::from);
    if !project.icon.is_empty() {
        check(&project.icon)?;
    }
    for scene in project.scenes.values() {
        kiln_core::hierarchy_transform::world_transforms(scene).map_err(anyhow::Error::msg)?;
        kiln_runtime::SliceSession::with_scene_sources(project, &scene.id, |path| {
            resource(root, path)
                .and_then(|p| Ok(fs::read_to_string(p)?))
                .map_err(|e| e.to_string())
        })
        .map_err(anyhow::Error::msg)?;
        for entity in &scene.entities {
            for component in &entity.components {
                match component {
                    Component::Sprite {
                        texture: Some(path),
                        ..
                    } => {
                        image(path)?;
                    }
                    Component::Particles { effect } => {
                        effect.validate().map_err(anyhow::Error::msg)?
                    }
                    _ => {}
                }
            }
        }
    }
    for prefab in project.prefabs.values() {
        for entity in [&prefab.root_entity] {
            for c in &entity.components {
                if let Component::Sprite {
                    texture: Some(path),
                    ..
                } = c
                {
                    image(path)?;
                }
            }
        }
    }
    let mut audio = kiln_runtime::audio::AudioEngine::default();
    for asset in &project.audio_assets {
        audio
            .register_bytes(asset, fs::read(resource(root, &asset.path)?)?)
            .map_err(anyhow::Error::msg)?;
    }
    for (id, font) in &project.ui.fonts {
        let chars = font
            .extra_chars
            .chars()
            .chain(project.ui.screens.values().flat_map(|screen| {
                screen
                    .widgets
                    .iter()
                    .filter(|widget| &widget.font == id)
                    .flat_map(|widget| widget.text.chars())
            }));
        kiln_ui::authored::fonts::build_atlas(&fs::read(resource(root, &font.path)?)?, chars)
            .map_err(anyhow::Error::msg)?;
    }
    for tileset in project.tilesets.values() {
        image(&tileset.texture)?;
    }
    for clip in project.animation_clips.values() {
        let image = image::open(resource(root, &clip.texture)?)?;
        clip.validate([image.width(), image.height()])
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
}
