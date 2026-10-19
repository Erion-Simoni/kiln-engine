//! DLC pack support: additional archives with version dependencies.

use crate::version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

/// Error type for DLC operations.
#[derive(Debug, Error)]
pub enum DlcError {
    #[error("DLC not found: {0}")]
    NotFound(String),
    #[error("Version requirement not satisfied: {0} requires {1}")]
    VersionMismatch(String, String),
    #[error("Failed to load DLC metadata: {0}")]
    LoadError(String),
}

/// DLC metadata stored in the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlcMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub requires_version: String, // Minimum game version required
    pub dependencies: Vec<String>, // Other DLC IDs this depends on
}

/// DLC pack: archive + metadata.
#[derive(Debug, Clone)]
pub struct DlcPack {
    pub metadata: DlcMetadata,
    pub archive_path: String,
}

/// DLC manager for loading and managing multiple DLC packs.
pub struct DlcManager {
    packs: HashMap<String, DlcPack>,
}

impl DlcManager {
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
        }
    }

    /// Register a DLC pack.
    pub fn register(&mut self, pack: DlcPack) -> Result<(), DlcError> {
        let id = pack.metadata.id.clone();
        self.packs.insert(id, pack);
        Ok(())
    }

    /// Check if a DLC is compatible with the current game version.
    pub fn check_compatibility(&self, dlc_id: &str, game_version: &Version) -> Result<(), DlcError> {
        let pack = self
            .packs
            .get(dlc_id)
            .ok_or_else(|| DlcError::NotFound(dlc_id.to_string()))?;

        let _required_version = Version::from_str(&pack.metadata.requires_version)
            .map_err(|e| DlcError::LoadError(e.to_string()))?;

        if !game_version.satisfies(&pack.metadata.requires_version) {
            return Err(DlcError::VersionMismatch(
                dlc_id.to_string(),
                pack.metadata.requires_version.clone(),
            ));
        }

        // Check dependencies
        for dep_id in &pack.metadata.dependencies {
            if !self.packs.contains_key(dep_id) {
                return Err(DlcError::NotFound(dep_id.to_string()));
            }
        }

        Ok(())
    }

    /// Get all enabled DLC packs (all compatible packs).
    pub fn enabled_packs(&self, game_version: &Version) -> Vec<&DlcPack> {
        self.packs
            .values()
            .filter(|pack| {
                if let Ok(_v) = Version::from_str(&pack.metadata.requires_version) {
                    game_version.satisfies(&pack.metadata.requires_version)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get a DLC pack by ID.
    pub fn get(&self, id: &str) -> Option<&DlcPack> {
        self.packs.get(id)
    }
}

impl Default for DlcManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlc_registration() {
        let mut manager = DlcManager::new();
        let pack = DlcPack {
            metadata: DlcMetadata {
                id: "dlc1".to_string(),
                name: "Test DLC".to_string(),
                version: "1.0.0".to_string(),
                requires_version: "1.0.0".to_string(),
                dependencies: vec![],
            },
            archive_path: "test.kilnar".to_string(),
        };

        manager.register(pack).unwrap();
        assert!(manager.get("dlc1").is_some());
    }

    #[test]
    fn dlc_compatibility() {
        let mut manager = DlcManager::new();
        let pack = DlcPack {
            metadata: DlcMetadata {
                id: "dlc1".to_string(),
                name: "Test DLC".to_string(),
                version: "1.0.0".to_string(),
                requires_version: "1.0.0".to_string(),
                dependencies: vec![],
            },
            archive_path: "test.kilnar".to_string(),
        };

        manager.register(pack).unwrap();

        let game_version = Version::new(1, 2, 0);
        assert!(manager.check_compatibility("dlc1", &game_version).is_ok());

        let old_version = Version::new(0, 9, 0);
        assert!(manager.check_compatibility("dlc1", &old_version).is_err());
    }

    #[test]
    fn dlc_dependencies() {
        let mut manager = DlcManager::new();
        let pack1 = DlcPack {
            metadata: DlcMetadata {
                id: "dlc1".to_string(),
                name: "Base DLC".to_string(),
                version: "1.0.0".to_string(),
                requires_version: "1.0.0".to_string(),
                dependencies: vec![],
            },
            archive_path: "test1.kilnar".to_string(),
        };

        let pack2 = DlcPack {
            metadata: DlcMetadata {
                id: "dlc2".to_string(),
                name: "Dependent DLC".to_string(),
                version: "1.0.0".to_string(),
                requires_version: "1.0.0".to_string(),
                dependencies: vec!["dlc1".to_string()],
            },
            archive_path: "test2.kilnar".to_string(),
        };

        manager.register(pack1).unwrap();
        manager.register(pack2).unwrap();

        let game_version = Version::new(1, 0, 0);
        assert!(manager.check_compatibility("dlc2", &game_version).is_ok());
    }
}
