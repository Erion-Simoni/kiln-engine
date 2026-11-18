//! Persisted identities and orphaned import presets reserve their ID namespace.
use kiln_core::Project;

#[test]
fn importing_new_assets_after_load_skips_persisted_ids_and_presets() {
    let mut project = Project::default();
    let counter = kiln_core::next_id();
    let saved = (counter + 1).to_string();
    let orphaned = (counter + 2).to_string();
    let platform_only = (counter + 3).to_string();
    project.asset_ids.insert("old.png".into(), saved.clone());
    project
        .asset_import_settings
        .insert(orphaned.clone(), Default::default());
    project
        .asset_platform_import_settings
        .insert(platform_only.clone(), Default::default());
    let mut loaded = Project::from_toml(&project.to_toml().unwrap()).unwrap();
    let ids = loaded.ensure_asset_ids(["new.png", "next.png", "old.png", "new.png"]);
    assert!(
        ![&saved, &orphaned, &platform_only].contains(&&ids[0]),
        "Fresh allocation reused a persisted identity/preset"
    );
    assert_ne!(ids[0], ids[1]);
    assert_eq!(ids[0], ids[3]);
    assert_eq!(ids[2], saved);
    assert_eq!(loaded.asset_import_settings, project.asset_import_settings);
    assert_eq!(
        loaded.asset_platform_import_settings,
        project.asset_platform_import_settings
    );
    assert_eq!(loaded.asset_ids.len(), 3);
}
