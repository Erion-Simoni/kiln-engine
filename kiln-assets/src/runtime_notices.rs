//! Runtime-matched notice copying and offline exported-byte verification.
use crate::distribution::{digest, inspect_tree, valid_path};
use kiln_core::{
    distribution::{BundleFile, BundleManifest, BUNDLE_MANIFEST},
    export::{self, ExportReceipt, REQUIRED_RUNTIME_NOTICES, RUNTIME_NOTICES_FILE},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FILE_LIMIT: u64 = 512 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct NoticeFile {
    path: String,
    bytes: u64,
    sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct NoticeManifest {
    schema: u32,
    source_revision: String,
    source_dirty: bool,
    runtime_files: Vec<NoticeFile>,
    files: Vec<NoticeFile>,
}

fn runtime_paths(web: bool) -> Vec<String> {
    if web {
        crate::web_bundle::FILES
            .iter()
            .map(|name| format!("runtime/{name}"))
            .collect()
    } else {
        vec![export::game_binary().into()]
    }
}

fn check_file(root: &Path, file: &NoticeFile) -> Result<(), String> {
    if !valid_path(&file.path) || file.bytes > FILE_LIMIT {
        return Err("Invalid runtime notice inventory path/bounds".into());
    }
    let path = root.join(&file.path);
    for parent in Path::new(&file.path).ancestors().skip(1) {
        if !fs::symlink_metadata(root.join(parent))
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err("Runtime/notice parent is a link or not a directory".into());
        }
    }
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() != file.bytes || digest(&path)? != file.sha256 {
        return Err(format!("Changed runtime/notice file: {}", file.path));
    }
    Ok(())
}

fn runtime_entries(
    files: &[BundleFile],
    output: &Path,
    web: bool,
) -> Result<Vec<NoticeFile>, String> {
    runtime_paths(web)
        .into_iter()
        .map(|path| {
            let original = if web {
                path.replacen("runtime/", "web-runtime/", 1)
            } else {
                format!("kiln-player{}", std::env::consts::EXE_SUFFIX)
            };
            let entry = files
                .iter()
                .find(|file| file.path == original)
                .ok_or("Installation is missing the selected runtime payload")?;
            let file = NoticeFile {
                path,
                bytes: entry.bytes,
                sha256: entry.sha256.clone(),
            };
            check_file(output, &file)?;
            Ok(file)
        })
        .collect()
}

fn copy_file(
    source: &Path,
    output: &Path,
    entry: &BundleFile,
    cancelled: &mut impl FnMut() -> Result<(), String>,
) -> Result<NoticeFile, String> {
    let from = source.join(&entry.path);
    let meta = fs::symlink_metadata(&from).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() != entry.bytes || entry.bytes > FILE_LIMIT {
        return Err(format!("Changed installation notice: {}", entry.path));
    }
    let to = output.join(&entry.path);
    fs::create_dir_all(to.parent().ok_or("Notice has no parent")?).map_err(|e| e.to_string())?;
    let mut input = fs::File::open(from).map_err(|e| e.to_string())?;
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut total = 0;
    let mut buffer = [0; 65536];
    loop {
        cancelled()?;
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > entry.bytes {
            return Err("Installation notice grew during copy".into());
        }
        target
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
        hash.update(&buffer[..count]);
    }
    if total != entry.bytes || hex::encode(hash.finalize()) != entry.sha256 {
        return Err(format!(
            "Installation notice changed during copy: {}",
            entry.path
        ));
    }
    Ok(NoticeFile {
        path: entry.path.clone(),
        bytes: entry.bytes,
        sha256: entry.sha256.clone(),
    })
}

fn installation_for(
    selected_runtime: &Path,
    web: bool,
) -> Result<Option<(PathBuf, BundleManifest)>, String> {
    let selected = selected_runtime.canonicalize().map_err(|e| e.to_string())?;
    let installation = selected
        .parent()
        .ok_or("Runtime has no installation parent")?;
    if !installation.join(BUNDLE_MANIFEST).exists() {
        if installation.join("licenses").exists() {
            return Err("Runtime notice inventory lacks a matching installation manifest".into());
        }
        return Ok(None);
    }
    let bundle = crate::distribution::verify(installation)?;
    let expected = if web {
        "web-runtime".into()
    } else {
        format!("kiln-player{}", std::env::consts::EXE_SUFFIX)
    };
    if installation
        .join(expected)
        .canonicalize()
        .map_err(|e| e.to_string())?
        != selected
    {
        return Err("Selected runtime is not the installation's matched runtime".into());
    }
    for path in REQUIRED_RUNTIME_NOTICES {
        if !bundle
            .files
            .iter()
            .any(|file| file.path == path && file.bytes > 0)
        {
            return Err(format!(
                "Runtime installation lacks {path}; prepare an updated bundle"
            ));
        }
    }
    Ok(Some((installation.to_path_buf(), bundle)))
}

/// Copy notices from the actual selected runtime's verified portable installation.
/// Returns None for loose developer runtimes. The caller owns private staging and
/// must discard it on failure. No commands or devices are opened by this function.
pub fn attach(
    selected_runtime: &Path,
    output: &Path,
    web: bool,
    mut cancelled: impl FnMut() -> Result<(), String>,
) -> Result<Option<String>, String> {
    cancelled()?;
    let Some((installation, bundle)) = installation_for(selected_runtime, web)? else {
        return Ok(None);
    };
    let runtime_files = runtime_entries(&bundle.files, output, web)?;
    cancelled()?;
    fs::create_dir(output.join("licenses")).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for entry in bundle
        .files
        .iter()
        .filter(|file| file.path.starts_with("licenses/"))
    {
        if entry.path == RUNTIME_NOTICES_FILE {
            return Err("Installation contains a reserved notice manifest path".into());
        }
        files.push(copy_file(&installation, output, entry, &mut cancelled)?);
    }
    let manifest = NoticeManifest {
        schema: 1,
        source_revision: bundle.source_revision,
        source_dirty: bundle.source_dirty,
        runtime_files,
        files,
    };
    write_manifest(output, &manifest, &mut cancelled).map(Some)
}

fn write_manifest(
    output: &Path,
    manifest: &NoticeManifest,
    cancelled: &mut impl FnMut() -> Result<(), String>,
) -> Result<String, String> {
    let data = serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())?;
    if data.len() as u64 > MANIFEST_LIMIT {
        return Err("Export notice manifest exceeds bounds".into());
    }
    cancelled()?;
    let path = output.join(RUNTIME_NOTICES_FILE);
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .and_then(|mut file| file.write_all(&data))
        .map_err(|e| e.to_string())?;
    digest(&path)
}

