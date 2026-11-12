//! Native, bounded KilnScript editing using the runtime compiler.
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MAX_SOURCE: usize = 1_048_576;
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
pub(crate) struct ScriptEditor {
    root: Option<PathBuf>,
    path: String,
    saved: String,
    source: String,
    message: String,
    valid: bool,
}

fn resource(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.extension().and_then(|ext| ext.to_str()) != Some("ks")
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
    if std::fs::metadata(&target).map_err(|e| e.to_string())?.len() > MAX_SOURCE as u64 {
        return Err("Script exceeds 1 MiB".into());
    }
    let source = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
    if source.len() > MAX_SOURCE {
        return Err("Script exceeds 1 MiB".into());
    }
    Ok(source)
}

impl ScriptEditor {
    pub(crate) fn dirty(&self) -> bool {
        self.source != self.saved
    }

    pub(crate) fn require_saved(&self) -> Result<(), String> {
        if self.dirty() {
            Err("Save the script or choose Discard Edits in Script Editor first.".into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn open(&mut self, root: &Path, path: &str) -> Result<(), String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if self.root.as_ref() == Some(&root) && self.path == path {
            return Ok(());
        }
        self.require_saved()?;
        let source = read(&root, path)?;
        self.saved = source.clone();
        self.source = source;
        self.root = Some(root);
        self.path = path.into();
        self.message = "Edit, Validate, then Save Script. Press Play to run saved code.".into();
        self.valid = true;
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
        if self.source.len() > MAX_SOURCE {
            return Err("Script exceeds 1 MiB".into());
        }
        let mut project = kiln_core::Project::default();
        project.scenes.insert(
            "validate".into(),
            kiln_core::Scene {
                id: "validate".into(),
                name: "Validate".into(),
                script: Some(self.path.clone()),
                entities: vec![],
                nested_scenes: vec![],
            },
        );
        kiln_runtime::SliceSession::with_scene_sources(&project, "validate", |path| {
            if path == self.path {
                Ok(self.source.clone())
            } else {
                read(root, path)
            }
        })
        .map(|_| ())
    }

    fn save(&mut self) -> Result<(), String> {
        self.validate()?;
        let root = self.root.as_ref().ok_or("Open a script first")?;
        if read(root, &self.path)? != self.saved {
            return Err("File changed on disk. Your edits are preserved; reconcile the external file before saving.".into());
        }
        let target = resource(root, &self.path)?;
        let mut temp = tempfile::NamedTempFile::new_in(target.parent().ok_or("Missing directory")?)
            .map_err(|e| e.to_string())?;
        temp.write_all(self.source.as_bytes())
            .map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(target).map_err(|e| e.to_string())?;
        self.saved = self.source.clone();
        Ok(())
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, locked: bool) {
        ui.heading("Script Editor");
        if self.root.is_none() {
            ui.add_space(16.0);
            ui.label("Give a character behavior");
            ui.label("1. Select your character in the Hierarchy.");
            ui.label("2. In the Inspector, choose Create Movement Script or Attach Script.");
            ui.label("3. Edit code here, save it, then press Play and use arrows or WASD.");
            return;
        }
        ui.label(
            egui::RichText::new(format!(
                "{}{}",
                self.path,
                if self.dirty() { "  • Unsaved" } else { "" }
            ))
            .monospace(),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!locked && self.dirty(), egui::Button::new("Save Script")).clicked() {
                let result = self.save(); self.valid = result.is_ok();
                self.message = result.err().unwrap_or_else(|| "Script saved. Press Play to run the updated code.".into());
            }
            if ui.button("Validate").clicked() {
                let result = self.validate(); self.valid = result.is_ok();
                self.message = result.err().unwrap_or_else(|| "Compilation passed (including imports). Runtime behavior is checked in Play.".into());
            }
            if ui.add_enabled(!locked && self.dirty(), egui::Button::new("Discard Edits")).clicked() {
                self.source = self.saved.clone(); self.message = "Unsaved edits discarded.".into(); self.valid = true;
            }
            if ui.add_enabled(!self.dirty() && !locked, egui::Button::new("Reload from Disk")).clicked() {
                match read(self.root.as_ref().expect("Open resource"), &self.path) {
                    Ok(source) => { self.source = source.clone(); self.saved = source; self.message = "Reloaded.".into(); self.valid = true; }
                    Err(error) => { self.message = error; self.valid = false; }
                }
            }
        });
        ui.small("KilnScript (.ks)  ·  Changes run after Save and the next Play  ·  Speed is set in the Inspector");
        ui.separator();
        ui.colored_label(
            if self.valid {
                egui::Color32::from_rgb(152, 180, 166)
            } else {
                egui::Color32::LIGHT_RED
            },
            &self.message,
        );
        let editor = egui::ScrollArea::both()
            .id_salt("script_source")
            .show(ui, |ui| {
                ui.add_enabled(
                    !locked,
                    egui::TextEdit::multiline(&mut self.source)
                        .code_editor()
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(28)
                        .char_limit(MAX_SOURCE),
                )
            })
            .inner;
        if !locked
            && editor.has_focus()
            && ui.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::S,
                ))
            })
        {
            let result = self.save();
            self.valid = result.is_ok();
            self.message = result
                .err()
                .unwrap_or_else(|| "Script saved. Press Play to run updated code.".into());
        }
    }
}

#[cfg(test)]
mod tests {
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
