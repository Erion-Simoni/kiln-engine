//! Semantic versioning for game versions and DLC compatibility.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Semantic version (major.minor.patch).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Check if this version satisfies a requirement (e.g., ">=1.0.0").
    pub fn satisfies(&self, requirement: &str) -> bool {
        // Simple implementation: exact match or major.minor match for now
        // Full semver range parsing can be added later if needed
        if let Ok(req_version) = Version::from_str(requirement.trim_start_matches(">=")) {
            *self >= req_version
        } else if let Ok(req_version) = Version::from_str(requirement) {
            *self == req_version
        } else {
            false
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat);
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| VersionParseError::InvalidNumber)?;

        Ok(Version { major, minor, patch })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VersionParseError {
    #[error("Invalid version format, expected major.minor.patch")]
    InvalidFormat,
    #[error("Invalid version number")]
    InvalidNumber,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse() {
        let v = Version::from_str("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn version_comparison() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert_eq!(v1, Version::from_str("1.2.3").unwrap());
    }

    #[test]
    fn version_satisfies() {
        let v = Version::new(1, 2, 3);
        assert!(v.satisfies("1.2.3"));
        assert!(v.satisfies(">=1.0.0"));
        assert!(!v.satisfies("2.0.0"));
    }
}
