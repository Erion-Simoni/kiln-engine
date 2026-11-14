//! Project-relative gameplay source editing, with guarded atomic saves.
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

const SOURCE_LIMIT: usize = 1_048_576;

pub(crate) const MOVEMENT: &str = r#"// Character movement: arrows or WASD. Set speed in the Inspector.
// This moves a Transform directly; it is not a collision-based controller.
if script_phase() == "update" {
    let position = get_entity_pos(self_entity());
    let speed = get_property("speed");
    let dt = get_delta_time();
    set_entity_pos(self_entity(),
        array_get(position, 0) + get_input_x() * speed * dt,
        array_get(position, 1) + get_input_y() * speed * dt);
}
"#;

#[derive(Default)]
pub struct ScriptEditor {
    root: Option<PathBuf>,
    pub path: String,
    pub source: String,
    saved: Option<String>,
    loaded: String,
    message: String,
    show_api: bool,
    api_filter: String,
    highlighted_source: String,
    highlighted_job: egui::text::LayoutJob,
}

/// Unsaved text and the disk baseline used by guarded saves after recovery.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ScriptDraft {
    path: String,
    source: String,
    saved: Option<String>,
}

fn resource(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.contains(['\\', ':'])
        || relative.chars().any(char::is_control)
        || !relative
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        || path.extension().and_then(|ext| ext.to_str()) != Some("ks")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Choose a project-relative .ks file".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut target = root.clone();
    for part in path.components() {
        target.push(part);
        if target.exists() {
            let resolved = target.canonicalize().map_err(|e| e.to_string())?;
            if !resolved.starts_with(&root) {
                return Err("Script path leaves the project".into());
            }
            if std::fs::symlink_metadata(&target)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Script paths cannot contain symbolic links".into());
            }
        }
    }
    Ok(target)
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    let target = resource(root, path)?;
    if std::fs::metadata(&target).map_err(|e| e.to_string())?.len() > SOURCE_LIMIT as u64 {
        return Err("Script exceeds 1 MiB".into());
    }
    let source = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
    if source.len() > SOURCE_LIMIT {
        return Err("Script exceeds 1 MiB".into());
    }
    Ok(source)
}

impl ScriptEditor {
    pub(crate) fn recovery_draft(&self) -> Option<ScriptDraft> {
        self.is_dirty().then(|| ScriptDraft {
            path: self.loaded.clone(),
            source: self.source.clone(),
            saved: self.saved.clone(),
        })
    }

    /// Restore only the buffer; source files remain untouched until explicit Save.
    pub(crate) fn from_recovery(root: &Path, draft: &ScriptDraft) -> Result<Self, String> {
        if draft.source.len() > SOURCE_LIMIT
            || draft.saved.as_ref().is_some_and(|s| s.len() > SOURCE_LIMIT)
        {
            return Err("Recovered script exceeds 1 MiB".into());
        }
        let mut editor = Self {
            root: Some(root.canonicalize().map_err(|e| e.to_string())?),
            ..Self::default()
        };
        let target = editor.destination(&draft.path)?;
        let disk = match std::fs::read_to_string(target) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        editor.path = draft.path.clone();
        editor.loaded = draft.path.clone();
        editor.source = draft.source.clone();
        editor.saved = if disk.as_deref() == Some(draft.source.as_str()) {
            disk
        } else {
            draft.saved.clone()
        };
        editor.message = "Recovered script text. Save Script or Save Project to write it.".into();
        Ok(editor)
    }

    pub(crate) fn dirty(&self) -> bool {
        self.is_dirty()
    }
    pub(crate) fn require_saved(&self) -> Result<(), String> {
        if self.is_dirty() {
            Err("Save or Revert the script before switching files or creating movement code".into())
        } else {
            Ok(())
        }
    }

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
            if self.loaded == path && self.root.as_deref() == root.canonicalize().ok().as_deref() {
                self.path = self.loaded.clone();
                return Ok(());
            }
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

