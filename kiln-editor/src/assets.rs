//! Editor asset browser.
//!
//! Scans the project directory for image assets, decodes PNGs with
//! [`kiln_assets::decode_png`] and keeps a CPU-side preview of the selected
//! image, which the asset panel draws with `egui`.
//!
//! Supports drag-and-drop of assets into the scene, filtering by name,
//! and folder creation.

use kiln_assets::decode_png;

/// Typed payload keeps image drops distinct from prefab IDs.
#[derive(Debug, Clone)]
pub struct SpriteAssetPayload {
    pub path: String,
}
use std::path::{Path, PathBuf};

/// The kind of an asset listed in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// A PNG image, which can be previewed and used as a sprite texture.
    Image,
    /// Any other file, listed but not previewed.
    Other,
    /// A folder/directory.
    Folder,
}

/// A single asset found in the project directory.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    /// Path relative to the project directory, using `/` separators.
    pub path: String,
    /// Size of the file in bytes.
    pub size: u64,
    /// What kind of asset this is.
    pub kind: AssetKind,
}

/// The state of the editor's asset browser.
#[derive(Debug, Default)]
pub struct AssetBrowser {
    /// Root directory that is scanned for assets.
    root: Option<PathBuf>,
    /// Assets found in the last scan, sorted by path.
    entries: Vec<AssetEntry>,
    /// Path of the asset the preview was built from.
    preview_source: Option<String>,
    /// Decoded preview pixels (RGBA8) of the selected asset.
    preview_pixels: Option<Vec<u8>>,
    /// Preview size in pixels.
    preview_size: Option<(usize, usize)>,
    /// Load errors of the last scan, as `(path, message)` pairs.
    errors: Vec<(String, String)>,
    /// Filter string for filtering the asset list by name.
    filter: String,
    /// Currently dragged asset path for drag operations.
    drag_source: Option<String>,
}

impl AssetBrowser {
    /// Create an empty browser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Directory that is scanned for assets.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Assets found in the last scan.
    pub fn entries(&self) -> &[AssetEntry] {
        &self.entries
    }

    /// Filtered assets based on the current filter string.
    pub fn filtered_entries(&self) -> Vec<&AssetEntry> {
        if self.filter.is_empty() {
            self.entries.iter().collect()
        } else {
            let filter = self.filter.to_lowercase();
            self.entries
                .iter()
                .filter(|e| e.path.to_lowercase().contains(&filter))
                .collect()
        }
    }

    /// Errors of the last scan.
    pub fn errors(&self) -> &[(String, String)] {
        &self.errors
    }

    /// Path of the asset that is currently previewed.
    pub fn preview_source(&self) -> Option<&str> {
        self.preview_source.as_deref()
    }

    /// Decoded preview pixels of the selected asset, as RGBA8.
    pub fn preview_pixels(&self) -> Option<&[u8]> {
        self.preview_pixels.as_deref()
    }

    /// Current filter text.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Set the filter text for filtering assets.
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    /// Currently dragged asset path, if any.
    pub fn drag_source(&self) -> Option<&str> {
        self.drag_source.as_deref()
    }

    /// Set the dragged asset source.
    pub fn set_drag_source(&mut self, source: Option<&str>) {
        self.drag_source = source.map(|s| s.to_string());
    }

    /// Scan `root` for assets, replacing the current listing.
    ///
    /// Project manifest files (`kiln.toml` and friends) are skipped, since
    /// they are parsed separately and are not assets. Files are listed
    /// relative to `root`, so `sprites/hero.png` shows up under that name.
    pub fn scan(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        self.entries.clear();
        self.errors.clear();
        self.preview_source = None;
        self.preview_pixels = None;
        self.preview_size = None;
        self.drag_source = None;

        if !root.is_dir() {
            return;
        }

        let mut files = Vec::new();
        collect_files(root, &mut files);
        files.sort();

        for file in files {
            let relative = relative_path(root, &file);
            if is_manifest(&relative) {
                continue;
            }
            let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            let kind = match file.extension().and_then(|s| s.to_str()) {
                Some("png") => AssetKind::Image,
                _ => AssetKind::Other,
            };
            self.entries.push(AssetEntry {
                path: relative,
                size,
                kind,
            });
        }
    }

