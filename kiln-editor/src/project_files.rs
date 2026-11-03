//! Project file operations shared by editor UI and headless acceptance tests.

use kiln_core::{Project, Scene};
use std::io::Write;
use std::path::{Path, PathBuf};

/// A successfully loaded document and its canonical manifest location.
pub struct LoadedProject {
    pub project: Project,
    pub path: PathBuf,
}

/// Accept a project directory or a manifest file, without changing editor state.
pub fn load(path: &Path) -> Result<LoadedProject, String> {
    let path = if path.is_dir() {
        path.join("kiln.toml")
    } else {
        path.to_path_buf()
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let project = Project::from_toml(&source).map_err(|e| e.to_string())?;
    Ok(LoadedProject { project, path })
}

/// Serialize to a sibling temporary file, then replace the manifest atomically.
/// A failure before replacement leaves the previous manifest untouched.
pub fn save(project: &Project, path: &Path) -> Result<PathBuf, String> {
    let source = project.to_toml().map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Choose a project file name")?;
    let path = parent.join(name);
    let mut temporary = tempfile::NamedTempFile::new_in(&parent).map_err(|e| e.to_string())?;
    if let Ok(metadata) = std::fs::metadata(&path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
    }
    temporary
        .write_all(source.as_bytes())
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Save a second manifest without changing the root used by relative assets.
pub fn save_as(project: &Project, source: &Path, destination: &Path) -> Result<PathBuf, String> {
    let root = source
        .parent()
        .ok_or("The project has no asset directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let target_root = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if root != target_root {
        return Err("Save As must use the current project directory so relative assets remain valid. Project relocation is not supported yet.".into());
    }
    save(project, destination)
}

/// Create a project in a new directory. Existing projects/files are never overwritten.
pub fn create(name: &str, destination: &Path, template: &str) -> Result<LoadedProject, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a project name".into());
    }
    if name == "."
        || name == ".."
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:<>|?*".contains(c))
    {
        return Err(
            "Use a project name without path separators or reserved filename characters".into(),
        );
    }
    if !["empty_2d", "platformer", "topdown"].contains(&template) {
        return Err(format!("Unknown template: {template}"));
    }
    if destination.exists() {
        return Err("Choose a new project directory; the destination already exists".into());
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let directory = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or("Choose a new project directory")?;
    let staging = tempfile::Builder::new()
        .prefix(".kiln-create-")
        .tempdir_in(&parent)
        .map_err(|e| e.to_string())?;
    std::fs::create_dir(staging.path().join("art")).map_err(|e| e.to_string())?;
    std::fs::create_dir(staging.path().join("scripts")).map_err(|e| e.to_string())?;
    std::fs::write(
        staging.path().join("scripts/main.ks"),
        "// Runs once per frame. Add your game rules here.\nlet dt = get_delta_time();\n",
    )
    .map_err(|e| e.to_string())?;
    let mut project = Project {
        name: name.into(),
        ..Default::default()
    };
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("scripts/main.ks".into()),
            entities: Vec::new(),
            nested_scenes: Vec::new(),
        },
    );
    save(&project, &staging.path().join("kiln.toml"))?;
    let destination = parent.join(directory);
    std::fs::rename(staging.path(), &destination).map_err(|e| e.to_string())?;
    load(&destination)
}
