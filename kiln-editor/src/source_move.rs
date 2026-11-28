//! Guarded source-import edits accompanying a project asset move.
use crate::assets::AssetBrowser;
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

struct Edit {
    path: String,
    original: Vec<u8>,
    replacement: String,
}

fn resolve(importer: &str, module: &str) -> Result<String, String> {
    if module.contains(['\\', ':', '"']) || module.chars().any(char::is_control) {
        return Err("Import paths must be portable project paths".into());
    }
    let mut parts: Vec<_> = importer.split('/').collect();
    parts.pop();
    for part in Path::new(module).components() {
        match part {
            Component::Normal(value) => parts.push(value.to_str().ok_or("Non-UTF8 import")?),
            Component::CurDir => {}
            Component::ParentDir if !parts.is_empty() => {
                parts.pop();
            }
            _ => return Err("Import escapes the project".into()),
        }
    }
    Ok(parts.join("/"))
}

fn relative(importer: &str, target: &str) -> String {
    let mut folder: Vec<_> = importer.split('/').collect();
    folder.pop();
    let target: Vec<_> = target.split('/').collect();
    let shared = folder
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts = vec![".."; folder.len() - shared];
    parts.extend_from_slice(&target[shared..]);
    parts.join("/")
}

// Match the runtime's top-level, line-based import grammar. Track strings,
// escapes, comments and function braces so ordinary source text stays exact.
fn rewrite(source: &str, path: &str, old: &str, new: &str) -> Result<String, String> {
    let final_path = if path == old { new } else { path };
    let mut output = String::new();
    let mut quoted = false;
    let mut braces = 0_i32;
    for (number, line) in source.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        if !quoted && braces == 0 && trimmed.starts_with("import ") {
            let module = trimmed
                .strip_prefix("import \"")
                .and_then(|s| s.strip_suffix("\";"))
                .ok_or_else(|| {
                    format!("{path}:{}: expected import \"relative.ks\";", number + 1)
                })?;
            let target =
                resolve(path, module).map_err(|e| format!("{path}:{}: {e}", number + 1))?;
            if target == old || path == old {
                let target = if target == old { new } else { &target };
                let replacement = relative(final_path, target);
                let start = line.find("import \"").ok_or("Missing import prefix")? + 8;
                output.push_str(&line[..start]);
                output.push_str(&replacement);
                output.push_str(&line[start + module.len()..]);
            } else {
                output.push_str(line);
            }
        } else {
            output.push_str(line);
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if quoted && c == '\\' {
                    chars.next();
                    continue;
                }
                if c == '"' {
                    quoted = !quoted;
                } else if !quoted {
                    if c == '/' && chars.peek() == Some(&'/') {
                        break;
                    }
                    if c == '{' {
                        braces += 1;
                    }
                    if c == '}' {
                        braces -= 1;
                    }
                }
            }
        }
    }
    Ok(output)
}

fn gather(
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    remaining: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > 16 {
        return Err("Source move exceeds the directory depth limit".into());
    }
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        if *remaining == 0 {
            return Err("Source move exceeds 4096 directory entries".into());
        }
        *remaining -= 1;
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || ["target", "export", "dist"].contains(&name.as_ref()) {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("Source moves require project files without symbolic links".into());
        }
        if kind.is_dir() {
            gather(&entry.path(), paths, remaining, depth + 1)?;
        } else if kind.is_file() && entry.path().extension().is_some_and(|e| e == "ks") {
            if paths.len() >= 512 {
                return Err("Source move exceeds 512 scripts".into());
            }
            paths.push(entry.path());
        }
    }
    Ok(())
}

