//! Editor asset browser.
//!
//! Scans the project directory for image assets, decodes PNGs with
//! [`kiln_assets::decode_png`] and keeps a CPU-side preview of the selected
//! image, which the asset panel draws with `egui`.

use kiln_assets::decode_png;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The kind of an asset listed in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// A PNG image, which can be previewed and used as a sprite texture.
    Image,
    /// Any other file, listed but not previewed.
    Other,
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

    /// Scan `root` for assets, replacing the current listing.
    ///
    /// The manifest file itself is skipped. Files are listed relative to
    /// `root`, so `sprites/hero.png` shows up under that name.
    pub fn scan(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        self.entries.clear();
        self.errors.clear();
        self.preview_source = None;
        self.preview_pixels = None;
        self.preview_size = None;

        if !root.is_dir() {
            return;
        }

        let mut files = Vec::new();
        collect_files(root, &mut files);
        files.sort();

        for file in files {
            let relative = relative_path(root, &file);
            let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            let kind = match extension(&file).as_deref() {
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
    /// decoded; the previous preview is kept in that case.
    pub fn select(&mut self, path: &str) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Err("no project directory".to_string());
        };

        let full = root.join(path);
        let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
        let image = decode_png(&bytes).map_err(|e| e.to_string())?;

        self.preview_source = Some(path.to_string());
        self.preview_size = Some((image.width as usize, image.height as usize));
        self.preview_pixels = Some(image.pixels);
        Ok(())
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
        .replace('\\', "/")
}

/// Lower-cased file extension of `path`, if any.
fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// Group assets by their top level directory, for a tree-like listing.
pub fn group_by_folder(entries: &[AssetEntry]) -> HashMap<String, Vec<&AssetEntry>> {
    let mut groups: HashMap<String, Vec<&AssetEntry>> = HashMap::new();
    for entry in entries {
        let folder = match entry.path.rsplit_once('/') {
            Some((folder, _)) => folder.to_string(),
            None => String::new(),
        };
        groups.entry(folder).or_default().push(entry);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_assets::encode_png_rgba;

    fn write_file(dir: &Path, relative: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, b"not a real png").unwrap();
    }

    #[test]
    fn scan_lists_files_recursively() {
        let dir = std::env::temp_dir().join("kiln-editor-assets-scan");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_file(&dir, "sprites/hero.png");
        write_file(&dir, "notes.txt");

        let mut browser = AssetBrowser::new();
        browser.scan(&dir);

        let paths: Vec<&str> = browser.entries().iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"sprites/hero.png"), "{paths:?}");
        assert!(paths.contains(&"notes.txt"), "{paths:?}");

        let hero = browser
            .entries()
            .iter()
            .find(|e| e.path == "sprites/hero.png")
            .unwrap();
        assert_eq!(hero.kind, AssetKind::Image);

        let notes = browser
            .entries()
            .iter()
            .find(|e| e.path == "notes.txt")
            .unwrap();
        assert_eq!(notes.kind, AssetKind::Other);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_missing_directory_is_empty() {
        let mut browser = AssetBrowser::new();
        browser.scan(Path::new("does/not/exist"));
        assert!(browser.entries().is_empty());
    }

    #[test]
    fn groups_by_folder() {
        let entries = vec![
            AssetEntry {
                path: "sprites/hero.png".to_string(),
                size: 1,
                kind: AssetKind::Image,
            },
            AssetEntry {
                path: "sprites/villain.png".to_string(),
                size: 2,
                kind: AssetKind::Image,
            },
            AssetEntry {
                path: "readme.txt".to_string(),
                size: 3,
                kind: AssetKind::Other,
            },
        ];

        let groups = group_by_folder(&entries);
        assert_eq!(groups["sprites"].len(), 2);
        assert_eq!(groups[""].len(), 1);
    }

    #[test]
    fn selecting_an_invalid_png_reports_an_error() {
        let dir = std::env::temp_dir().join("kiln-editor-assets-select");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_file(&dir, "bad.png");

        let mut browser = AssetBrowser::new();
        browser.scan(&dir);
        assert!(browser.select("bad.png").is_err());
        assert!(browser.preview_source().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selecting_a_valid_png_builds_a_preview() {
        let dir = std::env::temp_dir().join("kiln-editor-assets-preview");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // A 2x1 RGBA PNG: one opaque red pixel, one translucent blue pixel.
        let png_bytes =
            encode_png_rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128]).expect("encode png");
        std::fs::write(dir.join("hero.png"), &png_bytes).unwrap();

        let mut browser = AssetBrowser::new();
        browser.scan(&dir);
        browser.select("hero.png").expect("preview");

        assert_eq!(browser.preview_source(), Some("hero.png"));
        assert_eq!(browser.preview_size(), Some((2, 1)));
        assert_eq!(browser.preview_pixels().map(|p| p.len()), Some(8));

        let image = browser.preview_color_image().expect("egui image");
        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clearing_the_preview_removes_it() {
        let mut browser = AssetBrowser::new();
        browser.clear_preview();
        assert!(browser.preview_source().is_none());
        assert!(browser.preview_color_image().is_none());
        assert!(browser.preview_size().is_none());
    }
}
