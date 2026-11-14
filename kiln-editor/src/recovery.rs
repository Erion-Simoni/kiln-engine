//! Crash-recovery snapshots for unsaved project documents.

use crate::script_editor::ScriptDraft;
use kiln_core::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Versioned editor sidecar, separate from the saved project format.
#[derive(Serialize, Deserialize)]
pub(crate) struct RecoverySnapshot {
    recovery_version: u32,
    pub project: Project,
    pub script: Option<ScriptDraft>,
}

/// Return the sidecar recovery file for a project manifest.
pub fn path_for(project_path: &Path) -> PathBuf {
    let name = project_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    project_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{name}.recovery.json"))
}

/// Save a project-only recovery snapshot (legacy caller compatibility).
pub fn save(project_path: &Path, project: &Project) -> Result<PathBuf, String> {
    save_with_script(project_path, project, None)
}

/// Read the recovered project document for legacy callers.
pub fn load_if_newer(project_path: &Path) -> Result<Option<(PathBuf, Project)>, String> {
    let recovery_path = path_for(project_path);
    if !recovery_path.exists()
        || !project_path.exists()
        || !newer_than_saved(&recovery_path, project_path)?
    {
        return Ok(None);
    }
    load_snapshot_if_newer(project_path)
        .map(|snapshot| snapshot.map(|(path, snapshot)| (path, snapshot.project)))
}

/// Atomically write the current in-memory project as a recovery snapshot.
pub(crate) fn save_with_script(
    project_path: &Path,
    project: &Project,
    script: Option<ScriptDraft>,
) -> Result<PathBuf, String> {
    let recovery_path = path_for(project_path);
    let bytes = serde_json::to_vec(&RecoverySnapshot {
        recovery_version: 1,
        project: project.clone(),
        script,
    })
    .map_err(|error| error.to_string())?;
    let mut file = tempfile::Builder::new()
        .prefix(".kiln-")
        .suffix(".recovery.tmp")
        .tempfile_in(recovery_path.parent().ok_or("Missing recovery directory")?)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    file.persist(&recovery_path)
        .map_err(|error| error.to_string())?;
    Ok(recovery_path)
}