fn prepare(root: &Path, old: &str, new: &str) -> Result<Vec<Edit>, String> {
    let mut paths = Vec::new();
    gather(root, &mut paths, &mut 4096, 0)?;
    paths.sort();
    let mut total = 0;
    let mut edits = Vec::new();
    for file in paths {
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        total += length;
        if length > 1_048_576 || total > 8_388_608 {
            return Err(
                "Source move exceeds script size limits (1 MiB/file, 8 MiB/project)".into(),
            );
        }
        let mut source = String::new();
        std::fs::File::open(&file)
            .map_err(|e| e.to_string())?
            .take(1_048_577)
            .read_to_string(&mut source)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        if source.len() as u64 != length {
            return Err("A source file changed while preparing the move; try again".into());
        }
        let path = file
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("Non-UTF8 source path")?
            .replace('\\', "/");
        let replacement = rewrite(&source, &path, old, new)?;
        if replacement != source {
            edits.push(Edit {
                path,
                original: source.into_bytes(),
                replacement,
            });
        }
    }
    Ok(edits)
}

fn stage(root: &Path, edit: &Edit, bytes: &[u8]) -> Result<tempfile::NamedTempFile, String> {
    let path = root.join(&edit.path);
    let mut temporary = tempfile::Builder::new()
        .prefix(".kiln-source-move-")
        .tempfile_in(path.parent().ok_or("Missing source folder")?)
        .map_err(|e| e.to_string())?;
    temporary
        .as_file()
        .set_permissions(path.metadata().map_err(|e| e.to_string())?.permissions())
        .map_err(|e| e.to_string())?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    Ok(temporary)
}

/// Move a script and rewrite its importers and its own relative imports. All
/// edits/backups are staged first; a publication failure rolls back applied edits.
/// Non-script assets retain the ordinary asset-browser move behavior.
pub(crate) fn move_asset(
    browser: &mut AssetBrowser,
    old: &str,
    new: &str,
) -> Result<usize, String> {
    move_with(browser, old, new, |_| Ok(()))
}

