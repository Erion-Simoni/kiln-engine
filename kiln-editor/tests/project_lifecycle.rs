//! Real disk-backed save/discard/cancel regressions for destructive transitions.
use kiln_core::{Project, Scene};
use kiln_editor::{
    project_files,
    project_lifecycle::{ProjectAction, ProjectLifecycle, SaveDecision},
};

fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Project,
    ProjectLifecycle,
) {
    let directory = tempfile::tempdir().unwrap();
    let loaded =
        project_files::create("Original", &directory.path().join("original"), "empty_2d").unwrap();
    let mut guard = ProjectLifecycle::default();
    guard.mark_saved(&loaded.project);
    (directory, loaded.path, loaded.project, guard)
}
fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn clean_project_or_no_project_can_exit_without_a_prompt() {
    let (_directory, _, project, mut guard) = fixture();
    assert_eq!(
        guard.request(ProjectAction::Exit, Some(&project)).unwrap(),
        Some(ProjectAction::Exit)
    );
    assert!(guard.pending().is_none());
    assert_eq!(
        guard.request(ProjectAction::Exit, None).unwrap(),
        Some(ProjectAction::Exit)
    );
}

#[test]
fn metadata_resources_and_removed_or_inactive_scenes_are_unsaved_changes() {
    let (_directory, _, project, guard) = fixture();
    let mut edited = project.clone();
    edited.window.width += 32;
    assert!(guard.is_dirty(Some(&edited)).unwrap());
    edited = project.clone();
    edited.name = "Changed project name".into();
    assert!(guard.is_dirty(Some(&edited)).unwrap());
    edited = project.clone();
    edited.icon = "art/new-icon.png".into();
    assert!(guard.is_dirty(Some(&edited)).unwrap());
    edited = project.clone();
    edited.scenes.clear();
    assert!(guard.is_dirty(Some(&edited)).unwrap());
    edited = project.clone();
    edited.scenes.insert(
        "inactive".into(),
        Scene {
            id: "inactive".into(),
            name: "Inactive level".into(),
            script: None,
            entities: vec![],
            nested_scenes: vec![],
        },
    );
    assert!(guard.is_dirty(Some(&edited)).unwrap());
    assert!(!guard.is_dirty(Some(&project)).unwrap());
}

