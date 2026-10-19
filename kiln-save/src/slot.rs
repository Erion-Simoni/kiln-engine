//! Save slot management with versioned data.

use crate::version::{SaveVersion, VersionMigration, MigrationError};
use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

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

    /// Create a new save slot manager with a custom directory (for testing).
    pub fn with_directory(save_dir: PathBuf) -> Self {
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
        fs::write(&slot_path, json).map_err(SaveSlotError::Io)?;

        Ok(())
    }

    /// Load data from a slot with automatic migration.
    pub fn load(&self, slot_id: u32) -> Result<SaveSlot, SaveSlotError> {
        let slot_path = self.slot_path(slot_id);
        let json = fs::read_to_string(&slot_path).map_err(SaveSlotError::Io)?;
        let mut slot: SaveSlot = serde_json::from_str(&json).map_err(SaveSlotError::Serialization)?;

        // Apply migrations if needed
        let current_version = SaveVersion::current();
        while slot.metadata.version < current_version {
            let migrated = self.apply_migration(&slot)?;
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
                let slot: SaveSlot = serde_json::from_str(&json).map_err(SaveSlotError::Serialization)?;
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
            if migration.from_version() == slot.metadata.version {
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
        fn from_version(&self) -> SaveVersion {
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
