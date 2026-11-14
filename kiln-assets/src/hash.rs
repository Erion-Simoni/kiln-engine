use sha2::{Digest, Sha256};
use std::path::Path;
use std::{fs, io::Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HashError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compute SHA-256 content hash of a file.
pub fn content_hash(path: &Path) -> Result<String, HashError> {
    let mut input = std::io::BufReader::new(fs::File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn streamed_hash_includes_bytes_beyond_chunk_boundaries() {
        let file = NamedTempFile::new().unwrap();
        let bytes = (0..200_003).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        fs::write(file.path(), &bytes).unwrap();
        assert_eq!(
            content_hash(file.path()).unwrap(),
            hex::encode(Sha256::digest(&bytes))
        );
        fs::write(file.path(), b"abc").unwrap();
        assert_eq!(
            content_hash(file.path()).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
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
