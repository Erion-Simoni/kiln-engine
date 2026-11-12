//! Project-relative gameplay source editing, with guarded atomic saves.
use std::{
    io::Write,
    path::{Path, PathBuf},
};

const SOURCE_LIMIT: usize = 1_048_576;

#[derive(Default)]
pub struct ScriptEditor {
    root: Option<PathBuf>,
    pub path: String,
    pub source: String,
    saved: Option<String>,
    loaded: String,
    message: String,
}

impl ScriptEditor {
    pub fn is_dirty(&self) -> bool {
        !self.loaded.is_empty() && self.saved.as_deref().unwrap_or("") != self.source
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn destination(&self, path: &str) -> Result<PathBuf, String> {
        if path.is_empty()
            || path.contains(['\\', ':'])
            || path.chars().any(char::is_control)
            || !path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
            || Path::new(path).extension().and_then(|s| s.to_str()) != Some("ks")
        {
            return Err("Choose a project-relative .ks file without traversal".into());
        }
        let root = self.root.as_ref().ok_or("Open a project first")?;
        let target = root.join(path);
        for parent in Path::new(path).ancestors().skip(1) {
            let metadata =
                std::fs::symlink_metadata(root.join(parent)).map_err(|e| e.to_string())?;
            if !metadata.is_dir() {
                return Err("Script parent must be a directory, not a link".into());
            }
        }
        if let Ok(metadata) = std::fs::symlink_metadata(&target) {
            if !metadata.is_file() || metadata.len() > SOURCE_LIMIT as u64 {
                return Err("Script must be a regular file of at most 1 MiB".into());
            }
        }
        Ok(target)
    }

    pub fn open(&mut self, root: &Path, path: &str) -> Result<(), String> {
        if self.is_dirty() {
            return Err("Save or Revert the current script before opening another".into());
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let previous = self.root.replace(root);
        let result =
            self.destination(path)
                .and_then(|target| match std::fs::read_to_string(target) {
                    Ok(source) => Ok(Some(source)),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(error) => Err(error.to_string()),
                });
        let source = match result {
            Ok(source) => source,
            Err(error) => {
                self.root = previous;
                return Err(error);
            }
        };
        self.source = source.clone().unwrap_or_default();
        self.saved = source;
        self.loaded = path.into();
        self.path = path.into();
        self.message =
            "Script opened. Save Script writes source; Save Project also saves this draft.".into();
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), String> {
        if self.loaded.is_empty() {
            return Ok(());
        }
        if self.source.len() > SOURCE_LIMIT {
            return Err("Script exceeds 1 MiB".into());
        }
        let target = self.destination(&self.loaded)?;
        let disk = match std::fs::read_to_string(&target) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if disk != self.saved {
            return Err("Script changed outside the editor. Copy your draft, Revert, then merge the changes.".into());
        }
        let mut file =
            tempfile::NamedTempFile::new_in(target.parent().ok_or("Script parent missing")?)
                .map_err(|e| e.to_string())?;
        file.write_all(self.source.as_bytes())
            .map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(target).map_err(|e| e.to_string())?;
        self.saved = Some(self.source.clone());
        self.message = "Script saved".into();
        Ok(())
    }

    fn revert(&mut self) -> Result<(), String> {
        let root = self.root.clone().ok_or("Open a project first")?;
        let path = self.loaded.clone();
        let draft = std::mem::take(&mut self.source);
        self.source = self.saved.clone().unwrap_or_default();
        if let Err(error) = self.open(&root, &path) {
            self.source = draft;
            return Err(error);
        }
        Ok(())
    }

    pub fn render(&mut self, ui: &mut egui::Ui, root: &Path, default_path: &str) {
        if self.root.is_none() {
            if let Err(error) = self.open(root, default_path) {
                self.message = error;
            }
        }
        ui.horizontal(|ui| {
            ui.label("Script:");
            ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(260.0));
            if ui.button("Open / New").clicked() {
                let path = self.path.clone();
                if let Err(error) = self.open(root, &path) {
                    self.message = error;
                }
            }
        });
        ui.horizontal(|ui| {
            let shortcut =
                ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S));
            if ui.button("Save Script").clicked() || shortcut {
                if let Err(error) = self.save() {
                    self.message = error;
                }
            }
            if ui.button("Revert").clicked() {
                if let Err(error) = self.revert() {
                    self.message = error;
                }
            }
            if ui.button("Check Syntax").clicked() {
                self.message = match kiln_runtime::SliceSession::new(&self.source) {
                    Ok(_) => "Syntax OK (Play/Export also validate imports and resources)".into(),
                    Err(errors) => errors
                        .iter()
                        .map(|e| {
                            format!(
                                "{}:{}: {}",
                                self.loaded,
                                self.source
                                    .get(..e.span.start.min(self.source.len()))
                                    .unwrap_or("")
                                    .bytes()
                                    .filter(|b| *b == b'\n')
                                    .count()
                                    + 1,
                                e.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                };
            }
            if self.is_dirty() {
                ui.colored_label(egui::Color32::YELLOW, "Unsaved script");
            }
        });
        ui.label(&self.message);
        egui::ScrollArea::both()
            .id_salt("gameplay_script_source")
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.source)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(28),
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_are_atomic_and_conflicts_preserve_the_draft_and_disk() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("scripts")).unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), "scripts/main.ks").unwrap();
        editor.source = "// Héllo\nlet x = 1;\n".into();
        assert!(editor.is_dirty());
        editor.save().unwrap();
        assert!(!editor.is_dirty());
        editor.source = "let x = 2;".into();
        assert!(editor.open(root.path(), "scripts/other.ks").is_err());
        std::fs::write(root.path().join("scripts/main.ks"), "external").unwrap();
        assert!(editor.save().is_err());
        assert!(editor.is_dirty());
        assert_eq!(
            std::fs::read_to_string(root.path().join("scripts/main.ks")).unwrap(),
            "external"
        );
        editor.revert().unwrap();
        assert_eq!(editor.source, "external");
    }

    #[test]
    fn traversal_links_and_oversized_sources_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let mut editor = ScriptEditor::default();
        for path in [
            "../outside.ks",
            "/outside.ks",
            "C:/outside.ks",
            "a\\b.ks",
            "main.toml",
        ] {
            assert!(editor.open(root.path(), path).is_err());
        }
        editor.open(root.path(), "main.ks").unwrap();
        editor.source = "x".repeat(SOURCE_LIMIT + 1);
        assert!(editor.save().is_err());
        assert!(!root.path().join("main.ks").exists());
        #[cfg(unix)]
        {
            editor.reset();
            std::os::unix::fs::symlink(root.path(), root.path().join("linked")).unwrap();
            assert!(editor.open(root.path(), "linked/main.ks").is_err());
        }
    }
}
