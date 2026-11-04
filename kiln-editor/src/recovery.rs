//! Crash-recovery snapshots for unsaved project documents.

use kiln_core::Project;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

/// Atomically write the current in-memory project as a recovery snapshot.
pub fn save(project_path: &Path, project: &Project) -> Result<PathBuf, String> {
    let recovery_path = path_for(project_path);
    let temporary_path = recovery_path.with_extension("recovery.tmp");
    let bytes = serde_json::to_vec(project).map_err(|error| error.to_string())?;
    let mut file = File::create(&temporary_path).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temporary_path, &recovery_path).map_err(|error| error.to_string())?;
    Ok(recovery_path)
}

/// Read a recovery snapshot only when it is newer than the saved project file.
pub fn load_if_newer(project_path: &Path) -> Result<Option<(PathBuf, Project)>, String> {
    let recovery_path = path_for(project_path);
    if !recovery_path.exists() || !project_path.exists() {
        return Ok(None);
    }
    let project_time = project_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    let recovery_time = recovery_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    if !is_newer(recovery_time, project_time) {
        return Ok(None);
    }
    let bytes = fs::read(&recovery_path).map_err(|error| error.to_string())?;
    let project = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(Some((recovery_path, project)))
}

/// Remove a recovery sidecar after save, explicit discard, or successful restore.
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
        let path = save(&project_path, &project).unwrap();
        assert!(path.exists());
        let (_, recovered) = load_if_newer(&project_path).unwrap().unwrap();
        assert_eq!(recovered.name, project.name);
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
