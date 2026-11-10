//! Offline identity and integrity validation for a prepared wasm-bindgen runtime.
use kiln_core::export::RuntimeInfo;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
pub const FILES: [&str; 3] = ["kiln_web.js", "kiln_web_bg.wasm", "runtime.json"];
const WASM_LIMIT: u64 = 256 * 1024 * 1024;
#[derive(Debug, Serialize, Deserialize)]
pub struct WebBundle {
    pub runtime: RuntimeInfo,
    pub javascript_sha256: String,
    pub wasm_sha256: String,
}
fn bytes(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.len() > limit {
        return Err(format!(
            "Invalid/oversized runtime file: {}",
            path.display()
        ));
    }
    fs::read(path).map_err(|e| e.to_string())
}
fn leb(bytes: &[u8], position: &mut usize) -> Result<usize, String> {
    let mut value = 0u64;
    for shift in (0..35).step_by(7) {
        let byte = *bytes.get(*position).ok_or("Truncated WASM section size")?;
        *position += 1;
        value |= ((byte & 127) as u64) << shift;
        if byte < 128 {
            return usize::try_from(value)
                .ok()
                .filter(|v| *v <= u32::MAX as usize)
                .ok_or("Invalid WASM section size".into());
        }
    }
    Err("Invalid WASM section size".into())
}
/// Read only bounded custom-section framing; browser instantiation checks full WASM semantics.
pub fn runtime_info(wasm: &[u8]) -> Result<RuntimeInfo, String> {
    if wasm.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err("Invalid WASM header/version".into());
    }
    let mut position = 8;
    let mut metadata = None;
    while position < wasm.len() {
        let kind = wasm[position];
        position += 1;
        let size = leb(wasm, &mut position)?;
        let end = position
            .checked_add(size)
            .filter(|end| *end <= wasm.len())
            .ok_or("Truncated WASM section")?;
        if kind == 0 {
            let mut cursor = 0;
            let section = &wasm[position..end];
            let length = leb(section, &mut cursor)?;
            let name_end = cursor
                .checked_add(length)
                .filter(|v| *v <= section.len())
                .ok_or("Truncated WASM custom section")?;
            if &section[cursor..name_end] == b"kiln.runtime.v1" {
                if metadata.is_some() {
                    return Err("Duplicate runtime metadata".into());
                }
                if section.len() - name_end > 4096 {
                    return Err("Oversized runtime metadata".into());
                }
                metadata =
                    Some(std::str::from_utf8(&section[name_end..]).map_err(|e| e.to_string())?);
            }
        }
        position = end;
    }
    let lines: Vec<_> = metadata
        .ok_or("WASM lacks Kiln runtime metadata; rebuild the runtime")?
        .lines()
        .collect();
    if lines.len() != 3 || !["fonts=0", "fonts=1"].contains(&lines[2]) {
        return Err("Unsupported WASM runtime metadata".into());
    }
    let runtime = RuntimeInfo {
        protocol: lines[0].parse().map_err(|_| "Invalid runtime protocol")?,
        version: lines[1].into(),
        platform: "web".into(),
        arch: "wasm32".into(),
        audio: true,
        fonts: lines[2] == "fonts=1",
        gamepad: true,
    };
    runtime.validate("web")?;
    Ok(runtime)
}
fn inspect(dir: &Path) -> Result<WebBundle, String> {
    let js = String::from_utf8(bytes(&dir.join(FILES[0]), 8 * 1024 * 1024)?)
        .map_err(|e| e.to_string())?;
    for name in ["run", "prepare_game", "start_game"] {
        if !js.contains(&format!("export function {name}(")) {
            return Err(format!(
                "Runtime bindings lack {name}; generate matching wasm-bindgen bindings"
            ));
        }
    }
    if js.lines().any(|line| line.starts_with("import ")) || js.contains("/snippets/") {
        return Err(
            "Runtime needs extra binding resources not supported by this bundle format".into(),
        );
    }
    let wasm_path = dir.join(FILES[1]);
    let runtime = runtime_info(&bytes(&wasm_path, WASM_LIMIT)?)?;
    Ok(WebBundle {
        runtime,
        javascript_sha256: crate::content_hash(&dir.join(FILES[0])).map_err(|e| e.to_string())?,
        wasm_sha256: crate::content_hash(&wasm_path).map_err(|e| e.to_string())?,
    })
}
/// Prepare a trusted generated JS/WASM pair once; exporting only needs these files.
pub fn prepare(dir: &Path) -> Result<(), String> {
    let bundle = inspect(dir)?;
    let data = serde_json::to_vec_pretty(&bundle).map_err(|e| e.to_string())?;
    fs::write(dir.join(FILES[2]), data).map_err(|e| e.to_string())
}
pub fn verify(dir: &Path) -> Result<RuntimeInfo, String> {
    let expected: WebBundle =
        serde_json::from_slice(&bytes(&dir.join(FILES[2]), 65536)?).map_err(|e| e.to_string())?;
    let actual = inspect(dir)?;
    if expected.runtime != actual.runtime
        || expected.javascript_sha256 != actual.javascript_sha256
        || expected.wasm_sha256 != actual.wasm_sha256
    {
        return Err("Prepared JS/WASM runtime changed or was mixed; regenerate the bundle".into());
    }
    Ok(actual.runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn module(metadata: &str) -> Vec<u8> {
        let name = b"kiln.runtime.v1";
        let mut bytes = b"\0asm\x01\0\0\0".to_vec();
        bytes.extend([0, (name.len() + 1 + metadata.len()) as u8, name.len() as u8]);
        bytes.extend(name);
        bytes.extend(metadata.as_bytes());
        bytes
    }
    #[test]
    fn changed_pairs_and_incompatible_or_truncated_metadata_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let js="export function run(){}\nexport function prepare_game(){}\nexport function start_game(){}";
        fs::write(dir.path().join(FILES[0]), js).unwrap();
        let wasm = module(&format!("1\n{}\nfonts=1\n", env!("CARGO_PKG_VERSION")));
        fs::write(dir.path().join(FILES[1]), &wasm).unwrap();
        prepare(dir.path()).unwrap();
        assert_eq!(verify(dir.path()).unwrap().platform, "web");
        fs::write(dir.path().join(FILES[0]), format!("{js}\n// changed")).unwrap();
        assert!(verify(dir.path()).unwrap_err().contains("changed"));
        fs::write(dir.path().join(FILES[0]), js).unwrap();
        fs::write(dir.path().join(FILES[1]), module("1\n999.0\nfonts=1\n")).unwrap();
        assert!(verify(dir.path()).is_err());
        assert!(runtime_info(&module(&format!(
            "1\n{}\nfonts=0\n",
            env!("CARGO_PKG_VERSION")
        )))
        .is_err());
        for length in 0..wasm.len() {
            assert!(runtime_info(&wasm[..length]).is_err());
        }
        let mut duplicate = wasm.clone();
        duplicate.extend(&wasm[8..]);
        assert!(runtime_info(&duplicate).unwrap_err().contains("Duplicate"));
    }
}
