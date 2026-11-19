//! Interrupted derived-resource publication must not corrupt saved authoring state.
use kiln_core::{atlas::AtlasRecipe, Project};
use kiln_editor::project_files;
use std::{fs, path::Path};

fn fixture(root: &Path) -> Project {
    for (name, color) in [
        ("hero.png", [255, 0, 0, 255]),
        ("coin.png", [0, 255, 0, 255]),
    ] {
        fs::write(
            root.join(name),
            kiln_assets::encode_png_rgba(2, 2, &color.repeat(4)).unwrap(),
        )
        .unwrap();
    }
    let mut project = Project::default();
    let recipe = AtlasRecipe {
        output: "atlas.png".into(),
        size: 16,
        padding: 1,
        sources: [("hero".into(), "hero.png".into())].into(),
        ..Default::default()
    };
    kiln_assets::atlas_project::build(&mut project, root, "game", &recipe).unwrap();
    project_files::save(&project, &root.join("kiln.toml")).unwrap();
    project
}

#[test]
fn reopening_repairs_interrupted_atlas_without_rewriting_manifest_or_source_art() {
    let root = tempfile::tempdir().unwrap();
    let saved = fixture(root.path());
    let manifest = fs::read(root.path().join("kiln.toml")).unwrap();
    let original_png = fs::read(root.path().join("atlas.png")).unwrap();
    let original_art = fs::read(root.path().join("hero.png")).unwrap();
    let mut unsaved = saved.clone();
    let mut recipe = saved.atlases["game"].clone();
    recipe.sources.insert("aaa".into(), "coin.png".into());
    kiln_assets::atlas_project::build(&mut unsaved, root.path(), "game", &recipe).unwrap();
    assert_ne!(
        fs::read(root.path().join("atlas.png")).unwrap(),
        original_png
    );
    let loaded = project_files::load_recovering(root.path()).unwrap();
    assert_eq!(
        fs::read(root.path().join("atlas.png")).unwrap(),
        original_png,
        "Reopening left a newer generated image inconsistent with the saved recipe/clip regions"
    );
    assert_eq!(
        serde_json::to_value(loaded.project).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    assert!(loaded
        .messages
        .iter()
        .any(|m| m.starts_with("Reconstructed")));
    assert_eq!(fs::read(root.path().join("kiln.toml")).unwrap(), manifest);
    assert_eq!(
        fs::read(root.path().join("hero.png")).unwrap(),
        original_art
    );
}

#[test]
fn save_as_preserves_pending_generation_and_leaves_source_files_and_provenance_unchanged() {
    let parent = tempfile::tempdir().unwrap();
    let source = parent.path().join("source");
    fs::create_dir(&source).unwrap();
    let saved = fixture(&source);
    let manifest = source.join("kiln.toml");
    let saved_manifest = fs::read(&manifest).unwrap();
    let mut draft = saved.clone();
    let mut recipe = saved.atlases["game"].clone();
    recipe.size = 32;
    kiln_assets::atlas_project::build(&mut draft, &source, "game", &recipe).unwrap();
    let png = fs::read(source.join("atlas.png")).unwrap();
    let provenance = fs::read(source.join(".kiln-atlas-generations.json")).unwrap();
    fs::write(source.join(".kiln-atlas-orphan.tmp"), b"retained").unwrap();
    let target = parent.path().join("copy/kiln.toml");
    project_files::save_as(&draft, &manifest, &target).unwrap();
    let loaded = project_files::load(&target).unwrap();
    assert_eq!(
        serde_json::to_value(loaded.project).unwrap(),
        serde_json::to_value(draft).unwrap()
    );
    assert_eq!(
        fs::read(target.parent().unwrap().join("atlas.png")).unwrap(),
        png
    );
    assert!(!target
        .parent()
        .unwrap()
        .join(".kiln-atlas-generations.json")
        .exists());
    assert!(!target
        .parent()
        .unwrap()
        .join(".kiln-atlas-orphan.tmp")
        .exists());
    assert_eq!(fs::read(&manifest).unwrap(), saved_manifest);
    assert_eq!(fs::read(source.join("atlas.png")).unwrap(), png);
    assert_eq!(
        fs::read(source.join(".kiln-atlas-generations.json")).unwrap(),
        provenance
    );
    assert_eq!(
        fs::read(source.join(".kiln-atlas-orphan.tmp")).unwrap(),
        b"retained"
    );
}

#[test]
fn disk_conflict_check_does_not_roll_back_a_pending_atlas_generation() {
    let root = tempfile::tempdir().unwrap();
    let saved = fixture(root.path());
    let mut lifecycle = kiln_editor::project_lifecycle::ProjectLifecycle::default();
    lifecycle.mark_saved(&saved);
    let mut draft = saved.clone();
    let mut recipe = saved.atlases["game"].clone();
    recipe.sources.insert("aaa".into(), "coin.png".into());
    kiln_assets::atlas_project::build(&mut draft, root.path(), "game", &recipe).unwrap();
    let png = fs::read(root.path().join("atlas.png")).unwrap();
    lifecycle
        .require_unchanged_disk(&root.path().join("kiln.toml"))
        .unwrap();
    assert_eq!(fs::read(root.path().join("atlas.png")).unwrap(), png);
    project_files::save(&draft, &root.path().join("kiln.toml")).unwrap();
    assert!(!root.path().join(".kiln-atlas-generations.json").exists());
}
