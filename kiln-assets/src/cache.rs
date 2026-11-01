use crate::hash::{content_hash, HashError};
use crate::settings::ImportSettings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("Walk error: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
}

/// Cache entry tracking a file's content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub hash: String,
    pub path: PathBuf,
}

/// Import result indicating whether file was skipped or imported.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportResult {
    /// File skipped (hash unchanged)
    Skipped { path: PathBuf },
    /// File imported (hash changed or new)
    Imported { path: PathBuf, hash: String },
}

/// Asset cache tracking file hashes for incremental imports.
pub struct AssetCache {
    cache_dir: PathBuf,
    entries: HashMap<PathBuf, CacheEntry>,
    /// Asset dependency graph: dependency path -> set of dependent paths.
    dependencies: HashMap<String, Vec<String>>,
    /// Import settings per asset path.
    import_settings: HashMap<PathBuf, ImportSettings>,
}

impl AssetCache {
    /// Create a new cache with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            entries: HashMap::new(),
            dependencies: HashMap::new(),
            import_settings: HashMap::new(),
        }
    }

    /// Load cache from disk if it exists.
    pub fn load(&mut self) -> Result<(), CacheError> {
        let cache_file = self.cache_dir.join("kiln-cache.json");
        if !cache_file.exists() {
            return Ok(());
        }

        let data = fs::read_to_string(&cache_file)?;
        let loaded: Vec<CacheEntry> = serde_json::from_str(&data)?;

        self.entries.clear();
        for entry in loaded {
            self.entries.insert(entry.path.clone(), entry);
        }

        Ok(())
    }

    /// Save cache to disk.
    pub fn save(&self) -> Result<(), CacheError> {
        fs::create_dir_all(&self.cache_dir)?;
        let cache_file = self.cache_dir.join("kiln-cache.json");

        let entries: Vec<&CacheEntry> = self.entries.values().collect();
        let data = serde_json::to_string_pretty(&entries)?;
        fs::write(&cache_file, data)?;

        Ok(())
    }

    /// Import a file, returning whether it was skipped or imported.
    pub fn import(&mut self, path: &Path) -> Result<ImportResult, CacheError> {
        let hash = content_hash(path)?;
        let canonical = fs::canonicalize(path)?;

        if let Some(existing) = self.entries.get(&canonical) {
            if existing.hash == hash {
                return Ok(ImportResult::Skipped { path: canonical });
            }
        }

        self.entries.insert(
            canonical.clone(),
            CacheEntry {
                hash: hash.clone(),
                path: canonical.clone(),
            },
        );

        Ok(ImportResult::Imported {
            path: canonical,
            hash,
        })
    }

    /// Import all files in a directory recursively.
    pub fn import_dir(&mut self, dir: &Path) -> Result<Vec<ImportResult>, CacheError> {
        let mut results = Vec::new();

        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let result = self.import(entry.path())?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get the cached hash for a file, if known.
    pub fn get_hash(&self, path: &Path) -> Option<&str> {
        let canonical = path.canonicalize().ok()?;
        self.entries.get(&canonical).map(|e| e.hash.as_str())
    }

    /// Register import settings for a specific asset path.
    pub fn set_import_settings(
        &mut self,
        path: &Path,
        settings: ImportSettings,
    ) {
        self.import_settings.insert(path.to_path_buf(), settings);
    }

    /// Get the import settings for a given path, returning defaults if none set.
    pub fn get_import_settings(&self, path: &Path) -> ImportSettings {
        self.import_settings
            .get(path)
            .copied()
            .unwrap_or_default()
    }

    /// Record that `dependent` depends on `dependency`.
    pub fn add_dependency(&mut self, dependent: &str, dependency: &str) {
        self.dependencies
            .entry(dependency.to_string())
            .or_default()
            .push(dependent.to_string());
    }

    /// Get all assets that depend on the given path.
    pub fn dependents(&self, dependency: &str) -> Vec<&str> {
        self.dependencies
            .get(dependency)
            .map(|deps| deps.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Find all asset paths that no longer exist on disk.
    pub fn missing_assets(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for path in self.entries.keys() {
            if !path.exists() {
                missing.push(path.to_string_lossy().to_string());
            }
        }
        missing
    }

    /// Find groups of duplicate assets (same content hash, different paths).
    pub fn duplicate_assets(&self) -> Vec<Vec<PathBuf>> {
        let mut by_hash: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for (path, entry) in &self.entries {
            by_hash.entry(entry.hash.clone()).or_default().push(path.clone());
        }
        by_hash
            .into_values()
            .filter(|paths| paths.len() > 1)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn import_new_file() {
        let cache_dir = TempDir::new().unwrap();
        let mut cache = AssetCache::new(cache_dir.path().to_path_buf());

        let file = cache_dir.path().join("test.txt");
        fs::write(&file, b"content").unwrap();

        let result = cache.import(&file).unwrap();
        assert!(matches!(result, ImportResult::Imported { .. }));
    }

    #[test]
    fn skip_unchanged_file() {
        let cache_dir = TempDir::new().unwrap();
        let mut cache = AssetCache::new(cache_dir.path().to_path_buf());

        let file = cache_dir.path().join("test.txt");
        fs::write(&file, b"content").unwrap();

        let result1 = cache.import(&file).unwrap();
        assert!(matches!(result1, ImportResult::Imported { .. }));

        let result2 = cache.import(&file).unwrap();
        assert!(matches!(result2, ImportResult::Skipped { .. }));
    }

    #[test]
    fn reimport_changed_file() {
        let cache_dir = TempDir::new().unwrap();
        let mut cache = AssetCache::new(cache_dir.path().to_path_buf());

        let file = cache_dir.path().join("test.txt");
        fs::write(&file, b"content").unwrap();

        cache.import(&file).unwrap();

        fs::write(&file, b"new content").unwrap();
        let result = cache.import(&file).unwrap();
        assert!(matches!(result, ImportResult::Imported { .. }));
    }

    #[test]
    fn save_and_load_cache() {
        let cache_dir = TempDir::new().unwrap();
        let file = cache_dir.path().join("test.txt");
        fs::write(&file, b"content").unwrap();

        {
            let mut cache = AssetCache::new(cache_dir.path().to_path_buf());
            cache.import(&file).unwrap();
            cache.save().unwrap();
        }

        let mut cache = AssetCache::new(cache_dir.path().to_path_buf());
        cache.load().unwrap();

        let hash = cache.get_hash(&file);
        assert!(hash.is_some());
    }
}
