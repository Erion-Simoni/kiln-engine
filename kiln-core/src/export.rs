//! Export protocol shared by the editor, exporter and standalone player.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const EXPORT_PROTOCOL: u32 = 1;
pub const MAX_RECEIPT_BYTES: u64 = 16 * 1024 * 1024;
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
    /// Legacy runtimes omit this field and cannot export authored physics.
    #[serde(default)]
    pub physics: bool,
    /// Authored fragment pipelines, constants and texture parameters.
    #[serde(default)]
    pub materials: bool,
}
impl RuntimeInfo {
    pub fn validate(&self, platform: &str) -> Result<(), String> {
        let target_matches = if platform == "web" {
            self.platform == "web" && self.arch == "wasm32"
        } else {
            self.platform == platform
                && self.platform == std::env::consts::OS
                && self.arch == std::env::consts::ARCH
        };
        if self.protocol != EXPORT_PROTOCOL
            || self.version != env!("CARGO_PKG_VERSION")
            || !target_matches
            || !self.audio
            || !self.fonts
            || !self.gamepad
        {
            return Err(
                "Runtime does not match this target/engine or lacks audio/fonts/gamepad".into(),
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
    /// Require this job's complete output rather than stale files or log text.
    pub fn validate(&self, dir: &Path, job: &str, project: &str) -> Result<(), String> {
        let web = self.runtime.platform == "web";
        self.runtime
            .validate(if web { "web" } else { std::env::consts::OS })?;
        if self.protocol != EXPORT_PROTOCOL
            || self.job_id != job
            || self.project_id != project
            || (!web && self.files.len() != 2)
            || self.files.len() > 100_010
        {
            return Err("Export receipt identity or file list is invalid".into());
        }
        let required: &[&str] = if web {
            &[
                "index.html",
                "game.js",
                "runtime/kiln_web.js",
                "runtime/kiln_web_bg.wasm",
                "runtime/runtime.json",
                "project/kiln.toml",
            ]
        } else {
            &[game_binary(), "assets.kar"]
        };
        if required
            .iter()
            .any(|name| !self.files.iter().any(|f| f.path == *name))
        {
            return Err("Export output is missing a required file".into());
        }
        let mut seen = std::collections::HashSet::new();
        for file in &self.files {
            let path = Path::new(&file.path);
            if !seen.insert(&file.path)
                || path
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                || file.path.is_empty()
            {
                return Err("Invalid or duplicate export resource path".into());
            }
            let meta = std::fs::symlink_metadata(dir.join(path)).map_err(|e| e.to_string())?;
            if !meta.is_file()
                || meta.len() != file.bytes
                || (required.contains(&file.path.as_str()) && meta.len() == 0)
            {
                return Err(format!("Export output is incomplete: {}", file.path));
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

#[cfg(test)]
mod capability_tests {
    #[test]
    fn legacy_runtime_json_does_not_claim_physics() {
        let legacy = format!(
            r#"{{"protocol":1,"version":"{}","platform":"web","arch":"wasm32","audio":true,"fonts":true,"gamepad":true}}"#,
            env!("CARGO_PKG_VERSION")
        );
        let runtime: super::RuntimeInfo = serde_json::from_str(&legacy).unwrap();
        runtime.validate("web").unwrap();
        assert!(!runtime.physics);
        assert!(!runtime.materials);
        let current = super::RuntimeInfo {
            physics: true,
            ..runtime
        };
        let encoded = serde_json::to_string(&current).unwrap();
        assert!(
            serde_json::from_str::<super::RuntimeInfo>(&encoded)
                .unwrap()
                .physics
        );
    }
}