/// Verify attached notice bytes, exact notice tree/receipt coverage and runtime
/// payload binding. This is integrity checking, not publisher authentication.
pub fn verify_export(output: &Path, receipt: &ExportReceipt) -> Result<(), String> {
    receipt.validate(output, &receipt.job_id, &receipt.project_id)?;
    let Some(expected_hash) = &receipt.runtime_notice_manifest_sha256 else {
        return if fs::symlink_metadata(output.join("licenses")).is_ok() {
            Err("Export contains unreceipted runtime notices".into())
        } else {
            Ok(())
        };
    };
    // Reject linked parents before opening the manifest or its inventoried files.
    let actual: BTreeSet<_> = inspect_tree(&output.join("licenses"))?
        .into_iter()
        .map(|name| format!("licenses/{name}"))
        .collect();
    let path = output.join(RUNTIME_NOTICES_FILE);
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MANIFEST_LIMIT || digest(&path)? != *expected_hash {
        return Err("Changed runtime notice manifest".into());
    }
    let manifest: NoticeManifest =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    verify_manifest(output, receipt, &manifest, &actual)
}

fn verify_manifest(
    output: &Path,
    receipt: &ExportReceipt,
    manifest: &NoticeManifest,
    actual: &BTreeSet<String>,
) -> Result<(), String> {
    let expected_runtime: BTreeSet<_> = runtime_paths(receipt.runtime.platform == "web")
        .into_iter()
        .collect();
    let listed_runtime: BTreeSet<_> = manifest
        .runtime_files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    if manifest.schema != 1
        || manifest.files.len() >= 20000
        || listed_runtime != expected_runtime
        || listed_runtime.len() != manifest.runtime_files.len()
    {
        return Err("Invalid runtime notice manifest coverage".into());
    }
    let mut expected = BTreeSet::from([RUNTIME_NOTICES_FILE.to_string()]);
    let mut total = 0u64;
    for file in &manifest.files {
        if !valid_path(&file.path)
            || !file.path.starts_with("licenses/")
            || !expected.insert(file.path.clone())
        {
            return Err("Invalid/duplicate runtime notice path".into());
        }
        total = total
            .checked_add(file.bytes)
            .filter(|value| *value <= TOTAL_LIMIT)
            .ok_or("Runtime notices exceed bounds")?;
    }
    let receipted: BTreeSet<_> = receipt
        .files
        .iter()
        .filter(|file| file.path.starts_with("licenses/"))
        .map(|file| file.path.clone())
        .collect();
    if &expected != actual || expected != receipted {
        return Err("Runtime notice tree/receipt coverage changed".into());
    }
    for file in manifest.files.iter().chain(&manifest.runtime_files) {
        check_file(output, file)?;
    }
    Ok(())
}
