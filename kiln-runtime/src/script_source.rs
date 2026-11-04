//! Bounded project-relative script module loading with diagnostic origins.

use std::collections::HashSet;
use std::path::{Component, Path};

pub(crate) struct ExpandedSource {
    pub source: String,
    lines: Vec<(String, usize)>,
}

impl ExpandedSource {
    pub fn diagnostic(&self, diagnostic: &kiln_script::Diagnostic) -> String {
        let mut offset = diagnostic.span.start.min(self.source.len());
        while !self.source.is_char_boundary(offset) {
            offset -= 1;
        }
        let prefix = &self.source[..offset];
        let index = prefix.bytes().filter(|byte| *byte == b'\n').count();
        let column = prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
        let (path, line) = self
            .lines
            .get(index)
            .or_else(|| self.lines.last())
            .cloned()
            .unwrap_or(("<script>".into(), 1));
        format!("{path}:{line}:{column}: {}", diagnostic.message)
    }
}

pub(crate) fn expand(
    path: &str,
    read: &impl Fn(&str) -> Result<String, String>,
) -> Result<ExpandedSource, String> {
    fn load(
        path: &str,
        read: &impl Fn(&str) -> Result<String, String>,
        stack: &mut Vec<String>,
        loaded: &mut HashSet<String>,
        output: &mut ExpandedSource,
    ) -> Result<(), String> {
        if stack.iter().any(|parent| parent == path) || stack.len() >= 32 {
            return Err(format!("{path}: cyclic or excessively deep script import"));
        }
        if !loaded.insert(path.into()) {
            return Ok(());
        }
        if Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!("Invalid script path '{path}'"));
        }
        let text = read(path).map_err(|error| format!("{path}: {error}"))?;
        if text.len() > 1_048_576 {
            return Err(format!("{path}: script exceeds 1 MiB"));
        }
        stack.push(path.into());
        let mut quoted = false;
        let mut braces = 0i32;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if !quoted && braces == 0 && trimmed.starts_with("import ") {
                let module = trimmed
                    .strip_prefix("import \"")
                    .and_then(|value| value.strip_suffix("\";"))
                    .ok_or_else(|| {
                        format!("{path}:{}:1: expected import \"relative.ks\";", index + 1)
                    })?;
                let mut parts: Vec<String> = Path::new(path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                for part in Path::new(module).components() {
                    match part {
                        Component::Normal(value) => {
                            parts.push(value.to_string_lossy().into_owned())
                        }
                        Component::CurDir => {}
                        Component::ParentDir if !parts.is_empty() => {
                            parts.pop();
                        }
                        _ => {
                            return Err(format!(
                                "{path}:{}:1: import escapes the project",
                                index + 1
                            ))
                        }
                    }
                }
                load(&parts.join("/"), read, stack, loaded, output)?;
            } else {
                output.source.push_str(line);
                output.source.push('\n');
                output.lines.push((path.into(), index + 1));
                let mut chars = line.chars().peekable();
                while let Some(character) = chars.next() {
                    if quoted && character == '\\' {
                        chars.next();
                        continue;
                    }
                    if character == '"' {
                        quoted = !quoted;
                    } else if !quoted {
                        if character == '/' && chars.peek() == Some(&'/') {
                            break;
                        }
                        if character == '{' {
                            braces += 1;
                        }
                        if character == '}' {
                            braces -= 1;
                        }
                    }
                }
            }
            if output.source.len() > 1_048_576 {
                return Err("Expanded scripts exceed 1 MiB".into());
            }
        }
        stack.pop();
        Ok(())
    }
    let mut output = ExpandedSource {
        source: String::new(),
        lines: Vec::new(),
    };
    load(
        path,
        read,
        &mut Vec::new(),
        &mut HashSet::new(),
        &mut output,
    )?;
    Ok(output)
}