fn move_with(
    browser: &mut AssetBrowser,
    old: &str,
    new: &str,
    before_publish: impl Fn(usize) -> Result<(), String>,
) -> Result<usize, String> {
    let script = |p: &str| Path::new(p).extension().is_some_and(|e| e == "ks");
    if !script(old) && !script(new) {
        browser.rename_asset(old, new)?;
        return Ok(0);
    }
    if !script(old) || !script(new) {
        return Err("Keep the .ks extension when moving a source module".into());
    }
    for path in [old, new] {
        if path.is_empty()
            || path.contains(['\\', ':', '"'])
            || path.chars().any(char::is_control)
            || !path.split('/').all(|part| {
                !part.is_empty()
                    && !part.starts_with('.')
                    && !["target", "export", "dist"].contains(&part)
            })
        {
            return Err("Source paths must be portable project-relative paths".into());
        }
    }
    // The browser validates both move paths before performing any mutation.
    let root = browser
        .root()
        .ok_or("No project root")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let edits = prepare(&root, old, new)?;
    let mut staged = Vec::new();
    for edit in &edits {
        staged.push((
            stage(&root, edit, edit.replacement.as_bytes())?,
            stage(&root, edit, &edit.original)?,
        ));
    }
    for edit in &edits {
        if std::fs::read(root.join(&edit.path)).map_err(|e| e.to_string())? != edit.original {
            return Err(format!(
                "{} changed on disk; source move cancelled",
                edit.path
            ));
        }
    }
    browser.rename_asset(old, new)?;
    let mut applied: Vec<(PathBuf, tempfile::NamedTempFile)> = Vec::new();
    for (index, (edit, (replacement, backup))) in edits.iter().zip(staged).enumerate() {
        let path = root.join(if edit.path == old { new } else { &edit.path });
        let result = before_publish(index).and_then(|_| {
            replacement
                .persist(&path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        });
        if let Err(error) = result {
            let mut failures = Vec::new();
            for (path, backup) in applied.into_iter().rev() {
                if let Err(failure) = backup.persist(&path) {
                    let retained = failure
                        .file
                        .keep()
                        .map(|(_, p)| p.display().to_string())
                        .unwrap_or_else(|e| e.to_string());
                    failures.push(format!(
                        "Could not restore {}: {}; original retained at {retained}",
                        path.display(),
                        failure.error
                    ));
                }
            }
            if let Err(rollback) = browser.rename_asset(new, old) {
                failures.push(format!("Could not restore moved path: {rollback}"));
            }
            return Err(format!(
                "Source move failed: {error}{}",
                if failures.is_empty() {
                    String::new()
                } else {
                    format!("; rollback: {}", failures.join("; "))
                }
            ));
        }
        applied.push((path, backup));
    }
    Ok(edits.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, path: &str, source: &str) {
        std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        std::fs::write(root.join(path), source).unwrap();
    }
    fn read(root: &Path, path: &str) -> String {
        std::fs::read_to_string(root.join(path)).unwrap()
    }
    fn browser(root: &Path) -> AssetBrowser {
        let mut browser = AssetBrowser::new();
        browser.scan(root);
        browser
    }
    fn validate(root: &Path, entry: &str) {
        let mut project = kiln_core::Project::default();
        project.scenes.insert(
            "main".into(),
            kiln_core::Scene {
                id: "main".into(),
                name: "Main".into(),
                script: Some(entry.into()),
                entities: vec![],
                nested_scenes: vec![],
            },
        );
        kiln_runtime::SliceSession::with_scene_sources(&project, "main", |path| {
            std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string())
        })
        .unwrap();
    }
    #[test]
    fn module_moves_rewrite_callers_and_own_imports_without_changing_other_bytes() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "scripts/main.ks",
            "// λ\r\n  import \"helper.ks\";\r\nlet result = answer();\r\n",
        );
        write(
            root.path(),
            "scripts/helper.ks",
            "import \"../shared/base.ks\";\nfn answer() { return base(); }\n",
        );
        write(root.path(), "shared/base.ks", "fn base() { return 42; }\n");
        std::fs::create_dir(root.path().join("modules")).unwrap();
        validate(root.path(), "scripts/main.ks");
        let mut browser = browser(root.path());
        assert_eq!(
            move_asset(&mut browser, "scripts/helper.ks", "modules/math.ks").unwrap(),
            1
        );
        assert_eq!(
            read(root.path(), "scripts/main.ks"),
            "// λ\r\n  import \"../modules/math.ks\";\r\nlet result = answer();\r\n"
        );
        // Its own dependency remains ../shared/base.ks from the new directory.
        assert_eq!(
            read(root.path(), "modules/math.ks"),
            "import \"../shared/base.ks\";\nfn answer() { return base(); }\n"
        );
        assert!(!root.path().join("scripts/helper.ks").exists());
        validate(root.path(), "scripts/main.ks");
        std::fs::create_dir(root.path().join("modules/deeper")).unwrap();
        assert_eq!(
            move_asset(&mut browser, "modules/math.ks", "modules/deeper/math.ks").unwrap(),
            2
        );
        assert_eq!(
            read(root.path(), "modules/deeper/math.ks"),
            "import \"../../shared/base.ks\";\nfn answer() { return base(); }\n"
        );
        validate(root.path(), "scripts/main.ks");
    }
    #[test]
    fn comments_strings_function_bodies_and_unrelated_import_spellings_stay_exact() {
        let source = "// import \"old.ks\";\nlet text = \"import \\\"old.ks\\\";\";\nfn f() {\nimport \"old.ks\";\n}\nlet multiline = \"x\nimport \\\"old.ks\\\";\nx\";\nimport \"./old.ks\";\nimport \"else.ks\";";
        let expected = source.replacen("import \"./old.ks\";", "import \"sub/new.ks\";", 1);
        assert_eq!(
            rewrite(
                source,
                "scripts/main.ks",
                "scripts/old.ks",
                "scripts/sub/new.ks"
            )
            .unwrap(),
            expected
        );
    }
    #[test]
    fn importer_moves_rebase_unchanged_dependencies_and_keep_no_final_newline() {
        assert_eq!(
            rewrite(
                "\timport \"../common.ks\";",
                "code/main.ks",
                "code/main.ks",
                "deep/code/main.ks"
            )
            .unwrap(),
            "\timport \"../../common.ks\";"
        );
        assert_eq!(
            rewrite(
                "import \"same.ks\";\n",
                "code/main.ks",
                "other/same.ks",
                "renamed.ks"
            )
            .unwrap(),
            "import \"same.ks\";\n"
        );
    }
    #[test]
    fn failed_publication_restores_all_sources_and_original_module_path() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "a.ks", "import \"module.ks\";\n");
        write(root.path(), "b.ks", "import \"module.ks\";\n");
        write(root.path(), "module.ks", "fn answer() { return 42; }\n");
        let mut browser = browser(root.path());
        let error = move_with(&mut browser, "module.ks", "renamed.ks", |index| {
            if index == 1 {
                Err("injected publication failure".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert!(error.contains("injected publication failure"));
        assert_eq!(read(root.path(), "a.ks"), "import \"module.ks\";\n");
        assert_eq!(read(root.path(), "b.ks"), "import \"module.ks\";\n");
        assert_eq!(
            read(root.path(), "module.ks"),
            "fn answer() { return 42; }\n"
        );
        assert!(!root.path().join("renamed.ks").exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
    }
    #[test]
    fn rollback_restores_a_moved_modules_own_rewritten_imports() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "base.ks", "fn base() { return 42; }\n");
        write(root.path(), "module.ks", "import \"base.ks\";\n");
        write(root.path(), "z-caller.ks", "import \"module.ks\";\n");
        std::fs::create_dir(root.path().join("sub")).unwrap();
        let error = move_with(
            &mut browser(root.path()),
            "module.ks",
            "sub/module.ks",
            |index| {
                if index == 1 {
                    Err("injected after module publication".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(error.contains("injected after module publication"));
        assert_eq!(read(root.path(), "module.ks"), "import \"base.ks\";\n");
        assert_eq!(read(root.path(), "z-caller.ks"), "import \"module.ks\";\n");
        assert!(!root.path().join("sub/module.ks").exists());
    }

    #[test]
    fn collisions_invalid_paths_extensions_and_bad_imports_do_not_mutate_files() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "main.ks", "import \"module.ks\";\n");
        write(root.path(), "module.ks", "// module\n");
        write(root.path(), "existing.ks", "// preserve\n");
        let mut browser = browser(root.path());
        for destination in [
            "existing.ks",
            "../escape.ks",
            "missing/new.ks",
            "module.txt",
        ] {
            assert!(move_asset(&mut browser, "module.ks", destination).is_err());
            assert_eq!(read(root.path(), "main.ks"), "import \"module.ks\";\n");
            assert_eq!(read(root.path(), "module.ks"), "// module\n");
            assert_eq!(read(root.path(), "existing.ks"), "// preserve\n");
        }
        write(root.path(), "main.ks", "import \"../../escape.ks\";\n");
        assert!(move_asset(&mut browser, "module.ks", "renamed.ks").is_err());
        assert!(root.path().join("module.ks").exists());
        assert!(!root.path().join("renamed.ks").exists());
    }
    #[cfg(unix)]
    #[test]
    fn symbolic_link_sources_fail_before_any_move_or_external_write() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        write(external.path(), "external.ks", "import \"module.ks\";\n");
        write(root.path(), "module.ks", "// module\n");
        std::os::unix::fs::symlink(
            external.path().join("external.ks"),
            root.path().join("main.ks"),
        )
        .unwrap();
        assert!(move_asset(&mut browser(root.path()), "module.ks", "renamed.ks").is_err());
        assert_eq!(
            read(external.path(), "external.ks"),
            "import \"module.ks\";\n"
        );
        assert!(root.path().join("module.ks").exists());
    }
}
