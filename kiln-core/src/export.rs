//! Native export protocol shared by the editor, exporter and standalone player.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const EXPORT_PROTOCOL: u32 = 1;
pub const RECEIPT_FILE: &str = "export.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub protocol: u32,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub audio: bool,
    pub fonts: bool,
    pub gamepad: bool,
}
impl RuntimeInfo {
    pub fn validate(&self, platform: &str) -> Result<(), String> {
        if self.protocol != EXPORT_PROTOCOL
            || self.version != env!("CARGO_PKG_VERSION")
            || self.platform != platform
            || self.platform != std::env::consts::OS
            || self.arch != std::env::consts::ARCH
            || !self.audio
            || !self.fonts
            || !self.gamepad
        {
            return Err(
                "Player runtime does not match this host/engine or lacks audio/fonts/gamepad"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFile {
    pub path: String,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReceipt {
    pub protocol: u32,
    pub job_id: String,
    pub project_id: String,
    pub game_name: String,
    pub game_version: String,
    pub runtime: RuntimeInfo,
    pub files: Vec<ExportFile>,
}
impl ExportReceipt {
    /// Require this job's complete native output rather than stale files or log text.
    pub fn validate(&self, dir: &Path, job: &str, project: &str) -> Result<(), String> {
        self.runtime.validate(std::env::consts::OS)?;
        if self.protocol != EXPORT_PROTOCOL
            || self.job_id != job
            || self.project_id != project
            || self.files.len() != 2
        {
            return Err("Export receipt identity or file list is invalid".into());
        }
        for name in [game_binary(), "assets.kar"] {
            let file = self
                .files
                .iter()
                .find(|f| f.path == name)
                .ok_or("Export output is missing a required file")?;
            let meta = std::fs::symlink_metadata(dir.join(name)).map_err(|e| e.to_string())?;
            if !meta.is_file() || meta.len() == 0 || meta.len() != file.bytes {
                return Err(format!("Export output is incomplete: {name}"));
            }
        }
        Ok(())
    }
}

pub fn game_binary() -> &'static str {
    if cfg!(windows) {
        "game.exe"
    } else {
        "game"
    }
}

/// Job IDs are path components, never arbitrary paths supplied to cleanup logic.
pub fn valid_job(job: &str) -> bool {
    !job.is_empty()
        && job.len() <= 64
        && job.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}
pub fn staging_path(output: &Path, job: &str) -> PathBuf {
    output.with_file_name(format!(".kiln-export-{job}"))
}
pub fn cancel_path(output: &Path, job: &str) -> PathBuf {
    output.with_file_name(format!(".kiln-export-{job}.cancel"))
}
