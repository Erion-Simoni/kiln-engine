//! Editor asset browser.
//!
//! Scans the project directory for image assets, decodes PNGs with
//! [`kiln_assets::decode_png`] and keeps a CPU-side preview of the selected
//! image, which the asset panel draws with `egui`.
//!
//! Supports drag-and-drop of assets into the scene, filtering by name,
//! and folder creation.

use kiln_assets::decode_png;
use egui::{Id, Sense, Ui};
use std::collections::HashMap;
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
            Ok::<(usize, usize, Vec<u8>), String>((image.width as usize, image.height as usize, image.pixels))
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
        Some(egui::ColorImage::from_rgba_unmultiplied([width, height], pixels))
    }

    /// Size of the current preview in pixels, as `(width, height)`.
    pub fn preview_size(&self) -> Option<(usize, usize)> {
        self.preview_size
    }
}

/// Recursively collect files below `dir`.
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
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
