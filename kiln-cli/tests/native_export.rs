#![cfg(unix)]
//! Real CLI transactions; a protocol fixture replaces only the GPU player handshake.
use kiln_core::{
    export::{self, ExportReceipt, RuntimeInfo},
    Project, Scene,
};
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};
struct Fixture {
    root: PathBuf,
    project: PathBuf,
    player: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("kiln-export-{}", uuid::Uuid::new_v4()));
        let game = root.join("project with spaces");
        fs::create_dir_all(game.join("art/nested")).unwrap();
        let mut project = Project {
            id: "export-test".into(),
            name: "A Game".into(),
            ..Default::default()
        };
        project.scenes.insert(
            "main".into(),
            Scene {
                id: "main".into(),
                name: "Main".into(),
                script: Some("main.ks".into()),
                entities: vec![],
                nested_scenes: vec![],
            },
        );
        let manifest = game.join("custom project.toml");
        fs::write(&manifest, project.to_toml().unwrap()).unwrap();
        fs::write(game.join("main.ks"), "game_set(\"ready\",true);").unwrap();
        fs::write(game.join("art/nested/data.txt"), "preserve me").unwrap();
        fs::create_dir(game.join(".kiln-save")).unwrap();
        fs::write(game.join(".kiln-save/slot_0.json"), "private").unwrap();
        let player = root.join("player with spaces");
        let info = RuntimeInfo {
            protocol: export::EXPORT_PROTOCOL,
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            audio: true,
            fonts: true,
            gamepad: true,
        };
        fs::write(
            &player,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                serde_json::to_string(&info).unwrap()
            ),
        )
        .unwrap();
        fs::set_permissions(&player, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            root,
            project: manifest,
            player,
        }
    }
    fn command(&self, output: &Path, job: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
        cmd.args(["export", "--platform", std::env::consts::OS, "--project"])
            .arg(&self.project)
            .arg("--player")
            .arg(&self.player)
            .arg("--output")
            .arg(output)
            .arg("--job-id")
            .arg(job);
        cmd
    }
    fn run(&self, output: &Path, job: &str) -> Output {
        self.command(output, job).output().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn export_preserves_relative_paths_excludes_private_data_and_refuses_overwrite() {
    let f = Fixture::new();
    let output = f.project.parent().unwrap().join("export/published game");
    let result = f.run(&output, "success");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: ExportReceipt =
        serde_json::from_slice(&fs::read(output.join(export::RECEIPT_FILE)).unwrap()).unwrap();
    receipt.validate(&output, "success", "export-test").unwrap();
    let archive = kiln_assets::Archive::load(&output.join("assets.kar")).unwrap();
    assert_eq!(
        archive.extract("art/nested/data.txt").unwrap(),
        b"preserve me"
    );
    assert_eq!(
        Project::from_toml(std::str::from_utf8(&archive.extract("kiln.toml").unwrap()).unwrap())
            .unwrap()
            .id,
        "export-test"
    );
    assert!(archive
        .list()
        .iter()
        .all(|p| !p.contains(".kiln-save") && !p.contains("export/")));
    assert_eq!(
        fs::read(output.join(export::game_binary())).unwrap(),
        fs::read(&f.player).unwrap()
    );
    let before = fs::read(output.join("assets.kar")).unwrap();
    assert!(!f.run(&output, "again").status.success());
    assert_eq!(fs::read(output.join("assets.kar")).unwrap(), before);
    assert!(!export::staging_path(&output, "success").exists());
}
#[test]
fn invalid_source_runtime_and_platform_never_publish_or_leave_staging() {
    let f = Fixture::new();
    let output = f.root.join("result");
    fs::write(f.project.parent().unwrap().join("main.ks"), "if {").unwrap();
    assert!(!f.run(&output, "bad-script").status.success());
    assert!(!output.exists());
    assert!(!export::staging_path(&output, "bad-script").exists());
    fs::write(&f.player, "#!/bin/sh\nprintf 'not a runtime'\n").unwrap();
    assert!(!f.run(&output, "bad-player").status.success());
    assert!(!output.exists());
    let result = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(["export", "--platform", "web", "--project"])
        .arg(&f.project)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Only host-native export"));
    assert!(!output.exists());
}
#[test]
fn cancelling_an_inflight_handshake_reaps_it_and_cleans_partial_output() {
    let f = Fixture::new();
    let output = f.root.join("result");
    // No descendants: this represents an unresponsive runtime executable.
    fs::write(&f.player, "#!/bin/sh\nexec sleep 20\n").unwrap();
    let mut child = f
        .command(&output, "cancel-me")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !export::staging_path(&output, "cancel-me").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    fs::write(export::cancel_path(&output, "cancel-me"), b"cancel").unwrap();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("Cancelled exporter did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!output.exists());
    assert!(!export::staging_path(&output, "cancel-me").exists());
}
#[test]
fn missing_and_escaping_resources_fail_before_publishing() {
    let f = Fixture::new();
    let output = f.root.join("result");
    let mut project = Project::from_toml(&fs::read_to_string(&f.project).unwrap()).unwrap();
    project.audio_assets.push(kiln_core::project::AudioAsset {
        path: "missing.wav".into(),
        ..Default::default()
    });
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    assert!(!f.run(&output, "missing").status.success());
    assert!(!output.exists());
    project.audio_assets[0].path = "../outside.wav".into();
    fs::write(f.root.join("outside.wav"), b"outside").unwrap();
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    assert!(!f.run(&output, "escape").status.success());
    assert!(!output.exists());
    project.audio_assets.clear();
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    std::os::unix::fs::symlink(
        f.root.join("outside.wav"),
        f.project.parent().unwrap().join("linked.wav"),
    )
    .unwrap();
    assert!(!f.run(&output, "symlink").status.success());
    assert!(!output.exists());
}

#[test]
fn malformed_audio_fonts_and_images_are_rejected_without_a_device() {
    let f = Fixture::new();
    let output = f.root.join("result");
    let mut project = Project::from_toml(&fs::read_to_string(&f.project).unwrap()).unwrap();
    let media = f.project.parent().unwrap().join("bad.png");
    fs::write(media, b"invalid media").unwrap();
    project.audio_assets.push(kiln_core::project::AudioAsset {
        name: "hit".into(),
        path: "bad.png".into(),
        ..Default::default()
    });
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    assert!(!f.run(&output, "audio").status.success());
    project.audio_assets.clear();
    project.ui.fonts.insert(
        "font".into(),
        kiln_core::ui::UiFont {
            path: "bad.png".into(),
            ..Default::default()
        },
    );
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    assert!(!f.run(&output, "font").status.success());
    project.ui.fonts.clear();
    project.tilesets.insert(
        "tiles".into(),
        kiln_core::TileSet {
            texture: "bad.png".into(),
            ..Default::default()
        },
    );
    fs::write(&f.project, project.to_toml().unwrap()).unwrap();
    assert!(!f.run(&output, "image").status.success());
    assert!(!output.exists());
}
