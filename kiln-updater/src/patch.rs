//! Delta patch generation and application using qbsdiff.

use qbsdiff::{Bsdiff, Bspatch};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use thiserror::Error;

/// Error type for patch operations.
#[derive(Debug, Error)]
pub enum PatchError {
    #[error("Failed to read old archive: {0}")]
    ReadOld(#[from] std::io::Error),
    #[error("Failed to generate patch: {0}")]
    Generate(String),
    #[error("Failed to apply patch: {0}")]
    Apply(String),
    #[error("Hash verification failed")]
    HashMismatch,
}

/// Patch metadata for verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatchMetadata {
    pub from_version: String,
    pub to_version: String,
    pub old_hash: String,
    pub new_hash: String,
    pub patch_size: u64,
}

/// Generate a delta patch between two archives.
pub fn generate_patch(
    old_path: &str,
    new_path: &str,
) -> Result<(Vec<u8>, PatchMetadata), PatchError> {
    let old_data = std::fs::read(old_path)?;
    let new_data = std::fs::read(new_path)?;

    let old_hash = hash_bytes(&old_data);
    let new_hash = hash_bytes(&new_data);

    let mut patch = Vec::new();
    Bsdiff::new(&old_data, &new_data)
        .compare(Cursor::new(&mut patch))
        .map_err(|e| PatchError::Generate(e.to_string()))?;

    let metadata = PatchMetadata {
        from_version: "unknown".to_string(), // Will be set by caller
        to_version: "unknown".to_string(),
        old_hash,
        new_hash,
        patch_size: patch.len() as u64,
    };

    Ok((patch, metadata))
}

/// Apply a delta patch to an archive.
pub fn apply_patch(
    old_data: &[u8],
    patch_data: &[u8],
    expected_hash: &str,
) -> Result<Vec<u8>, PatchError> {
    let patcher = Bspatch::new(patch_data).map_err(|e| PatchError::Apply(e.to_string()))?;

    let mut new_data = Vec::with_capacity(patcher.hint_target_size() as usize);
    patcher
        .apply(old_data, Cursor::new(&mut new_data))
        .map_err(|e| PatchError::Apply(e.to_string()))?;

    let actual_hash = hash_bytes(&new_data);
    if actual_hash != expected_hash {
        return Err(PatchError::HashMismatch);
    }

    Ok(new_data)
}

/// Compute SHA-256 hash of bytes.
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_roundtrip() {
        let old = vec![1, 2, 3, 4, 5];
        let new = vec![1, 2, 4, 6];

        let mut patch = Vec::new();
        Bsdiff::new(&old, &new)
            .compare(Cursor::new(&mut patch))
            .unwrap();

        let patcher = Bspatch::new(&patch).unwrap();
        let mut patched = Vec::with_capacity(new.len());
        patcher.apply(&old, Cursor::new(&mut patched)).unwrap();

        assert_eq!(patched, new);
    }

    #[test]
    fn hash_verification() {
        let data = b"test data";
        let hash = hash_bytes(data);
        assert_eq!(hash.len(), 64); // SHA-256 is 64 hex chars
    }
}
