//! Save version and migration system.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Save data version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SaveVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SaveVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn current() -> Self {
        Self::new(1, 1, 0)
    }
}

impl fmt::Display for SaveVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Version migration trait for upgrading save data.
pub trait VersionMigration: Send + Sync {
    /// The source version this migration applies to.
    fn from_version(&self) -> SaveVersion;

    /// The target version after migration.
    fn to_version(&self) -> SaveVersion;

    /// Migrate save data from source to target version.
    fn migrate(&self, data: &str) -> Result<String, MigrationError>;
}

/// Migration error types.
#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid data format for version {0}")]
    InvalidFormat(SaveVersion),
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_new() {
        let version = SaveVersion::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_display() {
        let version = SaveVersion::new(1, 0, 0);
        assert_eq!(version.to_string(), "1.0.0");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = SaveVersion::new(1, 0, 0);
        let v2 = SaveVersion::new(1, 1, 0);
        assert!(v2 > v1);
    }
}
