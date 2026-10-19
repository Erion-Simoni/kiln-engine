use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HashError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compute SHA-256 content hash of a file.
pub fn content_hash(path: &Path) -> Result<String, HashError> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn hash_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let hash = content_hash(file.path()).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn hash_content() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"hello world").unwrap();
        let hash1 = content_hash(file.path()).unwrap();

        std::fs::write(file.path(), b"hello world!").unwrap();
        let hash2 = content_hash(file.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn hash_identical_files() {
        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();
        std::fs::write(file1.path(), b"test data").unwrap();
        std::fs::write(file2.path(), b"test data").unwrap();

        let hash1 = content_hash(file1.path()).unwrap();
        let hash2 = content_hash(file2.path()).unwrap();
        assert_eq!(hash1, hash2);
    }
}
