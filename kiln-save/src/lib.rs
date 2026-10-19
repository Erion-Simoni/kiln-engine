//! Kiln save system with versioned slots and migration.

pub mod slot;
pub mod version;

pub use slot::{SaveSlot, SaveSlotManager, SaveSlotMetadata};
pub use version::{SaveVersion, VersionMigration, MigrationError};
