//! Save slot management with versioned data.

use crate::version::{MigrationError, SaveVersion, VersionMigration};
use dirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, io::Write, path::Path};
use thiserror::Error;

/// Bound slot files before allocating/parsing untrusted data.
pub const MAX_SLOT_BYTES: u64 = 16 * 1024 * 1024;

/// Flush a temporary file before atomically replacing its destination.
/// Failures retain the previous file and remove the temporary file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Missing parent directory"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".kiln-save-{}-{}.tmp",
        std::process::id(),
        kiln_core::next_id()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Save slot metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlotMetadata {
    pub slot_id: u32,
    pub version: SaveVersion,
    pub timestamp: i64,
    pub description: String,
}

/// Save slot with metadata and data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    pub metadata: SaveSlotMetadata,
    pub data: String,
}

/// Save slot manager for loading and saving game data.
pub struct SaveSlotManager {
    save_dir: PathBuf,
    migrations: Vec<Box<dyn VersionMigration>>,
}

impl SaveSlotManager {
    /// Create a new save slot manager.
    pub fn new(game_id: &str) -> Result<Self, SaveSlotError> {
        let save_dir = dirs::data_local_dir()
            .ok_or(SaveSlotError::NoDataDir)?
            .join("kiln")
            .join(game_id)
            .join("saves");

        fs::create_dir_all(&save_dir).map_err(SaveSlotError::Io)?;

        Ok(Self {
            save_dir,
            migrations: Vec::new(),
        })
    }

    /// Create a new save slot manager with a custom directory.
    ///
    /// The directory is created if it does not exist yet, so a save always
    /// has somewhere to go.
    pub fn with_directory(save_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&save_dir);
        Self {
            save_dir,
            migrations: Vec::new(),
        }
    }

    /// Register a version migration.
    pub fn register_migration(&mut self, migration: Box<dyn VersionMigration>) {
        self.migrations.push(migration);
    }

    /// Save data to a slot.
    pub fn save(&self, slot_id: u32, data: &str, description: &str) -> Result<(), SaveSlotError> {
        if self.slot_path(slot_id).exists() {
            self.load(slot_id)?;
        }
        let metadata = SaveSlotMetadata {
            slot_id,
            version: SaveVersion::current(),
            timestamp: chrono::Utc::now().timestamp(),
            description: description.to_string(),
        };

        let slot = SaveSlot {
            metadata,
            data: data.to_string(),
        };

        let slot_path = self.slot_path(slot_id);
        let json = serde_json::to_string_pretty(&slot).map_err(SaveSlotError::Serialization)?;
        if json.len() as u64 > MAX_SLOT_BYTES {
            return Err(SaveSlotError::Invalid("Slot exceeds 16 MiB".into()));
        }
        atomic_write(&slot_path, json.as_bytes())?;

        Ok(())
    }

    /// Load data from a slot with automatic migration.
    pub fn load(&self, slot_id: u32) -> Result<SaveSlot, SaveSlotError> {
        let slot_path = self.slot_path(slot_id);
        if fs::metadata(&slot_path)?.len() > MAX_SLOT_BYTES {
            return Err(SaveSlotError::Invalid("Slot exceeds 16 MiB".into()));
        }
        let json = fs::read_to_string(&slot_path).map_err(SaveSlotError::Io)?;
        let mut slot: SaveSlot =
            serde_json::from_str(&json).map_err(SaveSlotError::Serialization)?;

        // Apply migrations if needed
        let current_version = SaveVersion::current();
        if slot.metadata.slot_id != slot_id || slot.metadata.version > current_version {
            return Err(SaveSlotError::Invalid(
                "Slot identity or version is incompatible".into(),
            ));
        }
        while slot.metadata.version < current_version {
            let migrated = self.apply_migration(&slot)?;
            if migrated.metadata.version <= slot.metadata.version
                || migrated.metadata.version > current_version
            {
                return Err(SaveSlotError::Invalid(
                    "Migration must advance towards the current version".into(),
                ));
            }
            slot = migrated;
        }

        Ok(slot)
    }

    /// List all save slots.
    pub fn list_slots(&self) -> Result<Vec<SaveSlotMetadata>, SaveSlotError> {
        let mut slots = Vec::new();

        for entry in fs::read_dir(&self.save_dir).map_err(SaveSlotError::Io)? {
            let entry = entry.map_err(SaveSlotError::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = fs::read_to_string(&path).map_err(SaveSlotError::Io)?;
                let slot: SaveSlot =
                    serde_json::from_str(&json).map_err(SaveSlotError::Serialization)?;
                slots.push(slot.metadata);
            }
        }

        slots.sort_by_key(|m| m.timestamp);
        Ok(slots)
    }

    /// Delete a save slot.
    pub fn delete(&self, slot_id: u32) -> Result<(), SaveSlotError> {
        let slot_path = self.slot_path(slot_id);
        fs::remove_file(&slot_path).map_err(SaveSlotError::Io)?;
        Ok(())
    }

    fn slot_path(&self, slot_id: u32) -> PathBuf {
        self.save_dir.join(format!("slot_{}.json", slot_id))
    }

    fn apply_migration(&self, slot: &SaveSlot) -> Result<SaveSlot, SaveSlotError> {
        for migration in &self.migrations {
            if migration.source_version() == slot.metadata.version {
                let migrated_data = migration.migrate(&slot.data)?;
                let mut new_slot = slot.clone();
                new_slot.data = migrated_data;
                new_slot.metadata.version = migration.to_version();
                return Ok(new_slot);
            }
        }
        Err(SaveSlotError::NoMigrationFound(slot.metadata.version))
    }
}

