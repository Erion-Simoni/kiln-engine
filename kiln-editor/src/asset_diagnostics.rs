//! Read-only dependency/content diagnostics away from the editor render thread.
use kiln_core::Project;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    thread::JoinHandle,
};
#[derive(Default)]
pub(crate) struct AssetDiagnostics {
    task: Option<JoinHandle<Result<Vec<String>, String>>>,
    pub lines: Vec<String>,
}
impl AssetDiagnostics {
    pub fn running(&self) -> bool {
        self.task.is_some()
    }
    pub fn start(&mut self, project: Project, root: PathBuf) {
        if self.running() {
            return;
        }
        self.lines = vec!["Checking project dependencies and content hashes…".into()];
        self.task = Some(std::thread::spawn(move || inspect(&project, &root)));
    }
    pub fn poll(&mut self) -> bool {
        if self.task.as_ref().is_some_and(|task| task.is_finished()) {
            self.lines = match self.task.take().expect("finished task").join() {
                Ok(Ok(lines)) => lines,
                Ok(Err(error)) => vec![error],
                Err(_) => vec!["Asset diagnostic worker failed".into()],
            };
            return true;
        }
        false
    }
}
fn inspect(project: &Project, root: &Path) -> Result<Vec<String>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let dependencies = project.resource_dependencies();
    let mut paths = dependencies
        .keys()
        .chain(project.asset_ids.keys())
        .cloned()
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let mut lines = Vec::new();
    let mut identities = BTreeMap::<&str, Vec<&str>>::new();
    for (path, id) in &project.asset_ids {
        identities
            .entry(id.as_str())
            .or_default()
            .push(path.as_str());
    }
    for (id, mut paths) in identities {
        if paths.len() > 1 {
            paths.sort();
            lines.push(format!("Duplicate asset ID {id}: {}", paths.join(", ")));
        }
    }
    let mut cache = kiln_assets::AssetCache::new(root.join(".kiln-cache"));
    for path in &paths {
        let owners = dependencies
            .get(path)
            .map(|owners| owners.join(", "))
            .unwrap_or_else(|| "asset registry".into());
        let full = root.join(path);
        match full.canonicalize() {
            Ok(resolved) if resolved.starts_with(&root) && resolved.is_file() => {
                if let Err(error) = cache.import(&resolved) {
                    lines.push(format!("Cannot inspect {path}: {error}"));
                }
            }
            Ok(_) => lines.push(format!("Unsafe/non-file resource {path} used by {owners}")),
            Err(_) => lines.push(format!("Missing {path} used by {owners}")),
        }
    }
    for group in cache.duplicate_assets() {
        let mut names = group
            .into_iter()
            .filter_map(|path| {
                path.strip_prefix(&root)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>();
        names.sort();
        lines.push(format!(
            "Duplicate content (retained): {}",
            names.join(", ")
        ));
    }
    lines.sort();
    if lines.is_empty() {
        lines.push(format!(
            "Checked {} resources: no missing references or duplicates",
            paths.len()
        ));
    }
    Ok(lines)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_name_dependents_and_duplicate_id_content_without_modifying_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.png"), b"same art").unwrap();
        std::fs::write(dir.path().join("b.png"), b"same art").unwrap();
        let mut project = Project::default();
        project.asset_ids.insert("a.png".into(), "duplicate".into());
        project.asset_ids.insert("b.png".into(), "duplicate".into());
        project.animation_clips.insert(
            "walk".into(),
            kiln_core::Clip::new("walk".into(), "missing.png".into()),
        );
        let lines = inspect(&project, dir.path()).unwrap();
        assert!(lines.iter().any(|s| s.contains("Duplicate asset ID")));
        assert!(lines.iter().any(|s| s.contains("Duplicate content")));
        assert!(lines
            .iter()
            .any(|s| s.contains("Missing missing.png used by clip:walk")));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        assert_eq!(
            std::fs::read(dir.path().join("a.png")).unwrap(),
            b"same art"
        );
    }
}
