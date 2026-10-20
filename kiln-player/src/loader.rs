//! Loading a game for the player.
//!
//! Two sources are supported:
//!
//! * a packed `.kar` archive (see [`kiln_assets::Archive`]), and
//! * a plain project directory containing a `kiln.toml` (or `kiln.ron`)
//!   manifest, as produced by `kiln init`.
//!
//! Both are normalised into a [`LoadedGame`] holding the parsed [`Project`],
//! the raw asset bytes, and the id of the scene that should be started.

use kiln_assets::archive::ArchiveError;
use kiln_assets::Archive;
use kiln_core::Project;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Manifest file names looked for inside an archive or project directory.
pub const PROJECT_FILE_NAMES: &[&str] = &["kiln.toml", "kiln.ron", "project.toml", "project.ron"];

/// Errors that can occur while loading a game.
#[derive(Debug, Error)]
pub enum LoaderError {
    /// Filesystem failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The `.kar` archive could not be read.
    #[error("Archive error: {0}")]
    Archive(#[from] ArchiveError),
    /// No project manifest was found in the archive or directory.
    #[error("No project file found (looked for: {0})", PROJECT_FILE_NAMES.join(", "))]
    ProjectFileNotFound,
    /// The project manifest could not be parsed.
    #[error("Failed to parse project file '{0}': {1}")]
    Parse(String, String),
    /// The project does not contain any scene.
    #[error("Project does not define any scene")]
    NoScenes,
    /// The requested scene does not exist in the project.
    #[error("Scene '{0}' not found in project")]
    SceneNotFound(String),
}

/// Result of loading a project's manifest and raw assets.
type LoadResult = Result<(Project, GameSource, HashMap<String, Vec<u8>>), LoaderError>;

/// Where a loaded game came from.
pub enum GameSource {
    /// A packed `.kar` archive.
    Archive {
        /// Path of the archive on disk.
        path: PathBuf,
        /// The opened archive.
        archive: Archive,
    },
    /// A directory holding the project manifest and assets.
    Directory {
        /// Path of the project directory.
        path: PathBuf,
    },
}

impl GameSource {
    /// Human readable description used in log output.
    pub fn describe(&self) -> String {
        match self {
            GameSource::Archive { path, archive } => {
                format!(
                    "archive {} ({} files)",
                    path.display(),
                    archive.list().len()
                )
            }
            GameSource::Directory { path } => format!("project directory {}", path.display()),
        }
    }
}

/// A game that has been loaded from disk, ready to be spawned into a world.
pub struct LoadedGame {
    /// The parsed project manifest.
    pub project: Project,
    /// The source the game was loaded from.
    pub source: GameSource,
    /// Asset files keyed by their path/name inside the source.
    pub assets: HashMap<String, Vec<u8>>,
    /// Id of the scene selected for startup.
    pub scene_id: String,
}

impl LoadedGame {
    /// The scene that should be started.
    pub fn scene(&self) -> &kiln_core::Scene {
        self.project
            .scenes
            .get(&self.scene_id)
            .expect("scene id was validated during load")
    }
}

impl std::fmt::Debug for LoadedGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedGame")
            .field("project", &self.project)
            .field("source", &format_args!("{}", self.source.describe()))
            .field("assets", &self.assets.keys().collect::<Vec<&String>>())
            .field("scene_id", &self.scene_id)
            .finish()
    }
}

/// Load a game from a `.kar` archive or a project directory.
///
/// When `scene` is `None` the first scene of the project (by sorted id) is
/// selected so that startup is deterministic.
pub fn load(path: &Path, scene: Option<&str>) -> Result<LoadedGame, LoaderError> {
    let (project, source, assets) = if path.is_dir() {
        load_directory(path)?
    } else {
        load_archive(path)?
    };

    let scene_id = select_scene(&project, scene)?;
    Ok(LoadedGame {
        project,
        source,
        assets,
        scene_id,
    })
}

/// Load from a project directory (`kiln.toml` / `kiln.ron` + assets).
fn load_directory(dir: &Path) -> LoadResult {
    let (manifest_path, manifest_bytes) = find_project_file(dir)?;
    let project = parse_project(&manifest_path, &manifest_bytes)?;

    let mut assets = HashMap::new();
    collect_directory_assets(dir, &manifest_path, &mut assets)?;

    Ok((
        project,
        GameSource::Directory {
            path: dir.to_path_buf(),
        },
        assets,
    ))
}

/// Load from a packed `.kar` archive.
fn load_archive(path: &Path) -> LoadResult {
    let archive = Archive::load(path)?;

    let mut assets = HashMap::new();
    for name in archive.list() {
        let name = name.to_string();
        if let Ok(bytes) = archive.extract(&name) {
            assets.insert(name, bytes);
        }
    }

    let (manifest_name, manifest_bytes) = assets
        .iter()
        .find(|(name, _)| is_project_file_name(name))
        .map(|(name, bytes)| (name.clone(), bytes.clone()))
        .ok_or(LoaderError::ProjectFileNotFound)?;

    let project = parse_project(Path::new(&manifest_name), &manifest_bytes)?;

    Ok((
        project,
        GameSource::Archive {
            path: path.to_path_buf(),
            archive,
        },
        assets,
    ))
}

