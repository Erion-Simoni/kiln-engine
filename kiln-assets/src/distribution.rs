//! Offline integrity checks for a portable editor installation (not a signature).
use kiln_core::distribution::{BundleManifest, BUNDLE_MANIFEST, BUNDLE_SCHEMA};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const LIMIT: u64 = 2 * 1024 * 1024 * 1024;
fn digest(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()))
}
fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.chars().any(char::is_control)
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn inspect_tree(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    for item in walkdir::WalkDir::new(root).follow_links(false) {
        let item = item.map_err(|e| e.to_string())?;
        if item.file_type().is_dir() {
            continue;
        }
        if !item.file_type().is_file() {
            return Err("Installation contains a link or special file".into());
        }
        let path = item
            .path()
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("Non-UTF8 installation path")?
            .replace(std::path::MAIN_SEPARATOR, "/");
        if !valid_path(&path) || !files.insert(path) || files.len() > 20000 {
            return Err("Invalid/oversized installation tree".into());
        }
    }
    Ok(files)
}
fn verify_files(root: &Path, manifest: &BundleManifest) -> Result<(), String> {
    let mut expected = BTreeSet::from([BUNDLE_MANIFEST.to_string()]);
    let mut total = 0u64;
    for entry in &manifest.files {
        if !valid_path(&entry.path) || !expected.insert(entry.path.clone()) {
            return Err("Invalid/duplicate installation path".into());
        }
        total = total
            .checked_add(entry.bytes)
            .filter(|v| *v <= LIMIT)
            .ok_or("Installation exceeds size limit")?;
        let path = root.join(&entry.path);
        let metadata = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", entry.path))?;
        if !metadata.is_file() || metadata.len() != entry.bytes || entry.bytes > 512 * 1024 * 1024 {
            return Err(format!("Missing/changed installation file: {}", entry.path));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if (metadata.permissions().mode() & 0o111 != 0) != entry.executable {
                return Err(format!("Changed executable permissions: {}", entry.path));
            }
        }
        if digest(&path)? != entry.sha256 {
            return Err(format!("Changed installation hash: {}", entry.path));
        }
    }
    if inspect_tree(root)? != expected {
        return Err("Installation has missing or unlisted files".into());
    }
    for tool in ["kiln-editor", "kiln-cli", "kiln-player"] {
        let name = format!("{tool}{}", std::env::consts::EXE_SUFFIX);
        if !manifest
            .files
            .iter()
            .any(|f| f.path == name && f.executable)
        {
            return Err(format!("Missing executable: {tool}"));
        }
    }
    for path in [
        "README.txt",
        "SHA256SUMS",
        "licenses/inventory.json",
        "licenses/README.txt",
        "Cargo.lock",
        "templates/empty_2d.ron",
        "templates/platformer.ron",
        "templates/topdown.ron",
        "examples/ui2d_editor/kiln.toml",
    ] {
        if !expected.contains(path) {
            return Err(format!("Missing required installation resource: {path}"));
        }
    }
    Ok(())
}
/// Check a moved, unmodified installation without launching its tools or devices.
/// A trusted download channel is still required: this manifest is not signed.
pub fn verify(root: &Path) -> Result<BundleManifest, String> {
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("Installation root must be a directory, not a link".into());
    }
    let path = root.join(BUNDLE_MANIFEST);
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
        return Err("Invalid/oversized bundle manifest".into());
    }
    let manifest: BundleManifest =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if manifest.schema != BUNDLE_SCHEMA
        || manifest.version != env!("CARGO_PKG_VERSION")
        || manifest.platform != std::env::consts::OS
        || manifest.arch != std::env::consts::ARCH
        || manifest.files.len() > 20000
        || manifest.tools.len() != 3
    {
        return Err("Incompatible editor installation".into());
    }
    for tool in ["kiln-editor", "kiln-cli", "kiln-player"] {
        manifest
            .tools
            .get(tool)
            .ok_or("Missing tool identity")?
            .validate(tool)?;
    }
    // Reject symlinked parent directories before reading any manifest resource.
    inspect_tree(root)?;
    verify_files(root, &manifest)?;
    crate::web_bundle::verify(&root.join("web-runtime"))?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::distribution::BundleFile;
    fn fixture() -> (tempfile::TempDir, BundleManifest) {
        let dir = tempfile::tempdir().unwrap();
        let manifest = BundleManifest {
            schema: 1,
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            tools: Default::default(),
            files: vec![],
            source_revision: "test".into(),
            source_dirty: true,
        };
        (dir, manifest)
    }
    #[test]
    fn rejects_path_traversal_duplicate_and_unlisted_files() {
        let (dir, mut manifest) = fixture();
        for path in [
            "../escape",
            "/absolute",
            "foo//bar",
            "foo/./bar",
            "C:/escape",
            "foo\\bar",
            BUNDLE_MANIFEST,
        ] {
            manifest.files = vec![BundleFile {
                path: path.into(),
                bytes: 0,
                sha256: String::new(),
                executable: false,
            }];
            assert!(verify_files(dir.path(), &manifest).is_err());
        }
        fs::write(dir.path().join("extra"), b"").unwrap();
        manifest.files.clear();
        assert!(verify_files(dir.path(), &manifest)
            .unwrap_err()
            .contains("unlisted"));
    }
    #[test]
    fn changed_content_and_permissions_are_rejected() {
        let (dir, mut manifest) = fixture();
        let path = dir.path().join("file");
        fs::write(&path, b"abc").unwrap();
        manifest.files.push(BundleFile {
            path: "file".into(),
            bytes: 3,
            sha256: digest(&path).unwrap(),
            executable: false,
        });
        fs::write(&path, b"xyz").unwrap();
        assert!(verify_files(dir.path(), &manifest)
            .unwrap_err()
            .contains("hash"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt};
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(verify_files(dir.path(), &manifest)
                .unwrap_err()
                .contains("permissions"));
            symlink(dir.path(), dir.path().join("link")).unwrap();
            assert!(inspect_tree(dir.path()).is_err());
        }
    }
    #[test]
    fn incompatible_identity_fails_without_launching_tools() {
        let (dir, mut manifest) = fixture();
        manifest.version = "wrong".into();
        fs::write(
            dir.path().join(BUNDLE_MANIFEST),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(dir.path()).unwrap_err().contains("Incompatible"));
    }
}
