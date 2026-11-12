use crate::distribution::{digest, inspect_tree};
use crate::runtime_notices::*;
use kiln_core::{
    distribution::{BundleFile, BundleManifest, ToolInfo, BUNDLE_MANIFEST},
    export::{
        self, ExportFile, ExportReceipt, RuntimeInfo, EXPORT_PROTOCOL, REQUIRED_RUNTIME_NOTICES,
        RUNTIME_NOTICES_FILE,
    },
};
use std::{fs, path::Path};

struct Fixture {
    _temp: tempfile::TempDir,
    installation: std::path::PathBuf,
    output: std::path::PathBuf,
}
fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn runtime(web: bool) -> RuntimeInfo {
    RuntimeInfo {
        protocol: EXPORT_PROTOCOL,
        version: env!("CARGO_PKG_VERSION").into(),
        platform: if web { "web" } else { std::env::consts::OS }.into(),
        arch: if web {
            "wasm32"
        } else {
            std::env::consts::ARCH
        }
        .into(),
        audio: true,
        fonts: true,
        gamepad: true,
        physics: true,
        materials: true,
    }
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let installation = temp.path().join("moved installation");
        let output = temp.path().join("private output");
        fs::create_dir(&output).unwrap();
        let mut tools = std::collections::BTreeMap::new();
        for tool in ["kiln-editor", "kiln-cli", "kiln-player"] {
            let path = installation.join(format!("{tool}{}", std::env::consts::EXE_SUFFIX));
            write(&path, format!("compiled {tool}"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            }
            tools.insert(
                tool.into(),
                ToolInfo::current(
                    tool,
                    vec![
                        "audio",
                        "fonts",
                        "gamepad",
                        "native-export",
                        "browser-export",
                        "bundle-verify",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    false,
                ),
            );
        }
        for path in [
            "README.txt",
            "SHA256SUMS",
            "Cargo.lock",
            "templates/empty_2d.ron",
            "templates/platformer.ron",
            "templates/topdown.ron",
            "examples/ui2d_editor/kiln.toml",
        ] {
            write(&installation.join(path), b"fixture");
        }
        for path in REQUIRED_RUNTIME_NOTICES {
            write(
                &installation.join(path),
                format!("Original {path}: \u{0141} attribution\n"),
            );
        }
        write(
            &installation.join("licenses/sources/library.crate"),
            vec![42; 131073],
        );
        let metadata = format!(
            "{EXPORT_PROTOCOL}\n{}\nfonts=1\nphysics=1\nmaterials=1\n",
            env!("CARGO_PKG_VERSION")
        );
        let name = b"kiln.runtime.v1";
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend([0, (name.len() + 1 + metadata.len()) as u8, name.len() as u8]);
        wasm.extend(name);
        wasm.extend(metadata.as_bytes());
        write(&installation.join("web-runtime/kiln_web_bg.wasm"), wasm);
        write(&installation.join("web-runtime/kiln_web.js"), b"export function run(){}\nexport function prepare_game(){}\nexport function start_game(){}");
        crate::web_bundle::prepare(&installation.join("web-runtime")).unwrap();
        let fixture = Self {
            _temp: temp,
            installation,
            output,
        };
        fixture.save_inventory(tools);
        fixture
    }
    fn save_inventory(&self, tools: std::collections::BTreeMap<String, ToolInfo>) {
        let files = walkdir::WalkDir::new(&self.installation)
            .into_iter()
            .map(Result::unwrap)
            .filter(|item| item.file_type().is_file() && item.file_name() != BUNDLE_MANIFEST)
            .map(|item| {
                let path = item
                    .path()
                    .strip_prefix(&self.installation)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace(std::path::MAIN_SEPARATOR, "/");
                let executable = tools
                    .keys()
                    .any(|tool| path == format!("{tool}{}", std::env::consts::EXE_SUFFIX));
                BundleFile {
                    path,
                    bytes: item.metadata().unwrap().len(),
                    sha256: digest(item.path()).unwrap(),
                    executable,
                }
            })
            .collect();
        let manifest = BundleManifest {
            schema: 1,
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            tools,
            files,
            source_revision: "fixture-source".into(),
            source_dirty: false,
        };
        write(
            &self.installation.join(BUNDLE_MANIFEST),
            serde_json::to_vec(&manifest).unwrap(),
        );
    }
    fn export(&self, web: bool) -> ExportReceipt {
        let selected = if web {
            let to = self.output.join("runtime");
            fs::create_dir(&to).unwrap();
            for name in crate::web_bundle::FILES {
                fs::copy(
                    self.installation.join("web-runtime").join(name),
                    to.join(name),
                )
                .unwrap();
            }
            for name in ["index.html", "game.js", "project/kiln.toml"] {
                write(&self.output.join(name), b"project");
            }
            self.installation.join("web-runtime")
        } else {
            let player = self
                .installation
                .join(format!("kiln-player{}", std::env::consts::EXE_SUFFIX));
            fs::copy(&player, self.output.join(export::game_binary())).unwrap();
            write(&self.output.join("assets.kar"), b"project archive");
            player
        };
        let hash = attach(&selected, &self.output, web, || Ok(())).unwrap();
        let files = walkdir::WalkDir::new(&self.output)
            .into_iter()
            .map(Result::unwrap)
            .filter(|item| item.file_type().is_file())
            .map(|item| ExportFile {
                path: item
                    .path()
                    .strip_prefix(&self.output)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
                bytes: item.metadata().unwrap().len(),
            })
            .collect();
        ExportReceipt {
            protocol: EXPORT_PROTOCOL,
            job_id: "notices".into(),
            project_id: "game".into(),
            game_name: "Game".into(),
            game_version: "1".into(),
            runtime: runtime(web),
            files,
            runtime_notice_manifest_sha256: hash,
        }
    }
}

#[test]
fn native_and_browser_exports_preserve_original_bytes_and_bind_the_runtime() {
    for web in [false, true] {
        let fixture = Fixture::new();
        let receipt = fixture.export(web);
        assert!(receipt.runtime_notice_manifest_sha256.is_some());
        verify_export(&fixture.output, &receipt).unwrap();
        for file in inspect_tree(&fixture.installation.join("licenses")).unwrap() {
            assert_eq!(
                fs::read(fixture.installation.join("licenses").join(&file)).unwrap(),
                fs::read(fixture.output.join("licenses").join(&file)).unwrap()
            );
        }
        let path = fixture.output.join(if web {
            "runtime/kiln_web.js"
        } else {
            export::game_binary()
        });
        let mut bytes = fs::read(&path).unwrap();
        bytes[0] ^= 1;
        fs::write(path, bytes).unwrap();
        assert!(verify_export(&fixture.output, &receipt)
            .unwrap_err()
            .contains("Changed runtime/notice"));
    }
}

#[test]
fn missing_changed_unlisted_unreceipted_and_tampered_manifests_fail() {
    for kind in [
        "missing", "changed", "extra", "receipt", "manifest", "outside",
    ] {
        let fixture = Fixture::new();
        let mut receipt = fixture.export(false);
        let path = fixture.output.join("licenses/project/LICENSE-MIT");
        match kind {
            "missing" => fs::remove_file(path).unwrap(),
            "changed" => {
                let mut bytes = fs::read(&path).unwrap();
                bytes[0] ^= 1;
                fs::write(path, bytes).unwrap();
            }
            "extra" => write(&fixture.output.join("licenses/extra"), b"unlisted"),
            "receipt" => receipt
                .files
                .retain(|file| file.path != "licenses/sources/library.crate"),
            "manifest" => {
                let path = fixture.output.join(RUNTIME_NOTICES_FILE);
                let mut bytes = fs::read(&path).unwrap();
                bytes[0] ^= 1;
                fs::write(path, bytes).unwrap();
            }
            "outside" => {
                write(&fixture.output.join("unrelated"), b"no");
                receipt.files.push(ExportFile {
                    path: "unrelated".into(),
                    bytes: 2,
                });
            }
            _ => unreachable!(),
        }
        assert!(verify_export(&fixture.output, &receipt).is_err(), "{kind}");
    }
}

#[test]
fn loose_runtime_is_explicit_and_incomplete_installations_fail() {
    let fixture = Fixture::new();
    let loose = fixture._temp.path().join("loose/player");
    write(&loose, b"developer runtime");
    assert!(attach(&loose, &fixture.output, false, || Ok(()))
        .unwrap()
        .is_none());
    let player = fixture
        .installation
        .join(format!("kiln-player{}", std::env::consts::EXE_SUFFIX));
    fs::remove_file(fixture.installation.join("licenses/project/LICENSE-MIT")).unwrap();
    assert!(attach(&player, &fixture.output, false, || Ok(())).is_err());
    assert!(!fixture.output.join("licenses").exists());
}

#[test]
fn only_the_selected_installation_runtime_can_supply_notices() {
    let fixture = Fixture::new();
    let player = fixture
        .installation
        .join(format!("kiln-player{}", std::env::consts::EXE_SUFFIX));
    fs::copy(&player, fixture.output.join(export::game_binary())).unwrap();
    let other = fixture
        .installation
        .join(format!("kiln-editor{}", std::env::consts::EXE_SUFFIX));
    assert!(attach(&other, &fixture.output, false, || Ok(()))
        .unwrap_err()
        .contains("matched runtime"));
    write(
        &fixture.output.join(export::game_binary()),
        b"different selected payload",
    );
    assert!(attach(&player, &fixture.output, false, || Ok(())).is_err());
    assert!(!fixture.output.join("licenses").exists());
}

#[test]
fn invalid_notice_metadata_fails_even_if_its_manifest_hash_is_updated() {
    for kind in ["traversal", "duplicate", "runtime", "bounds"] {
        let fixture = Fixture::new();
        let mut receipt = fixture.export(false);
        let path = fixture.output.join(RUNTIME_NOTICES_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match kind {
            "traversal" => value["files"][0]["path"] = "../escape".into(),
            "duplicate" => {
                let entry = value["files"][0].clone();
                value["files"].as_array_mut().unwrap().push(entry);
            }
            "runtime" => value["runtime_files"][0]["path"] = "licenses/inventory.json".into(),
            "bounds" => value["files"][0]["bytes"] = (2u64 * 1024 * 1024 * 1024 + 1).into(),
            _ => unreachable!(),
        }
        write(&path, serde_json::to_vec(&value).unwrap());
        receipt.runtime_notice_manifest_sha256 = Some(digest(&path).unwrap());
        receipt
            .files
            .iter_mut()
            .find(|file| file.path == RUNTIME_NOTICES_FILE)
            .unwrap()
            .bytes = fs::metadata(path).unwrap().len();
        assert!(verify_export(&fixture.output, &receipt).is_err(), "{kind}");
    }
}

#[test]
fn cancellation_and_existing_notice_destinations_never_succeed() {
    let fixture = Fixture::new();
    let player = fixture
        .installation
        .join(format!("kiln-player{}", std::env::consts::EXE_SUFFIX));
    fs::copy(&player, fixture.output.join(export::game_binary())).unwrap();
    let mut calls = 0;
    let result = attach(&player, &fixture.output, false, || {
        calls += 1;
        if calls > 4 {
            Err("cancelled".into())
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err(), "cancelled");
    assert!(!fixture.output.join(RUNTIME_NOTICES_FILE).exists());
    let before = fixture.output.join("licenses/keep");
    write(&before, b"keep");
    assert!(attach(&player, &fixture.output, false, || Ok(())).is_err());
    assert_eq!(fs::read(before).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn linked_notice_parents_are_rejected_before_reading_inventory_members() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let receipt = fixture.export(false);
    fs::rename(
        fixture.output.join("licenses/project"),
        fixture.output.join("relocated-project-notices"),
    )
    .unwrap();
    symlink(
        fixture.output.join("relocated-project-notices"),
        fixture.output.join("licenses/project"),
    )
    .unwrap();
    assert!(verify_export(&fixture.output, &receipt).is_err());
}
