use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid archive format")]
    InvalidFormat,
    #[error("File not found in archive: {0}")]
    NotFound(String),
}

/// Archive file entry metadata.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub path: String,
    pub offset: u64,
    pub size: u64,
}

/// Archive metadata (V2).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArchiveMetadata {
    pub version: String,
    pub created: u64, // Unix timestamp
}

/// Packed archive containing multiple files.
pub struct Archive {
    entries: HashMap<String, ArchiveEntry>,
    base_path: PathBuf,
}

/// Archive header magic bytes (V1 for backward compatibility).
const ARCHIVE_MAGIC_V1: &[u8; 8] = b"KILNARV1";

/// Archive header magic bytes (V2 with version support).
const ARCHIVE_MAGIC_V2: &[u8; 8] = b"KILNARV2";

impl Archive {
    /// Create a new empty archive.
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            entries: HashMap::new(),
            base_path,
        }
    }

    /// Pack files into the archive (V1 format for backward compatibility).
    pub fn pack(&mut self, files: &[PathBuf]) -> Result<(), ArchiveError> {
        self.pack_with_version(files, None)
    }

    /// Pack files into the archive with optional version (V2 format if version provided).
    pub fn pack_with_version(
        &mut self,
        files: &[PathBuf],
        version: Option<&str>,
    ) -> Result<(), ArchiveError> {
        let archive_path = &self.base_path;
        let file = File::create(archive_path)?;
        let mut writer = BufWriter::new(file);

        // Write magic header (V2 if version provided, else V1)
        let magic = if version.is_some() {
            ARCHIVE_MAGIC_V2
        } else {
            ARCHIVE_MAGIC_V1
        };
        writer.write_all(magic)?;

        // Write placeholder for entry count
        let _count_offset = writer.stream_position()?;
        writer.write_all(&0u32.to_le_bytes())?;

        // Write metadata if V2
        if let Some(ver) = version {
            let meta = ArchiveMetadata {
                version: ver.to_string(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let meta_bytes = serde_json::to_vec(&meta).map_err(|_| ArchiveError::InvalidFormat)?;
            let meta_len = meta_bytes.len() as u32;
            writer.write_all(&meta_len.to_le_bytes())?;
            writer.write_all(&meta_bytes)?;
        }

        let mut entries: Vec<ArchiveEntry> = Vec::new();
        let mut data_offset = writer.stream_position()?;

        // Write file data
        for path in files {
            let file = File::open(path)?;
            let metadata = file.metadata()?;
            let size = metadata.len();

            let entry = ArchiveEntry {
                path: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                offset: data_offset,
                size,
            };

            let mut reader = BufReader::new(file);
            io::copy(&mut reader, &mut writer)?;

            data_offset += size;
            entries.push(entry);
        }

        // Write entry table
        let table_offset = data_offset;
        writer.write_all(&(entries.len() as u32).to_le_bytes())?;

        for entry in &entries {
            let path_bytes = entry.path.as_bytes();
            writer.write_all(&(path_bytes.len() as u16).to_le_bytes())?;
            writer.write_all(path_bytes)?;
            writer.write_all(&entry.offset.to_le_bytes())?;
            writer.write_all(&entry.size.to_le_bytes())?;
        }

        // Write table offset
        writer.write_all(&table_offset.to_le_bytes())?;

        writer.flush()?;
        Ok(())
    }

    /// Load an archive from disk.
    pub fn load(path: &Path) -> Result<Self, ArchiveError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        let is_v2 = &magic == ARCHIVE_MAGIC_V2;
        if !is_v2 && &magic != ARCHIVE_MAGIC_V1 {
            return Err(ArchiveError::InvalidFormat);
        }

        // Skip entry count placeholder
        let mut _count_bytes = [0u8; 4];
        reader.read_exact(&mut _count_bytes)?;

        // Skip metadata if V2
        if is_v2 {
            let mut meta_len_bytes = [0u8; 4];
            reader.read_exact(&mut meta_len_bytes)?;
            let meta_len = u32::from_le_bytes(meta_len_bytes) as usize;
            reader.seek(io::SeekFrom::Current(meta_len as i64))?;
        }

        // Read table offset (last 8 bytes)
        let _file_size = reader.seek(io::SeekFrom::End(0))?;
        reader.seek(io::SeekFrom::End(-8))?;
        let mut table_offset_bytes = [0u8; 8];
        reader.read_exact(&mut table_offset_bytes)?;
        let table_offset = u64::from_le_bytes(table_offset_bytes);

        // Seek to entry table
        reader.seek(io::SeekFrom::Start(table_offset))?;

        // Read entry count
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let count = u32::from_le_bytes(count_bytes) as usize;

        let mut entries = HashMap::new();
        for _ in 0..count {
            let mut path_len_bytes = [0u8; 2];
            reader.read_exact(&mut path_len_bytes)?;
            let path_len = u16::from_le_bytes(path_len_bytes) as usize;

            let mut path_bytes = vec![0u8; path_len];
            reader.read_exact(&mut path_bytes)?;
            let path = String::from_utf8(path_bytes).map_err(|_| ArchiveError::InvalidFormat)?;

            let mut offset_bytes = [0u8; 8];
            reader.read_exact(&mut offset_bytes)?;
            let offset = u64::from_le_bytes(offset_bytes);

            let mut size_bytes = [0u8; 8];
            reader.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);

            entries.insert(path.clone(), ArchiveEntry { path, offset, size });
        }

        Ok(Self {
            entries,
            base_path: path.to_path_buf(),
        })
    }

    /// Get archive metadata (only for V2 archives).
    pub fn get_metadata(path: &Path) -> Result<Option<ArchiveMetadata>, ArchiveError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != ARCHIVE_MAGIC_V2 {
            return Ok(None); // V1 archives have no metadata
        }

        // Skip the entry count placeholder (4 bytes)
        let mut _count_bytes = [0u8; 4];
        reader.read_exact(&mut _count_bytes)?;

        // Read metadata length
        let mut meta_len_bytes = [0u8; 4];
        reader.read_exact(&mut meta_len_bytes)?;
        let meta_len = u32::from_le_bytes(meta_len_bytes) as usize;

        // Read metadata
        let mut meta_bytes = vec![0u8; meta_len];
        reader.read_exact(&mut meta_bytes)?;

        let metadata: ArchiveMetadata =
            serde_json::from_slice(&meta_bytes).map_err(|_| ArchiveError::InvalidFormat)?;

        Ok(Some(metadata))
    }

    /// Get metadata for a file in the archive.
    pub fn get_entry(&self, path: &str) -> Option<&ArchiveEntry> {
        self.entries.get(path)
    }

    /// Extract a file from the archive.
    pub fn extract(&self, path: &str) -> Result<Vec<u8>, ArchiveError> {
        let entry = self
            .get_entry(path)
            .ok_or_else(|| ArchiveError::NotFound(path.to_string()))?;

        let file = File::open(&self.base_path)?;
        let mut reader = BufReader::new(file);
        reader.seek(io::SeekFrom::Start(entry.offset))?;

        let mut data = vec![0u8; entry.size as usize];
        reader.read_exact(&mut data)?;

        Ok(data)
    }

    /// List all files in the archive.
    pub fn list(&self) -> Vec<&str> {
        self.entries.keys().map(|k| k.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn pack_and_extract() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("archive.kar");

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        let mut archive = Archive::new(archive_path.clone());
        archive.pack(&[file1, file2]).unwrap();

        let loaded = Archive::load(&archive_path).unwrap();
        let data1 = loaded.extract("file1.txt").unwrap();
        let data2 = loaded.extract("file2.txt").unwrap();

        assert_eq!(data1, b"content1");
        assert_eq!(data2, b"content2");
    }

    #[test]
    fn list_files() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("archive.kar");

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        let mut archive = Archive::new(archive_path.clone());
        archive.pack(&[file1, file2]).unwrap();

        let loaded = Archive::load(&archive_path).unwrap();
        let files = loaded.list();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&"file1.txt"));
        assert!(files.contains(&"file2.txt"));
    }

    #[test]
    fn extract_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("archive.kar");

        let file = temp_dir.path().join("file.txt");
        fs::write(&file, b"content").unwrap();

        let mut archive = Archive::new(archive_path.clone());
        archive.pack(&[file]).unwrap();

        let loaded = Archive::load(&archive_path).unwrap();
        let result = loaded.extract("nonexistent.txt");

        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }

    #[test]
    fn pack_with_version_v2() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("archive_v2.kar");

        let file = temp_dir.path().join("file.txt");
        fs::write(&file, b"content").unwrap();

        let mut archive = Archive::new(archive_path.clone());
        archive.pack_with_version(&[file], Some("1.0.0")).unwrap();

        let loaded = Archive::load(&archive_path).unwrap();
        let data = loaded.extract("file.txt").unwrap();

        assert_eq!(data, b"content");

        let metadata = Archive::get_metadata(&archive_path).unwrap();
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().version, "1.0.0");
    }

    #[test]
    fn v1_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("archive_v1.kar");

        let file = temp_dir.path().join("file.txt");
        fs::write(&file, b"content").unwrap();

        let mut archive = Archive::new(archive_path.clone());
        archive.pack(&[file]).unwrap(); // V1 format

        let loaded = Archive::load(&archive_path).unwrap();
        let data = loaded.extract("file.txt").unwrap();

        assert_eq!(data, b"content");

        let metadata = Archive::get_metadata(&archive_path).unwrap();
        assert!(metadata.is_none()); // V1 has no metadata
    }
}