    pub(crate) fn create_movement(root: &Path) -> Result<String, String> {
        let target = resource(root, "scripts/character_movement.ks")?;
        std::fs::create_dir_all(target.parent().ok_or("Missing script directory")?)
            .map_err(|e| e.to_string())?;
        for index in 0..1000 {
            let path = if index == 0 {
                "scripts/character_movement.ks".into()
            } else {
                format!("scripts/character_movement_{index}.ks")
            };
            let target = resource(root, &path)?;
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
            {
                Ok(mut file) => {
                    file.write_all(MOVEMENT.as_bytes())
                        .map_err(|e| e.to_string())?;
                    return Ok(path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("Too many movement scripts; attach an existing file instead".into())
    }

    fn validate(&self) -> Result<(), String> {
        let root = self.root.as_ref().ok_or("Open a script first")?;
        if self.source.len() > SOURCE_LIMIT {
            return Err("Script exceeds 1 MiB".into());
        }
        let mut project = kiln_core::Project::default();
        project.scenes.insert(
            "validate".into(),
            kiln_core::Scene {
                id: "validate".into(),
                name: "Validate".into(),
                script: Some(self.loaded.clone()),
                entities: vec![],
                nested_scenes: vec![],
            },
        );
        kiln_runtime::SliceSession::with_scene_sources(&project, "validate", |path| {
            if path == self.loaded {
                Ok(self.source.clone())
            } else {
                read(root, path)
            }
        })
        .map(|_| ())
    }

    pub fn save(&mut self) -> Result<(), String> {
        if self.loaded.is_empty() {
            return Ok(());
        }
        if self.source.len() > SOURCE_LIMIT {
            return Err("Script exceeds 1 MiB".into());
        }
        self.validate()?;
        let target = self.destination(&self.loaded)?;
        let disk = match std::fs::read_to_string(&target) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if disk != self.saved {
            return Err("Script changed on disk outside the editor. Copy your draft, Revert, then merge the changes.".into());
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
                self.message = match self.validate() {
                    Ok(()) => "Syntax OK (including imports; Play checks runtime behavior)".into(),
                    Err(error) => error,
                };
            }
            if ui.button("API Reference").clicked() {
                self.show_api = !self.show_api;
            }
            if self.is_dirty() {
                ui.colored_label(egui::Color32::YELLOW, "Unsaved script");
            }
        });
        ui.label(&self.message);
        let highlighted_source = &mut self.highlighted_source;
        let highlighted_job = &mut self.highlighted_job;
        let mut layouter = |ui: &egui::Ui, code: &str, wrap_width: f32| {
            if highlighted_source != code {
                *highlighted_source = code.to_owned();
                *highlighted_job = crate::script_highlight::highlight(code);
            }
            let mut job = highlighted_job.clone();
            job.wrap.max_width = wrap_width;
            ui.fonts(|fonts| fonts.layout_job(job))
        };
        egui::ScrollArea::both()
            .id_salt("gameplay_script_source")
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.source)
                        .code_editor()
                        .layouter(&mut layouter)
                        .char_limit(SOURCE_LIMIT)
                        .desired_width(f32::INFINITY)
                        .desired_rows(28),
                );
            });
        egui::Window::new("KilnScript API Reference").open(&mut self.show_api).show(ui.ctx(),|ui| {
            ui.label("Built-ins shared by scripts, graphs, native and browser games.");
            ui.horizontal(|ui| {ui.label("Filter:");ui.text_edit_singleline(&mut self.api_filter);});
            ui.label("Signatures show required argument counts. See docs/KILNSCRIPT.md for semantics.");
            let filter=self.api_filter.to_lowercase();
            let mut entries=kiln_runtime::slice::BUILTINS.to_vec();entries.sort_by_key(|entry|entry.0);
            egui::ScrollArea::vertical().max_height(350.0).show(ui,|ui| {
                for (name,_,count) in entries.into_iter().filter(|(name,_,_)|name.contains(&filter)) {
                    let arguments=(1..=count).map(|i|format!("arg{i}")).collect::<Vec<_>>().join(", ");
                    ui.monospace(format!("{name}({arguments})"));
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_uses_loaded_path_and_restores_new_file_without_writing_it() {
        let root = tempfile::tempdir().unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), "new.ks").unwrap();
        editor.source = "let value = 7;".into();
        editor.path = "unopened-text-field.ks".into();
        let draft = editor.recovery_draft().unwrap();
        let mut restored = ScriptEditor::from_recovery(root.path(), &draft).unwrap();
        assert_eq!(restored.path, "new.ks");
        assert!(restored.is_dirty());
        assert!(!root.path().join("new.ks").exists());
        restored.save().unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join("new.ks")).unwrap(),
            "let value = 7;"
        );
        assert!(restored.recovery_draft().is_none());
        let already_saved = ScriptEditor::from_recovery(root.path(), &draft).unwrap();
        assert!(!already_saved.is_dirty());
    }

    #[test]
    fn invalid_recovery_paths_and_oversized_text_are_rejected_without_writes() {
        let root = tempfile::tempdir().unwrap();
        let mut draft = ScriptDraft {
            path: "../outside.ks".into(),
            source: "draft".into(),
            saved: None,
        };
        assert!(ScriptEditor::from_recovery(root.path(), &draft).is_err());
        draft.path = "safe.ks".into();
        draft.source = "x".repeat(SOURCE_LIMIT + 1);
        assert!(ScriptEditor::from_recovery(root.path(), &draft).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
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
        editor.open(root.path(), "scripts/main.ks").unwrap();
        assert_eq!(editor.source, "let x = 2;");
        assert!(editor.is_dirty());
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

#[cfg(test)]
mod upstream_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn movement_creation_never_overwrites_existing_files_and_moves_attached_entity() {
        let root = tempfile::tempdir().unwrap();
        let path = ScriptEditor::create_movement(root.path()).unwrap();
        let second = ScriptEditor::create_movement(root.path()).unwrap();
        assert_ne!(path, second);
        assert_eq!(read(root.path(), &path).unwrap(), MOVEMENT);
        let mut project = kiln_core::Project::default();
        project.scenes.insert(
            "main".into(),
            kiln_core::Scene {
                id: "main".into(),
                name: "Main".into(),
                script: None,
                nested_scenes: vec![],
                entities: vec![kiln_core::Entity {
                    id: "hero".into(),
                    name: "Hero".into(),
                    components: vec![kiln_core::Component::Script {
                        path: path.clone(),
                        properties: HashMap::from([("speed".into(), 180.0)]),
                    }],
                }],
            },
        );
        let mut world = kiln_runtime::GameWorld::new();
        let hero = world.world_mut().spawn((
            kiln_runtime::SourceId::new("hero"),
            kiln_core::Transform::new(10.0, 20.0),
        ));
        let mut session =
            kiln_runtime::SliceSession::with_scene_sources(&project, "main", |path| {
                read(root.path(), path)
            })
            .unwrap();
        session.step(&mut world, (1.0, 0.0), 0.1).unwrap();
        let position = world
            .world_ref()
            .get::<&kiln_core::Transform>(hero)
            .unwrap();
        assert!((position.position.x - 28.0).abs() < 0.001);
        assert!((position.position.y - 20.0).abs() < 0.001);
    }

    #[test]
    fn script_saves_validate_and_preserve_external_changes_and_dirty_buffers() {
        let root = tempfile::tempdir().unwrap();
        let path = ScriptEditor::create_movement(root.path()).unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), &path).unwrap();
        editor.source.push_str("\n// edited\n");
        assert!(editor.require_saved().is_err());
        assert!(editor.open(root.path(), "other.ks").is_err());
        editor.save().unwrap();
        assert!(!editor.dirty());
        assert_eq!(read(root.path(), &path).unwrap(), editor.source);
        editor.source.push_str("\n// second edit\n");
        std::fs::write(root.path().join(&path), "// external edit").unwrap();
        assert!(editor.save().unwrap_err().contains("changed on disk"));
        assert!(editor.dirty());
        assert_eq!(read(root.path(), &path).unwrap(), "// external edit");
    }

    #[test]
    fn validation_reports_locations_and_resolves_imports_from_project() {
        let root = tempfile::tempdir().unwrap();
        let path = ScriptEditor::create_movement(root.path()).unwrap();
        let mut editor = ScriptEditor::default();
        editor.open(root.path(), &path).unwrap();
        std::fs::write(
            root.path().join("scripts/helper.ks"),
            "fn speed() { let x = 0.0; while x < 3.0 { x += 1.0; } if !false && x == 3.0 { return x; } else if false { return 0; } return 1; }",
        )
        .unwrap();
        editor.source = "import \"helper.ks\";\nlet x = speed();".into();
        editor.validate().unwrap();
        editor.source = "let broken = ;".into();
        assert!(editor
            .save()
            .unwrap_err()
            .contains("character_movement.ks:1:"));
        assert_eq!(read(root.path(), &path).unwrap(), MOVEMENT);
        assert!(resource(root.path(), "../outside.ks").is_err());
        assert!(resource(root.path(), "scripts/file.txt").is_err());
    }
}
