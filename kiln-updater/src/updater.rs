//! In-game updater: check, download, verify, apply, rollback.

use crate::patch::PatchError;
use crate::version::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

#[cfg(feature = "updater")]
use crate::patch::{apply_patch, hash_bytes};

/// Error type for updater operations.
#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error("Failed to check for updates: {0}")]
    CheckFailed(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("Patch error: {0}")]
    Patch(#[from] PatchError),
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
    #[error("Updater not available (feature disabled)")]
    NotAvailable,
}

/// Update manifest from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub patch_url: String,
    pub patch_hash: String,
    pub patch_size: u64,
    pub release_notes: String,
}

/// Game updater for in-game updates.
pub struct Updater {
    #[allow(dead_code)]
    update_url: String,
    #[allow(dead_code)]
    current_version: Version,
    game_path: String,
}

impl Updater {
    /// Create a new updater instance.
    pub fn new(update_url: &str, current_version: &str) -> Result<Self, UpdaterError> {
        let version = Version::from_str(current_version)
            .map_err(|e| UpdaterError::CheckFailed(e.to_string()))?;

        Ok(Self {
            update_url: update_url.to_string(),
            current_version: version,
            game_path: ".".to_string(), // Will be set by caller
        })
    }

    /// Set the game path (where archives are stored).
    pub fn set_game_path(&mut self, path: &str) {
        self.game_path = path.to_string();
    }

    /// Check if an update is available.
    #[cfg(feature = "updater")]
    pub async fn check_update(&self) -> Result<bool, UpdaterError> {
        let manifest_url = format!("{}/manifest.json", self.update_url);
        let response = reqwest::get(&manifest_url)
            .await
            .map_err(|e| UpdaterError::CheckFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdaterError::CheckFailed(
                "Failed to fetch manifest".to_string(),
            ));
        }

        let manifest: UpdateManifest = response
            .json()
            .await
            .map_err(|e| UpdaterError::CheckFailed(e.to_string()))?;

        let latest_version = Version::from_str(&manifest.version)
            .map_err(|e| UpdaterError::CheckFailed(e.to_string()))?;

        Ok(latest_version > self.current_version)
    }

    /// Check if an update is available (stub when feature disabled).
    #[cfg(not(feature = "updater"))]
    pub async fn check_update(&self) -> Result<bool, UpdaterError> {
        Err(UpdaterError::NotAvailable)
    }

    /// Download and apply an update.
    #[cfg(feature = "updater")]
    pub async fn apply_update(&self) -> Result<(), UpdaterError> {
        let manifest_url = format!("{}/manifest.json", self.update_url);
        let response = reqwest::get(&manifest_url)
            .await
            .map_err(|e| UpdaterError::DownloadFailed(e.to_string()))?;

        let manifest: UpdateManifest = response
            .json()
            .await
            .map_err(|e| UpdaterError::DownloadFailed(e.to_string()))?;

        // Download patch
        let patch_response = reqwest::get(&manifest.patch_url)
            .await
            .map_err(|e| UpdaterError::DownloadFailed(e.to_string()))?;

        let patch_data = patch_response
            .bytes()
            .await
            .map_err(|e| UpdaterError::DownloadFailed(e.to_string()))?;

        // Verify patch hash
        let actual_hash = crate::patch::hash_bytes(&patch_data);
        if actual_hash != manifest.patch_hash {
            return Err(UpdaterError::DownloadFailed("Hash mismatch".to_string()));
        }

        // Backup current archive
        let archive_path = Path::new(&self.game_path).join("game.kilnar");
        let backup_path = Path::new(&self.game_path).join("game.kilnar.backup");

        if archive_path.exists() {
            fs::copy(&archive_path, &backup_path)
                .map_err(|e| UpdaterError::RollbackFailed(e.to_string()))?;
        }

        // Apply patch
        let old_data = fs::read(&archive_path)
            .map_err(|e| UpdaterError::Patch(PatchError::ReadOld(e)))?;

        let new_data = apply_patch(&old_data, &patch_data, &manifest.patch_hash)?;

        // Write new archive
        fs::write(&archive_path, new_data)
            .map_err(|e| UpdaterError::Patch(PatchError::Generate(e.to_string())))?;

        // Remove backup on success
        if backup_path.exists() {
            fs::remove_file(&backup_path)
                .map_err(|e| UpdaterError::RollbackFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// Download and apply an update (stub when feature disabled).
    #[cfg(not(feature = "updater"))]
    pub async fn apply_update(&self) -> Result<(), UpdaterError> {
        Err(UpdaterError::NotAvailable)
    }

    /// Rollback to the previous version.
    pub fn rollback(&self) -> Result<(), UpdaterError> {
        let archive_path = Path::new(&self.game_path).join("game.kilnar");
        let backup_path = Path::new(&self.game_path).join("game.kilnar.backup");

        if !backup_path.exists() {
            return Err(UpdaterError::RollbackFailed("No backup found".to_string()));
        }

        fs::copy(&backup_path, &archive_path)
            .map_err(|e| UpdaterError::RollbackFailed(e.to_string()))?;

        fs::remove_file(&backup_path)
            .map_err(|e| UpdaterError::RollbackFailed(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_creation() {
        let updater = Updater::new("https://example.com", "1.0.0").unwrap();
        assert_eq!(updater.current_version, Version::new(1, 0, 0));
    }

    #[test]
    fn rollback_without_backup() {
        let mut updater = Updater::new("https://example.com", "1.0.0").unwrap();
        updater.set_game_path("/tmp/test");
        assert!(updater.rollback().is_err());
    }
}
