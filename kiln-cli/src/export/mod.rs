//! Host-native export without a source checkout or toolchain at export/play time.
mod resources;
use anyhow::{bail, Context, Result};
use kiln_assets::Archive;
use kiln_core::export::{
    self, ExportFile, ExportReceipt, RuntimeInfo, EXPORT_PROTOCOL, RECEIPT_FILE,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub struct Options {
    pub platform: String,
    pub project: PathBuf,
    pub output: PathBuf,
    pub player: Option<PathBuf>,
    pub job_id: Option<String>,
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn cancelled(marker: &Path) -> Result<()> {
    if marker.exists() {
        bail!("Export cancelled");
    }
    Ok(())
}

fn runtime_info(player: &Path, marker: &Path) -> Result<RuntimeInfo> {
    // The handshake never opens graphics/audio and has bounded output/time.
    let mut child = Command::new(player)
        .arg("--runtime-info")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Cannot start player runtime {}", player.display()))?;
    let mut pipe = child.stdout.take().context("Runtime stdout missing")?;
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        pipe.by_ref()
            .take(65537)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if marker.exists() || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Runtime handshake cancelled or timed out");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = reader
        .join()
        .map_err(|_| anyhow::anyhow!("Runtime handshake reader failed"))??;
    if !status.success() || bytes.len() > 65536 {
        bail!("Player runtime handshake failed: {status}");
    }
    serde_json::from_slice(&bytes)
        .context("Player does not implement the current export runtime protocol")
}

pub fn run(options: Options) -> Result<()> {
    if !["linux", "windows", "macos"].contains(&options.platform.as_str())
        || options.platform != std::env::consts::OS
    {
        bail!(
            "Only host-native export ({}) is available in this checkpoint; '{}' is not ready",
            std::env::consts::OS,
            options.platform
        );
    }
    let job = options
        .job_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if !export::valid_job(&job) {
        bail!("Invalid export job ID");
    }
    let manifest = options
        .project
        .canonicalize()
        .context("Project manifest is missing")?;
    let root = manifest.parent().context("Project has no directory")?;
    let project = resources::read_project(&manifest)?;
    let output = std::path::absolute(options.output)?;
    if output.file_name().is_none() {
        bail!("Choose an output subdirectory");
    }
    let parent = output.parent().context("Output has no parent")?;
    fs::create_dir_all(parent)?;
    let output = parent.canonicalize()?.join(output.file_name().unwrap());
    if root.starts_with(&output) {
        bail!("Output cannot replace the project directory or its parent");
    }
    if output.exists()
        && (fs::symlink_metadata(&output)?.file_type().is_symlink()
            || !output.is_dir()
            || fs::read_dir(&output)?.next().is_some())
    {
        bail!(
            "Output already exists and is not empty: {}. Choose a new directory",
            output.display()
        );
    }
    let stage = export::staging_path(&output, &job);
    let marker = export::cancel_path(&output, &job);
    cancelled(&marker)?;
    fs::create_dir(&stage).context("Cannot create private export staging directory")?;
    let stage = Staging(stage);
    println!("Validating player runtime");
    let player = options
        .player
        .unwrap_or(std::env::current_exe()?.with_file_name(if cfg!(windows) {
            "kiln-player.exe"
        } else {
            "kiln-player"
        }));
    let player = player.canonicalize().context(
        "Prebuilt player is missing; build/install the matching player once, or use --player",
    )?;
    let runtime = runtime_info(&player, &marker)?;
    runtime
        .validate(&options.platform)
        .map_err(anyhow::Error::msg)?;
    println!("Snapshotting saved project '{}'", project.name);
    let snapshot = stage.0.join("project");
    fs::create_dir(&snapshot)?;
    let files = resources::snapshot(
        root, &manifest, &output, &stage.0, &snapshot, &marker, &project,
    )?;
    resources::validate(&project, &snapshot)?;
    cancelled(&marker)?;
    println!(
        "Packing {} project files with relative resource paths",
        files.len()
    );
    let archive_path = stage.0.join("assets.kar");
    Archive::new(archive_path.clone()).pack_with_version(&files, Some(&project.version))?;
    let archive = Archive::load(&archive_path)?;
    for file in &files {
        cancelled(&marker)?;
        let relative = file
            .strip_prefix(&snapshot)?
            .to_string_lossy()
            .replace('\\', "/");
        if archive.extract(&relative)? != fs::read(file)? {
            bail!("Archive verification failed: {relative}");
        }
    }
    println!("Copying standalone runtime");
    let game_path = stage.0.join(export::game_binary());
    fs::copy(&player, &game_path)?;
    fs::remove_dir_all(&snapshot)?;
    let receipt = ExportReceipt {
        protocol: EXPORT_PROTOCOL,
        job_id: job.clone(),
        project_id: project.id.clone(),
        game_name: project.name,
        game_version: project.version,
        runtime,
        files: [export::game_binary(), "assets.kar"]
            .into_iter()
            .map(|name| {
                Ok(ExportFile {
                    path: name.into(),
                    bytes: fs::metadata(stage.0.join(name))?.len(),
                })
            })
            .collect::<Result<_, std::io::Error>>()?,
    };
    receipt
        .validate(&stage.0, &job, &project.id)
        .map_err(anyhow::Error::msg)?;
    fs::write(
        stage.0.join(RECEIPT_FILE),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    cancelled(&marker)?;
    // Never replace a previous export. Remove an empty destination for Windows rename.
    if output.exists() {
        fs::remove_dir(&output).context("Output became nonempty during export")?;
    }
    fs::rename(&stage.0, &output).context("Cannot publish complete export")?;
    println!("Export complete: {}", output.display());
    println!(
        "Launch {} directly; Rust and the source project are not required",
        export::game_binary()
    );
    Ok(())
}
