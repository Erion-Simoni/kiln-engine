//! Publish atlas recipes using guarded single-file replacement.
use crate::{
    atlas_provenance as provenance,
    atlas_recipe::{self, BakedAtlas},
};
use kiln_core::{atlas::AtlasRecipe, Project};
use std::{
    fs,
    io::Read,
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
        let owned = owned
            || provenance::owns(
                root,
                &project.id,
                id,
                &recipe.output,
                &atlas_recipe::image_hash(&bytes),
            )?;
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

/// Stage and atomically replace one generated PNG, then publish the recipe/clips
/// in memory. Caller saves the project document after success.
pub fn build(
    project: &mut Project,
    root: &Path,
    id: &str,
    recipe: &AtlasRecipe,
) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let _lock = provenance::lock(&root)?;
    let (baked, output, original) = prepare(project, &root, id, recipe)?;
    if original.as_deref() != Some(baked.png.as_slice()) {
        provenance::record(
            &root,
            &project.id,
            id,
            &recipe.output,
            &baked.recipe.built_hash,
        )?;
        let current = if output.exists() {
            Some(read(&root, &recipe.output)?)
        } else {
            None
        };
        if current != original {
            return Err("Atlas output changed while building; no files were replaced".into());
        }
        provenance::replace(&output, &baked.png)?;
    }
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

/// Reconstruct interrupted, known-generated outputs for the selected saved or
/// recovered project. Original art and independently edited outputs are preserved.
/// Reports per-recipe failures so a missing source does not hide a recovery draft.
pub fn recover(project: &mut Project, root: &Path) -> Vec<String> {
    if !root.join(provenance::FILE).exists() {
        return Vec::new();
    }
    let generations = match provenance::read(root) {
        Ok(generations) => generations,
        Err(error) => return vec![error],
    };
    let mut ids: Vec<_> = project.atlases.keys().cloned().collect();
    ids.sort();
    let mut messages = Vec::new();
    for id in ids {
        let recipe = project.atlases[&id].clone();
        if !generations
            .iter()
            .any(|g| g.project == project.id && g.atlas == id && g.output == recipe.output)
        {
            continue;
        }
        let current = read(root, &recipe.output)
            .ok()
            .map(|bytes| atlas_recipe::image_hash(&bytes));
        if current.as_deref() == Some(recipe.built_hash.as_str()) {
            continue;
        }
        match build(project, root, &id, &recipe) {
            Ok(()) => messages.push(format!(
                "Reconstructed interrupted atlas {id}; Save Project to commit generated metadata"
            )),
            Err(error) => messages.push(format!("Interrupted atlas {id} needs attention: {error}")),
        }
    }
    messages
}

/// Forget provenance only for outputs consistent with a successfully saved manifest.
/// Cleanup failure is nonfatal after the manifest's atomic replacement succeeded.
pub fn finish_saved(project: &Project, root: &Path) -> Result<(), String> {
    if !root.join(provenance::FILE).exists() {
        return Ok(());
    }
    let _lock = provenance::lock(root)?;
    let mut generations = provenance::read(root)?;
    let mut committed = Vec::new();
    for (id, recipe) in &project.atlases {
        if let Ok(bytes) = read(root, &recipe.output) {
            if atlas_recipe::image_hash(&bytes) == recipe.built_hash {
                committed.push((id.as_str(), recipe.output.as_str()));
            }
        }
    }
    generations.retain(|g| {
        g.project != project.id
            || !committed
                .iter()
                .any(|(id, path)| *id == g.atlas && *path == g.output)
    });
    if generations.is_empty() {
        fs::remove_file(root.join(provenance::FILE)).map_err(|e| e.to_string())?;
    } else {
        provenance::write(root, generations)?;
    }
    Ok(())
}

/// Copy only validated provenance into an isolated Save As/export snapshot.
/// Source metadata is read atomically and never modified or locked for writing.
pub fn copy_provenance(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.join(provenance::FILE).exists() {
        return Ok(());
    }
    provenance::write(destination, provenance::read(source)?)
}

/// Remove private bookkeeping from an owned export staging directory.
pub fn strip_export_provenance(root: &Path) -> Result<(), String> {
    for file in [provenance::FILE, provenance::LOCK] {
        match fs::remove_file(root.join(file)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

/// Private atlas bookkeeping excluded from copied authoring resources.
pub fn private_name(name: &str) -> bool {
    name == provenance::FILE
        || name == provenance::LOCK
        || name.starts_with(".kiln-atlas-") && name.ends_with(".tmp")
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Project, AtlasRecipe) {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("hero.png"),
            crate::encode_png_rgba(2, 2, &[255, 0, 0, 255].repeat(4)).unwrap(),
        )
        .unwrap();
        let recipe = AtlasRecipe {
            output: "atlas.png".into(),
            size: 16,
            padding: 1,
            sources: [("hero".into(), "hero.png".into())].into(),
            ..Default::default()
        };
        let mut project = Project::default();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        finish_saved(&project, root.path()).unwrap();
        (root, project, recipe)
    }
    fn json(project: &Project) -> serde_json::Value {
        serde_json::to_value(project).unwrap()
    }
    #[test]
    fn saved_and_older_recovery_recipes_reconstruct_after_multiple_uncommitted_builds() {
        let (root, saved, mut recipe) = fixture();
        let source = fs::read(root.path().join("hero.png")).unwrap();
        let saved_png = fs::read(root.path().join("atlas.png")).unwrap();
        let mut draft = saved.clone();
        recipe.size = 32;
        build(&mut draft, root.path(), "game", &recipe).unwrap();
        let recovered_32 = draft.clone();
        let png_32 = fs::read(root.path().join("atlas.png")).unwrap();
        recipe.size = 64;
        build(&mut draft, root.path(), "game", &recipe).unwrap();
        assert_eq!(
            crate::decode_png(&fs::read(root.path().join("atlas.png")).unwrap())
                .unwrap()
                .width,
            64
        );
        let mut reopened = saved.clone();
        assert!(recover(&mut reopened, root.path())
            .iter()
            .any(|m| m.starts_with("Reconstructed")));
        assert_eq!(json(&reopened), json(&saved));
        assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), saved_png);
        let mut restored: Project =
            serde_json::from_slice(&serde_json::to_vec(&recovered_32).unwrap()).unwrap();
        recover(&mut restored, root.path());
        assert_eq!(json(&restored), json(&recovered_32));
        assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), png_32);
        finish_saved(&restored, root.path()).unwrap();
        assert!(!root.path().join(provenance::FILE).exists());
        assert_eq!(fs::read(root.path().join("hero.png")).unwrap(), source);
    }
    #[test]
    fn recovery_preserves_independent_output_edits_and_foreign_project_outputs() {
        let (root, saved, mut recipe) = fixture();
        let mut draft = saved.clone();
        recipe.size = 32;
        build(&mut draft, root.path(), "game", &recipe).unwrap();
        let foreign = Project {
            id: "different-project".into(),
            ..saved.clone()
        };
        let mut foreign_loaded = foreign.clone();
        let before = fs::read(root.path().join("atlas.png")).unwrap();
        assert!(recover(&mut foreign_loaded, root.path()).is_empty());
        assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), before);
        fs::write(
            root.path().join("atlas.png"),
            crate::encode_png_rgba(3, 3, &[0, 255, 0, 255].repeat(9)).unwrap(),
        )
        .unwrap();
        let external = fs::read(root.path().join("atlas.png")).unwrap();
        let mut reopened = saved.clone();
        assert!(recover(&mut reopened, root.path())
            .iter()
            .any(|m| m.contains("needs attention")));
        assert_eq!(json(&reopened), json(&saved));
        assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), external);
    }
    #[test]
    fn invalid_provenance_and_active_publisher_refuse_before_replacing_output() {
        let (root, mut project, mut recipe) = fixture();
        let before = fs::read(root.path().join("atlas.png")).unwrap();
        let original = json(&project);
        recipe.size = 32;
        let lock = provenance::lock(root.path()).unwrap();
        assert!(build(&mut project, root.path(), "game", &recipe)
            .unwrap_err()
            .contains("lock"));
        drop(lock);
        fs::write(root.path().join(provenance::FILE), b"{invalid}").unwrap();
        assert!(build(&mut project, root.path(), "game", &recipe).is_err());
        assert_eq!(json(&project), original);
        assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), before);
        assert_eq!(
            fs::read(root.path().join(provenance::FILE)).unwrap(),
            b"{invalid}"
        );
    }
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
    fn dormant_prefab_clip_override_prevents_generated_clip_removal() {
        let (root, mut project, recipe) = fixture();
        build(&mut project, root.path(), "game", &recipe).unwrap();
        let mut overrides = kiln_core::PrefabOverride::new("template".into());
        overrides.overrides.insert(
            "sprite_resources".into(),
            kiln_core::ComponentOverride::SpriteResources {
                material: None,
                animation_clip: Some(Some(atlas::clip_name("game", "hero"))),
            },
        );
        project.prefabs.insert(
            "variant".into(),
            kiln_core::Prefab {
                id: "variant".into(),
                name: "Dormant link".into(),
                root_entity: Entity {
                    id: "template".into(),
                    name: "Template".into(),
                    components: vec![Component::PrefabInstance {
                        prefab_id: "base".into(),
                        overrides,
                    }],
                },
            },
        );
        let before = json(&project);
        let png = fs::read(root.path().join("sheet.png")).unwrap();
        let mut changed = recipe.clone();
        changed.sources.remove("hero");
        assert!(build(&mut project, root.path(), "game", &changed)
            .unwrap_err()
            .contains("still referenced"));
        assert_eq!(json(&project), before);
        assert_eq!(fs::read(root.path().join("sheet.png")).unwrap(), png);
        project.prefabs.clear();
        build(&mut project, root.path(), "game", &changed).unwrap();
        assert!(!project
            .animation_clips
            .contains_key(&atlas::clip_name("game", "hero")));
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