#[test]
fn cancel_retains_unsaved_document_and_original_disk_contents() {
    let (_directory, path, mut project, mut guard) = fixture();
    let original = std::fs::read(&path).unwrap();
    project.window.width = 345;
    let edited = json(&project);
    assert!(guard
        .request(ProjectAction::Exit, Some(&project))
        .unwrap()
        .is_none());
    assert_eq!(guard.resolve(SaveDecision::Cancel, Ok(())).unwrap(), None);
    assert!(guard.pending().is_none());
    assert_eq!(json(&project), edited);
    assert!(guard.is_dirty(Some(&project)).unwrap());
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn discard_releases_the_exact_action_without_writing_or_marking_saved() {
    let (directory, path, mut project, mut guard) = fixture();
    let original = std::fs::read(&path).unwrap();
    project.name = "Unsaved".into();
    let action = ProjectAction::Open(directory.path().join("other/kiln.toml"));
    guard.request(action.clone(), Some(&project)).unwrap();
    assert_eq!(
        guard.resolve(SaveDecision::Discard, Ok(())).unwrap(),
        Some(action)
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert!(guard.is_dirty(Some(&project)).unwrap());
}

#[test]
fn failed_save_keeps_pending_action_and_supports_retry_before_exit() {
    let (directory, path, mut project, mut guard) = fixture();
    let original = std::fs::read(&path).unwrap();
    project.window.height = 333;
    guard.request(ProjectAction::Exit, Some(&project)).unwrap();
    // A directory destination fails even when test runners have elevated permissions.
    let failure = project_files::save(&project, directory.path()).map(|_| ());
    assert!(guard.resolve(SaveDecision::Save, failure).is_err());
    assert_eq!(guard.pending(), Some(&ProjectAction::Exit));
    assert!(guard.save_error().is_some());
    assert!(guard.is_dirty(Some(&project)).unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let saved = project_files::save(&project, &path).map(|_| ());
    assert!(saved.is_ok());
    guard.mark_saved(&project);
    assert_eq!(
        guard.resolve(SaveDecision::Save, saved).unwrap(),
        Some(ProjectAction::Exit)
    );
    assert!(guard.pending().is_none());
    assert!(guard.save_error().is_none());
    assert!(!guard.is_dirty(Some(&project)).unwrap());
    assert_eq!(
        project_files::load(&path).unwrap().project.window.height,
        333
    );
}

#[test]
fn repeated_requests_cannot_replace_the_original_pending_transition() {
    let (directory, _, mut project, mut guard) = fixture();
    project.name = "Unsaved".into();
    let open = ProjectAction::Open(directory.path().join("next/kiln.toml"));
    guard.request(open.clone(), Some(&project)).unwrap();
    guard.request(ProjectAction::Exit, Some(&project)).unwrap();
    assert_eq!(guard.pending(), Some(&open));
    assert_eq!(
        guard.resolve(SaveDecision::Discard, Ok(())).unwrap(),
        Some(open)
    );
    assert!(guard
        .resolve(SaveDecision::Save, Err("stale button".into()))
        .unwrap()
        .is_none());
}

#[test]
fn save_before_open_writes_the_old_document_then_loads_the_new_baseline() {
    let (directory, path, mut project, mut guard) = fixture();
    let other = project_files::create("Next", &directory.path().join("next"), "empty_2d").unwrap();
    project.scenes.get_mut("main").unwrap().name = "Unsaved scene name".into();
    guard
        .request(ProjectAction::Open(other.path.clone()), Some(&project))
        .unwrap();
    let saved = project_files::save(&project, &path).map(|_| ());
    guard.mark_saved(&project);
    let Some(ProjectAction::Open(target)) = guard.resolve(SaveDecision::Save, saved).unwrap()
    else {
        panic!("expected open");
    };
    let loaded = project_files::load(&target).unwrap();
    guard.mark_saved(&loaded.project);
    assert!(!guard.is_dirty(Some(&loaded.project)).unwrap());
    assert_eq!(loaded.project.name, "Next");
    assert_eq!(
        project_files::load(&path).unwrap().project.scenes["main"].name,
        "Unsaved scene name"
    );
}

#[test]
fn canceling_creation_has_no_filesystem_side_effect_and_discard_allows_creation() {
    let (directory, path, mut project, mut guard) = fixture();
    let original = std::fs::read(&path).unwrap();
    project.name = "Unsaved".into();
    let destination = directory.path().join("new");
    let action = ProjectAction::Create {
        name: "New".into(),
        path: destination.clone(),
        template: "empty_2d".into(),
    };
    guard.request(action.clone(), Some(&project)).unwrap();
    assert!(!destination.exists());
    guard.resolve(SaveDecision::Cancel, Ok(())).unwrap();
    assert!(!destination.exists());
    guard.request(action, Some(&project)).unwrap();
    let Some(ProjectAction::Create {
        name,
        path: target,
        template,
    }) = guard.resolve(SaveDecision::Discard, Ok(())).unwrap()
    else {
        panic!("expected create");
    };
    let created = project_files::create(&name, &target, &template).unwrap();
    assert_eq!(created.project.name, "New");
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn failed_target_load_after_discard_preserves_the_outgoing_unsaved_baseline() {
    let (directory, path, mut project, mut guard) = fixture();
    project.window.width = 987;
    let edited = json(&project);
    let disk = std::fs::read(&path).unwrap();
    let target = directory.path().join("malformed.toml");
    std::fs::write(&target, "not valid TOML").unwrap();
    guard
        .request(ProjectAction::Open(target), Some(&project))
        .unwrap();
    let Some(ProjectAction::Open(target)) = guard.resolve(SaveDecision::Discard, Ok(())).unwrap()
    else {
        panic!("expected open");
    };
    assert!(project_files::load(&target).is_err());
    assert_eq!(json(&project), edited);
    assert!(guard.is_dirty(Some(&project)).unwrap());
    assert_eq!(std::fs::read(path).unwrap(), disk);
    assert!(guard
        .request(ProjectAction::Exit, Some(&project))
        .unwrap()
        .is_none());
}