/// Recover newer scenes, or script drafts alongside a more recently saved project.
pub(crate) fn load_snapshot_if_newer(
    project_path: &Path,
) -> Result<Option<(PathBuf, RecoverySnapshot)>, String> {
    let recovery_path = path_for(project_path);
    if !recovery_path.exists() || !project_path.exists() {
        return Ok(None);
    }
    let newer = newer_than_saved(&recovery_path, project_path)?;
    let bytes = fs::read(&recovery_path).map_err(|error| error.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut snapshot = if let Some(version) = value.get("recovery_version") {
        if version.as_u64() != Some(1) {
            return Err("Unsupported recovery snapshot version".into());
        }
        serde_json::from_value(value).map_err(|e| e.to_string())?
    } else {
        RecoverySnapshot {
            recovery_version: 1,
            project: serde_json::from_value(value).map_err(|e| e.to_string())?,
            script: None,
        }
    };
    if !newer {
        if snapshot.script.is_none() {
            return Ok(None);
        }
        snapshot.project = crate::project_files::load(project_path)?.project;
    }
    Ok(Some((recovery_path, snapshot)))
}

fn newer_than_saved(recovery_path: &Path, project_path: &Path) -> Result<bool, String> {
    let project_time = project_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    let recovery_time = recovery_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    Ok(is_newer(recovery_time, project_time))
}

/// Remove a recovery sidecar after successful save or explicit discard.
pub fn clear(path: &Path) -> Result<(), String> {
    let recovery_path = path_for(path);
    if !recovery_path.exists() {
        return Ok(());
    }
    match fs::remove_file(recovery_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Compare file modification times, exposed for filesystem-independent tests.
pub fn is_newer(recovery: SystemTime, project: SystemTime) -> bool {
    recovery > project
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script_editor::ScriptEditor;

    #[test]
    fn recovered_source_preserves_external_edits_and_failed_save_drafts() {
        let root = tempfile::tempdir().unwrap();
        let manifest = root.path().join("kiln.toml");
        let project = Project::default();
        crate::project_files::save(&project, &manifest).unwrap();
        fs::create_dir(root.path().join("scripts")).unwrap();
        let source = root.path().join("scripts/main.ks");
        fs::write(&source, "let value = 1;").unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), "scripts/main.ks").unwrap();
        editor.source = "let value = 2;".into();
        save_with_script(&manifest, &project, editor.recovery_draft()).unwrap();
        fs::write(&source, "let value = 3;").unwrap();
        let (_, snapshot) = load_snapshot_if_newer(&manifest).unwrap().unwrap();
        let mut restored =
            ScriptEditor::from_recovery(root.path(), snapshot.script.as_ref().unwrap()).unwrap();
        assert_eq!(restored.source, "let value = 2;");
        assert!(restored.is_dirty());
        assert!(restored.save().unwrap_err().contains("outside the editor"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "let value = 3;");
        assert_eq!(restored.source, "let value = 2;");
        assert!(path_for(&manifest).exists());
    }

    #[test]
    fn script_draft_survives_a_newer_manifest_without_reverting_saved_scenes() {
        let root = tempfile::tempdir().unwrap();
        let manifest = root.path().join("kiln.toml");
        let project = Project::default();
        crate::project_files::save(&project, &manifest).unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), "new.ks").unwrap();
        editor.source = "let unfinished =".into();
        save_with_script(&manifest, &project, editor.recovery_draft()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let newer = Project {
            name: "Newer saved scene document".into(),
            ..project
        };
        crate::project_files::save(&newer, &manifest).unwrap();
        assert!(load_if_newer(&manifest).unwrap().is_none());
        let (_, snapshot) = load_snapshot_if_newer(&manifest).unwrap().unwrap();
        assert_eq!(snapshot.project.name, newer.name);
        let mut restored =
            ScriptEditor::from_recovery(root.path(), snapshot.script.as_ref().unwrap()).unwrap();
        assert_eq!(restored.source, "let unfinished =");
        assert!(restored.save().is_err());
        assert!(restored.is_dirty());
        assert!(!root.path().join("new.ks").exists());
    }

    #[test]
    fn legacy_snapshots_load_and_unknown_versions_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let manifest = root.path().join("kiln.toml");
        fs::write(&manifest, "saved").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let sidecar = path_for(&manifest);
        fs::write(&sidecar, serde_json::to_vec(&Project::default()).unwrap()).unwrap();
        assert!(load_snapshot_if_newer(&manifest)
            .unwrap()
            .unwrap()
            .1
            .script
            .is_none());
        let original = br#"{"recovery_version":999,"project":{},"script":null}"#;
        fs::write(&sidecar, original).unwrap();
        assert!(load_snapshot_if_newer(&manifest).is_err());
        assert_eq!(fs::read(&sidecar).unwrap(), original);
    }

    #[test]
    fn recovery_sidecar_round_trips_and_clears() {
        let directory = tempfile::tempdir().unwrap();
        let project_path = directory.path().join("game.kiln.toml");
        fs::write(&project_path, "saved project").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let project = Project {
            name: "Recovered project".into(),
            ..Project::default()
        };
        let path = save_with_script(&project_path, &project, None).unwrap();
        assert!(path.exists());
        let (_, recovered) = load_snapshot_if_newer(&project_path).unwrap().unwrap();
        assert_eq!(recovered.project.name, project.name);
        clear(&project_path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn recovery_must_be_newer_than_the_project_save() {
        let earlier = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
        let later = earlier + std::time::Duration::from_secs(1);
        assert!(is_newer(later, earlier));
        assert!(!is_newer(earlier, later));
        assert!(!is_newer(earlier, earlier));
    }
}
