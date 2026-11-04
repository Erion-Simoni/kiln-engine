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

/// Save another manifest while preserving all paths relative to the project root.
/// A different destination copies the project directory, refusing to merge into
/// a nonempty destination so unrelated files can never be overwritten.
pub fn save_as(project: &Project, source: &Path, destination: &Path) -> Result<PathBuf, String> {
    let root = source
        .parent()
        .ok_or("The project has no asset directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination_parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let destination_parent = canonicalize_destination(destination_parent)?;
    if root == destination_parent {
        return save(project, destination);
    }
    if destination_parent.starts_with(&root) {
        return Err("Save As destination cannot be inside the current project directory".into());
    }
    let target_root = destination_parent.clone();
    if target_root.exists()
        && std::fs::read_dir(&target_root)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("Save As will not overwrite files in a nonempty destination directory".into());
    }

    let name = destination
        .file_name()
        .ok_or("Choose a project file name")?;
    let staging_parent = target_root
        .parent()
        .ok_or("Choose a project destination directory")?;
    std::fs::create_dir_all(staging_parent).map_err(|e| e.to_string())?;
    let staging = tempfile::Builder::new()
        .prefix(".kiln-save-as-")
        .tempdir_in(staging_parent)
        .map_err(|e| e.to_string())?;
    copy_project_directory(&root, source, staging.path())?;
    let staged_manifest = staging.path().join(name);
    save(project, &staged_manifest)?;

    let target_was_empty = target_root.is_dir();
    if target_was_empty {
        std::fs::remove_dir(&target_root).map_err(|e| e.to_string())?;
    }
    let staged_root = staging.keep();
    if let Err(error) = std::fs::rename(&staged_root, &target_root) {
        let _ = std::fs::remove_dir_all(&staged_root);
        if target_was_empty {
            let _ = std::fs::create_dir(&target_root);
        }
        return Err(error.to_string());
    }
    Ok(target_root.join(name))
}

fn canonicalize_destination(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    while !ancestor.exists() {
        missing.push(
            ancestor
                .file_name()
                .ok_or("Choose a project destination directory")?
                .to_os_string(),
        );
        ancestor = ancestor
            .parent()
            .ok_or("Choose a project destination directory")?;
    }
    let mut canonical = ancestor.canonicalize().map_err(|e| e.to_string())?;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

fn copy_project_directory(
    source_root: &Path,
    source_manifest: &Path,
    destination: &Path,
) -> Result<(), String> {
    let source_manifest = source_manifest.canonicalize().map_err(|e| e.to_string())?;
    copy_directory_contents(source_root, &source_manifest, destination)
}

fn copy_directory_contents(
    source: &Path,
    source_manifest: &Path,
    destination: &Path,
) -> Result<(), String> {
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if path == source_manifest
            || matches!(name_text.as_ref(), ".git" | "target" | ".kiln-save")
            || (name_text.starts_with('.')
                && (name_text.ends_with(".recovery.json") || name_text.ends_with(".recovery.tmp")))
        {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        let target = destination.join(&name);
        if kind.is_symlink() {
            return Err(format!(
                "Save As cannot copy a project symlink: {}",
                path.display()
            ));
        } else if kind.is_dir() {
            std::fs::create_dir(&target).map_err(|e| e.to_string())?;
            copy_directory_contents(&path, source_manifest, &target)?;
        } else if kind.is_file() {
            std::fs::copy(&path, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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