/// Whether the file name is one of the recognised manifest names.
fn is_project_file_name(name: &str) -> bool {
    let file_name = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name);
    PROJECT_FILE_NAMES.contains(&file_name)
}

/// Find the project manifest inside a directory.
fn find_project_file(dir: &Path) -> Result<(PathBuf, Vec<u8>), LoaderError> {
    for candidate in PROJECT_FILE_NAMES {
        let path = dir.join(candidate);
        if path.is_file() {
            let bytes = std::fs::read(&path)?;
            return Ok((path, bytes));
        }
    }
    Err(LoaderError::ProjectFileNotFound)
}

/// Parse a project manifest from TOML or RON, based on the file extension.
fn parse_project(path: &Path, bytes: &[u8]) -> Result<Project, LoaderError> {
    let text = std::str::from_utf8(bytes).map_err(|e| {
        LoaderError::Parse(
            path.display().to_string(),
            format!("manifest is not valid UTF-8: {e}"),
        )
    })?;

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let result = match extension.as_str() {
        "ron" => ron::from_str::<Project>(text).map_err(|e| e.to_string()),
        _ => Project::from_toml(text),
    };

    result.map_err(|e| LoaderError::Parse(path.display().to_string(), e))
}

/// Collect every file in the project directory as an asset.
///
/// Files are keyed by their path relative to the project directory, so
/// `sprites/hero.png` is stored under that key. The manifest itself is
/// skipped, because it is parsed separately.
fn collect_directory_assets(
    dir: &Path,
    manifest_path: &Path,
    assets: &mut HashMap<String, Vec<u8>>,
) -> Result<(), LoaderError> {
    collect_directory_assets_inner(dir, dir, manifest_path, assets)
}

/// Recursive helper for [`collect_directory_assets`].
fn collect_directory_assets_inner(
    root: &Path,
    dir: &Path,
    manifest_path: &Path,
    assets: &mut HashMap<String, Vec<u8>>,
) -> Result<(), LoaderError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_directory_assets_inner(root, &path, manifest_path, assets)?;
            continue;
        }

        if !path.is_file() || path == manifest_path {
            continue;
        }

        let name = path
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or_default()
            .replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        let bytes = std::fs::read(&path)?;
        assets.insert(name, bytes);
    }
    Ok(())
}

/// Pick the scene to start: the requested one, or the first by sorted id.
fn select_scene(project: &Project, requested: Option<&str>) -> Result<String, LoaderError> {
    if project.scenes.is_empty() {
        return Err(LoaderError::NoScenes);
    }

    if let Some(id) = requested {
        if project.scenes.contains_key(id) {
            return Ok(id.to_string());
        }
        return Err(LoaderError::SceneNotFound(id.to_string()));
    }

    let mut ids: Vec<&String> = project.scenes.keys().collect();
    ids.sort();
    Ok(ids[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::WindowSettings;

    fn write_project(dir: &Path) {
        let mut project = Project {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "0.1.0".to_string(),
            window: WindowSettings::default(),
            ..Project::default()
        };
        project.scenes.insert(
            "main".to_string(),
            kiln_core::Scene {
                id: "main".to_string(),
                name: "Main".to_string(),
                entities: vec![kiln_core::Entity {
                    id: "e1".to_string(),
                    name: "Player".to_string(),
                    components: vec![kiln_core::Component::Transform {
                        x: 0.0,
                        y: 0.0,
                        rotation: 0.0,
                        scale: 1.0,
                    }],
                }],
                nested_scenes: Vec::new(),
            },
        );
        let toml = project.to_toml().expect("serialize");
        std::fs::write(dir.join("kiln.toml"), toml).expect("write manifest");
    }

    #[test]
    fn loads_project_directory() {
        let dir = std::env::temp_dir().join("kiln-player-loader-dir");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        write_project(&dir);
        std::fs::write(dir.join("hero.png"), b"png-bytes").expect("write asset");

        let loaded = load(&dir, None).expect("load");
        assert_eq!(loaded.project.name, "Test");
        assert_eq!(loaded.assets.len(), 1);
        assert!(loaded.assets.contains_key("hero.png"));
    }

    #[test]
    fn missing_manifest_is_an_error() {
        let dir = std::env::temp_dir().join("kiln-player-loader-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");

        let err = load(&dir, None).expect_err("should fail");
        assert!(matches!(err, LoaderError::ProjectFileNotFound));
    }

    #[test]
    fn project_without_scenes_is_an_error() {
        let dir = std::env::temp_dir().join("kiln-player-loader-noscenes");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("kiln.toml"),
            r#"id = "test"
name = "Test"
version = "0.1.0"
features = []
scenes = {}
"#,
        )
        .expect("write manifest");

        let err = load(&dir, None).expect_err("should fail");
        assert!(matches!(err, LoaderError::NoScenes));
    }
}
