//! Bounded literal project-source search with safe relative navigation targets.
use std::{io::Read, path::Path};
#[derive(Clone, Debug)]
pub(crate) struct Hit {
    pub path: String,
    pub line: usize,
    pub character: usize,
    pub preview: String,
}
#[derive(Default)]
pub(crate) struct Results {
    pub hits: Vec<Hit>,
    pub limited: bool,
}

pub(crate) fn search(root: &Path, query: &str) -> Result<Results, String> {
    if query.is_empty() {
        return Ok(Results::default());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    fn gather(
        root: &Path,
        dir: &Path,
        paths: &mut Vec<String>,
        limited: &mut bool,
        remaining: &mut usize,
        depth: usize,
    ) -> Result<(), String> {
        if depth >= 64 || *remaining == 0 {
            *limited = true;
            return Ok(());
        }
        let mut entries = std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .take(*remaining + 1)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        if entries.len() > *remaining {
            entries.truncate(*remaining);
            *limited = true;
        }
        *remaining -= entries.len();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if paths.len() >= 512 {
                *limited = true;
                break;
            }
            let ty = entry.file_type().map_err(|e| e.to_string())?;
            let path = entry.path();
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with('.') && !["export", "dist", "target"].contains(&name.as_ref())
                {
                    gather(root, &path, paths, limited, remaining, depth + 1)?;
                }
            } else if ty.is_file() && path.extension().is_some_and(|ext| ext == "ks") {
                paths.push(
                    path.strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    let mut result = Results::default();
    gather(&root, &root, &mut paths, &mut result.limited, &mut 4096, 0)?;
    let mut total = 0_u64;
    for path in paths {
        let file = root.join(&path);
        let size = std::fs::metadata(&file).map_err(|e| e.to_string())?.len();
        if size > 1_048_576 || total + size > 8_388_608 {
            result.limited = true;
            continue;
        }
        let mut code = String::new();
        std::fs::File::open(file)
            .map_err(|e| e.to_string())?
            .take(1_048_577)
            .read_to_string(&mut code)
            .map_err(|e| e.to_string())?;
        if code.len() > 1_048_576 || total + code.len() as u64 > 8_388_608 {
            result.limited = true;
            continue;
        }
        total += code.len() as u64;
        let mut character = 0;
        for (index, line) in code.split_inclusive('\n').enumerate() {
            if let Some(byte) = line.find(query) {
                if result.hits.len() == 256 {
                    result.limited = true;
                    return Ok(result);
                }
                result.hits.push(Hit {
                    path: path.clone(),
                    line: index + 1,
                    character: character + line[..byte].chars().count(),
                    preview: line.trim().chars().take(180).collect(),
                });
            }
            character += line.chars().count();
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_returns_unicode_character_offsets_and_excludes_generated_and_hidden_paths() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("scripts")).unwrap();
        std::fs::create_dir(root.path().join(".kiln-cache")).unwrap();
        std::fs::create_dir(root.path().join("export")).unwrap();
        std::fs::write(
            root.path().join("scripts/enemy.ks"),
            "// 🦀\nfn fire() {return 1;}\nfire();",
        )
        .unwrap();
        for path in [".kiln-cache/ignored.ks", "export/game.ks"] {
            std::fs::write(root.path().join(path), "fire();").unwrap();
        }
        let found = search(root.path(), "fire").unwrap();
        assert_eq!(found.hits.len(), 2);
        assert_eq!(found.hits[0].path, "scripts/enemy.ks");
        assert_eq!(found.hits[0].line, 2);
        assert_eq!(found.hits[0].character, 8);
        assert!(!found.limited);
        assert_eq!(search(root.path(), "fn fire(").unwrap().hits.len(), 1);
    }
    #[test]
    fn deep_or_broad_project_trees_report_truncation_without_unbounded_recursion() {
        let root = tempfile::tempdir().unwrap();
        let mut deepest = root.path().to_path_buf();
        for _ in 0..70 {
            deepest.push("nested");
            std::fs::create_dir(&deepest).unwrap();
        }
        std::fs::write(deepest.join("deep.ks"), "needle").unwrap();
        std::fs::write(root.path().join("visible.ks"), "needle").unwrap();
        let found = search(root.path(), "needle").unwrap();
        assert!(found.limited);
        assert_eq!(found.hits.len(), 1);
        assert_eq!(found.hits[0].path, "visible.ks");
    }
}