/// Save slot error types.
#[derive(Error, Debug)]
pub enum SaveSlotError {
    #[error("Invalid save slot: {0}")]
    Invalid(String),
    #[error("No data directory found")]
    NoDataDir,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("No migration found for version {0}")]
    NoMigrationFound(SaveVersion),
    #[error("Slot not found: {0}")]
    SlotNotFound(u32),
    #[error("Migration error: {0}")]
    Migration(#[from] MigrationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::MigrationError;

    struct TestMigration;

    impl VersionMigration for TestMigration {
        fn source_version(&self) -> SaveVersion {
            SaveVersion::new(1, 0, 0)
        }

        fn to_version(&self) -> SaveVersion {
            SaveVersion::new(1, 1, 0)
        }

        fn migrate(&self, data: &str) -> Result<String, MigrationError> {
            Ok(format!("MIGRATED: {}", data))
        }
    }

    #[test]
    fn corrupt_future_and_oversized_replacements_preserve_the_previous_slot() {
        let dir = std::env::temp_dir().join(format!(
            "kiln-atomic-{}-{}",
            std::process::id(),
            kiln_core::next_id()
        ));
        let manager = SaveSlotManager::with_directory(dir.clone());
        manager.save(0, "progress", "before").unwrap();
        let path = dir.join("slot_0.json");
        let before = fs::read(&path).unwrap();
        assert!(manager
            .save(0, &"x".repeat(MAX_SLOT_BYTES as usize), "too big")
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        let mut future = manager.load(0).unwrap();
        future.metadata.version = SaveVersion::new(999, 0, 0);
        let future = serde_json::to_vec(&future).unwrap();
        fs::write(&path, &future).unwrap();
        assert!(manager.load(0).is_err());
        assert!(manager.save(0, "new", "after").is_err());
        assert_eq!(fs::read(&path).unwrap(), future);
        fs::write(&path, "broken").unwrap();
        assert!(manager.save(0, "new", "after").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken");
        // Rename failure cleans its temporary file without touching the destination.
        let blocked = dir.join("directory");
        fs::create_dir(&blocked).unwrap();
        assert!(atomic_write(&blocked, b"data").is_err());
        assert!(blocked.is_dir());
        assert!(!fs::read_dir(&dir).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_save_slot_metadata() {
        let metadata = SaveSlotMetadata {
            slot_id: 0,
            version: SaveVersion::new(1, 0, 0),
            timestamp: 0,
            description: "Test".to_string(),
        };
        assert_eq!(metadata.slot_id, 0);
    }

    #[test]
    fn test_save_slot_manager() {
        let _game_id = "test_game";
        let temp_dir = std::env::temp_dir().join("kiln_test_saves");
        let _ = fs::create_dir_all(&temp_dir);

        // Note: This test uses a custom save dir logic
        // In real usage, SaveSlotManager::new() uses dirs crate
    }

    #[test]
    fn test_save_load_slot() {
        let temp_dir = std::env::temp_dir().join("kiln_test_saves_2");
        let _ = fs::create_dir_all(&temp_dir);

        let manager = SaveSlotManager::with_directory(temp_dir.clone());

        let data = r#"{"player": {"x": 100, "y": 200}}"#;
        manager.save(0, data, "Test save").unwrap();

        let loaded = manager.load(0).unwrap();
        assert_eq!(loaded.data, data);
        assert_eq!(loaded.metadata.slot_id, 0);

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_save_migration_v1_to_v2() {
        let temp_dir = std::env::temp_dir().join("kiln_test_saves_3");
        let _ = fs::create_dir_all(&temp_dir);

        let mut manager = SaveSlotManager::with_directory(temp_dir.clone());

        // Register migration
        manager.register_migration(Box::new(TestMigration));

        // Simulate v1 save data
        let v1_data = r#"{"player": {"x": 100, "y": 200}}"#;

        // Save with v1 version (simulated by manually setting version)
        let metadata = SaveSlotMetadata {
            slot_id: 0,
            version: SaveVersion::new(1, 0, 0),
            timestamp: chrono::Utc::now().timestamp(),
            description: "V1 save".to_string(),
        };
        let slot = SaveSlot {
            metadata,
            data: v1_data.to_string(),
        };
        let slot_path = temp_dir.join("slot_0.json");
        let json = serde_json::to_string_pretty(&slot).unwrap();
        fs::write(&slot_path, json).unwrap();

        // Load should trigger migration to current version (1.1.0)
        let loaded = manager.load(0).unwrap();
        assert!(loaded.data.contains("MIGRATED"));

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }
}