    /// Move a project asset to another relative path without overwriting an
    /// existing file or allowing the target to escape the project directory.
    pub fn rename_asset(&mut self, source: &str, destination: &str) -> Result<(), String> {
        let root = self.root.clone().ok_or("No project directory is open")?;
        let source_path = safe_project_path(source)?;
        let destination_path = safe_project_path(destination)?;
        let from = root.join(source_path);
        let to = root.join(destination_path);
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let canonical_source = from.canonicalize().map_err(|error| error.to_string())?;
        let canonical_parent = to
            .parent()
            .ok_or("Destination path must be inside the project")?
            .canonicalize()
            .map_err(|error| format!("Destination folder does not exist: {error}"))?;
        if !canonical_source.starts_with(&root) || !canonical_parent.starts_with(&root) {
            return Err("Asset paths must stay inside the project directory".into());
        }
        if std::fs::symlink_metadata(&from)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err("Moving symbolic-link assets is not supported".into());
        }
        if !from.is_file() {
            return Err("Selected asset does not exist or is not a file".into());
        }
        if to.exists() {
            return Err("An asset already exists at the destination".into());
        }
        std::fs::rename(&from, &to).map_err(|error| format!("Could not move asset: {error}"))?;
        self.scan(&root);
        Ok(())
    }

    /// Decode the asset at `path` and keep it as the preview.
    ///
    /// Returns an error message when the asset is missing or cannot be
    /// decoded; the previous preview is kept in that case. The failure is
    /// also recorded in [`Self::errors`].
    pub fn select(&mut self, path: &str) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Err("no project directory".to_string());
        };

        let full = root.join(path);
        let result = (|| {
            let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
            let image = decode_png(&bytes).map_err(|e| e.to_string())?;
            Ok::<(usize, usize, Vec<u8>), String>((
                image.width as usize,
                image.height as usize,
                image.pixels,
            ))
        })();

        match result {
            Ok((width, height, pixels)) => {
                self.preview_source = Some(path.to_string());
                self.preview_size = Some((width, height));
                self.preview_pixels = Some(pixels);
                Ok(())
            }
            Err(message) => {
                self.errors.push((path.to_string(), message.clone()));
                Err(message)
            }
        }
    }

    /// Clear the current preview.
    pub fn clear_preview(&mut self) {
        self.preview_source = None;
        self.preview_pixels = None;
        self.preview_size = None;
    }

    /// Build an `egui` colour image of the current preview.
    ///
    /// Returns `None` when no asset is selected.
    pub fn preview_color_image(&self) -> Option<egui::ColorImage> {
        let (width, height) = self.preview_size?;
        let pixels = self.preview_pixels.as_ref()?;
        if pixels.len() != width * height * 4 {
            return None;
        }
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [width, height],
            pixels,
        ))
    }

    /// Size of the current preview in pixels, as `(width, height)`.
    pub fn preview_size(&self) -> Option<(usize, usize)> {
        self.preview_size
    }
}

fn safe_project_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Asset path cannot be empty".into());
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("Asset path must stay inside the project directory".into());
    }
    Ok(path.to_path_buf())
}

/// Recursively collect files below `dir`.
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | ".kiln-cache" | "target")
            ) {
                continue;
            }
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Path of `file` relative to `root`, using `/` separators.
fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or_default()
        .replace("\\", "/")
}

/// Check if a path is a project manifest file that should be skipped.
fn is_manifest(path: &str) -> bool {
    path == "kiln.toml" || path.ends_with("/kiln.toml") || path.ends_with(r"\kiln.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_rename_moves_files_and_rejects_escape_or_overwrite() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("art")).unwrap();
        std::fs::write(root.path().join("old.png"), b"png").unwrap();
        std::fs::write(root.path().join("art/existing.png"), b"keep").unwrap();
        let mut browser = AssetBrowser::new();
        browser.scan(root.path());

        assert!(browser.rename_asset("old.png", "../escape.png").is_err());
        assert!(browser.rename_asset("old.png", "art/existing.png").is_err());
        browser.rename_asset("old.png", "art/renamed.png").unwrap();
        assert!(!root.path().join("old.png").exists());
        assert_eq!(
            std::fs::read(root.path().join("art/renamed.png")).unwrap(),
            b"png"
        );
        assert!(browser
            .entries()
            .iter()
            .any(|entry| entry.path == "art/renamed.png"));
    }

    #[cfg(unix)]
    #[test]
    fn asset_scan_does_not_follow_project_symlinks() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.png"), b"outside").unwrap();
        symlink(outside.path(), root.path().join("linked")).unwrap();
        let mut browser = AssetBrowser::new();
        browser.scan(root.path());
        assert!(!browser
            .entries()
            .iter()
            .any(|entry| entry.path.contains("secret")));
    }
}
