//! Publish atlas recipes using guarded single-file replacement.
use crate::atlas_recipe::{self, BakedAtlas};
use kiln_core::{atlas::AtlasRecipe, Project};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    atlas_recipe::png_path(relative)?;
    let mut full = root.to_path_buf();
    for part in relative.split('/') {
        full.push(part);
        if let Ok(metadata) = fs::symlink_metadata(&full) {
            if metadata.file_type().is_symlink() {
                return Err("Atlas paths cannot contain symbolic links".into());
            }
        }
    }
    if !full
        .parent()
        .ok_or("Missing atlas directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err("Atlas path leaves the project".into());
    }
    Ok(full)
}

fn read(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let full = path(root, relative)?;
    let mut bytes = Vec::new();
    fs::File::open(&full)
        .map_err(|e| format!("{relative}: {e}"))?
        .take(atlas_recipe::MAX_PNG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > atlas_recipe::MAX_PNG_BYTES {
        return Err(format!("{relative}: PNG exceeds 80 MiB"));
    }
    Ok(bytes)
}

fn prepare(
    project: &Project,
    root: &Path,
    id: &str,
    recipe: &AtlasRecipe,
) -> Result<(BakedAtlas, PathBuf, Option<Vec<u8>>), String> {
    atlas_recipe::validate_clips(project, id, recipe)?;
    for (other_id, other) in &project.atlases {
        if other_id != id && other.output == recipe.output {
            return Err("Another atlas owns this output path".into());
        }
        if recipe
            .sources
            .values()
            .any(|source| source == &other.output)
            || other_id != id
                && other
                    .sources
                    .values()
                    .any(|source| source == &recipe.output)
        {
            return Err(
                "Use original PNG artwork as atlas inputs, rather than another generated atlas"
                    .into(),
            );
        }
    }
    let output = path(root, &recipe.output)?;
    let original = if output.exists() {
        let bytes = read(root, &recipe.output)?;
        let owned = project.atlases.get(id).is_some_and(|old| {
            old.output == recipe.output && old.built_hash == atlas_recipe::image_hash(&bytes)
        });
        if !owned {
            return Err("Atlas output already exists or changed externally; choose a new path or restore the generated file".into());
        }
        Some(bytes)
    } else {
        None
    };
    let baked = atlas_recipe::bake(id, recipe, |source| read(root, source))?;
    Ok((baked, output, original))
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Stage and atomically replace one generated PNG, then publish the recipe/clips
/// in memory. Caller saves the project document after success.
pub fn build(
    project: &mut Project,
    root: &Path,
    id: &str,
    recipe: &AtlasRecipe,
) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let (baked, output, original) = prepare(project, &root, id, recipe)?;
    let temporary = Temporary(
        output
            .parent()
            .ok_or("Missing output directory")?
            .join(format!(".kiln-atlas-{}.tmp", kiln_core::next_id())),
    );
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary.0)
        .map_err(|e| e.to_string())?;
    file.write_all(&baked.png).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    let current = if output.exists() {
        Some(read(&root, &recipe.output)?)
    } else {
        None
    };
    if current != original {
        return Err("Atlas output changed while building; no files were replaced".into());
    }
    fs::rename(&temporary.0, &output).map_err(|e| e.to_string())?;
    atlas_recipe::apply(project, id, &baked);
    let asset_id = project.ensure_asset_ids([recipe.output.as_str()])[0].clone();
    project.asset_import_settings.entry(asset_id).or_insert(
        kiln_core::project::AssetImportSettings {
            nearest_filter: true,
            generate_mipmaps: false,
            is_sprite: true,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::{atlas, Component, Entity, Scene};
    fn png(root: &Path, path: &str, color: [u8; 4]) {
        fs::write(
            root.join(path),
            crate::encode_png_rgba(2, 2, &color.repeat(4)).unwrap(),
        )
        .unwrap();
    }
    fn fixture() -> (tempfile::TempDir, Project, AtlasRecipe) {
        let root = tempfile::tempdir().unwrap();
        png(root.path(), "hero.png", [255, 0, 0, 255]);
        png(root.path(), "coin.png", [0, 255, 0, 255]);
        let recipe = AtlasRecipe {
            output: "sheet.png".into(),
            size: 16,
            padding: 1,
            sources: [
                ("hero".into(), "hero.png".into()),
                ("coin".into(), "coin.png".into()),
            ]
            .into(),
            ..Default::default()
        };
        (root, Project::default(), recipe)
    }
    fn json(project: &Project) -> serde_json::Value {
        serde_json::to_value(project).unwrap()
    }
    #[test]
    fn recipes_rebuild_renamed_sources_without_changing_regions_clips_or_asset_ids() {
        let (root, mut project, recipe) = fixture();
        project
            .asset_ids
            .insert("hero.png".into(), "stable-hero".into());
        build(&mut project, root.path(), "game", &recipe).unwrap();
        let original = project.clone();
        let original_png = fs::read(root.path().join("sheet.png")).unwrap();
        fs::rename(
            root.path().join("hero.png"),
            root.path().join("renamed.png"),
        )
        .unwrap();
        assert_eq!(project.rewrite_resource_path("hero.png", "renamed.png"), 1);
        let recipe = project.atlases["game"].clone();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        assert_eq!(project.asset_ids["renamed.png"], "stable-hero");
        assert_eq!(
            project.asset_ids["sheet.png"],
            original.asset_ids["sheet.png"]
        );
        assert_eq!(
            project.atlases["game"].built_regions,
            original.atlases["game"].built_regions
        );
        assert_eq!(
            fs::read(root.path().join("sheet.png")).unwrap(),
            original_png
        );
        assert_eq!(
            serde_json::to_value(&project.animation_clips).unwrap(),
            serde_json::to_value(&original.animation_clips).unwrap()
        );
        let reopened = Project::from_toml(&project.to_toml().unwrap()).unwrap();
        assert_eq!(json(&reopened), json(&project));
        assert!(project.resource_dependencies()["renamed.png"]
            .contains(&"atlas:game/source:hero".into()));
        png(root.path(), "renamed.png", [0, 0, 255, 255]);
        build(&mut project, root.path(), "game", &recipe).unwrap();
        let image = crate::decode_png(&fs::read(root.path().join("sheet.png")).unwrap()).unwrap();
        let [x, y, w, h] = project.atlases["game"].built_regions["hero"];
        assert_eq!([w, h], [2, 2]);
        for row in y..y + h {
            for column in x..x + w {
                let at = ((row * image.width + column) * 4) as usize;
                assert_eq!(&image.pixels[at..at + 4], &[0, 0, 255, 255]);
            }
        }
    }
    #[test]
    fn output_collision_external_edits_and_clip_edits_preserve_everything() {
        let (root, mut project, recipe) = fixture();
        png(root.path(), "sheet.png", [0, 0, 0, 255]);
        let before = fs::read(root.path().join("sheet.png")).unwrap();
        let original = json(&project);
        assert!(build(&mut project, root.path(), "game", &recipe).is_err());
        assert_eq!(json(&project), original);
        assert_eq!(fs::read(root.path().join("sheet.png")).unwrap(), before);
        fs::remove_file(root.path().join("sheet.png")).unwrap();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        let original = json(&project);
        png(root.path(), "sheet.png", [0, 0, 0, 255]);
        let external = fs::read(root.path().join("sheet.png")).unwrap();
        assert!(build(&mut project, root.path(), "game", &recipe).is_err());
        assert_eq!(json(&project), original);
        assert_eq!(fs::read(root.path().join("sheet.png")).unwrap(), external);
        fs::remove_file(root.path().join("sheet.png")).unwrap();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        project
            .animation_clips
            .get_mut("atlas:game:hero")
            .unwrap()
            .frames[0]
            .duration = 0.5;
        let original = json(&project);
        let before = fs::read(root.path().join("sheet.png")).unwrap();
        assert!(build(&mut project, root.path(), "game", &recipe)
            .unwrap_err()
            .contains("was edited"));
        assert_eq!(json(&project), original);
        assert_eq!(fs::read(root.path().join("sheet.png")).unwrap(), before);
    }
    #[test]
    fn referenced_generated_clips_cannot_be_removed_by_recipe_edits() {
        let (root, mut project, recipe) = fixture();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![],
                entities: vec![Entity {
                    id: "actor".into(),
                    name: "Actor".into(),
                    components: vec![Component::Sprite {
                        color: [1.0; 4],
                        width: 32.0,
                        height: 32.0,
                        texture: Some("sheet.png".into()),
                        layer: 0.0,
                        material: None,
                        animation_clip: Some(atlas::clip_name("game", "hero")),
                    }],
                }],
            },
        );
        let before = json(&project);
        let mut changed = recipe.clone();
        changed.sources.remove("hero");
        assert!(build(&mut project, root.path(), "game", &changed)
            .unwrap_err()
            .contains("still referenced"));
        assert_eq!(json(&project), before);
    }
    #[test]
    fn fit_and_path_rejection_leave_project_and_generated_file_unchanged() {
        let (root, mut project, recipe) = fixture();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        let before = json(&project);
        let bytes = fs::read(root.path().join("sheet.png")).unwrap();
        for output in [
            "../sheet.png",
            "target/sheet.png",
            "/sheet.png",
            "bad\\name.png",
        ] {
            let mut invalid = recipe.clone();
            invalid.output = output.into();
            assert!(build(&mut project, root.path(), "game", &invalid).is_err());
            assert_eq!(json(&project), before);
        }
        let mut too_small = recipe.clone();
        too_small.size = 2;
        assert!(build(&mut project, root.path(), "game", &too_small).is_err());
        assert_eq!(json(&project), before);
        assert_eq!(fs::read(root.path().join("sheet.png")).unwrap(), bytes);
        let mut extreme = crate::encode_png_rgba(1, 1, &[0; 4]).unwrap();
        extreme[16..24].fill(255);
        assert!(atlas_recipe::bake("game", &recipe, |_| Ok(extreme.clone())).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn atlas_source_and_output_symlinks_are_rejected() {
        let (root, mut project, mut recipe) = fixture();
        let external = tempfile::tempdir().unwrap();
        let before = json(&project);
        png(external.path(), "external.png", [0; 4]);
        std::os::unix::fs::symlink(
            external.path().join("external.png"),
            root.path().join("link.png"),
        )
        .unwrap();
        recipe.output = "link.png".into();
        assert!(build(&mut project, root.path(), "game", &recipe).is_err());
        recipe.output = "sheet.png".into();
        recipe.sources.insert("linked".into(), "link.png".into());
        assert!(build(&mut project, root.path(), "game", &recipe).is_err());
        assert!(!root.path().join("sheet.png").exists());
        assert_eq!(json(&project), before);
    }
}
