//! Identity of tools prepared together in a portable editor installation.
use serde::{Deserialize, Serialize};

pub const BUNDLE_SCHEMA: u32 = 1;
pub const BUNDLE_MANIFEST: &str = "bundle.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub schema: u32,
    pub tool: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub debug_assertions: bool,
    pub features: Vec<String>,
}
impl ToolInfo {
    pub fn current(tool: &str, features: Vec<String>, debug_assertions: bool) -> Self {
        Self {
            schema: BUNDLE_SCHEMA,
            tool: tool.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            debug_assertions,
            features,
        }
    }
    pub fn validate(&self, expected: &str) -> Result<(), String> {
        if self.schema != BUNDLE_SCHEMA
            || self.tool != expected
            || self.version != env!("CARGO_PKG_VERSION")
            || self.platform != std::env::consts::OS
            || self.arch != std::env::consts::ARCH
        {
            return Err(format!("Incompatible installed tool: {expected}"));
        }
        let required: &[&str] = match expected {
            "kiln-editor" | "kiln-player" => &["audio", "fonts", "gamepad"],
            "kiln-cli" => &["native-export", "browser-export", "bundle-verify"],
            _ => return Err("Unknown installed tool identity".into()),
        };
        if required
            .iter()
            .any(|name| !self.features.iter().any(|f| f == name))
        {
            return Err(format!(
                "Installed tool lacks required features: {expected}"
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub executable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema: u32,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub tools: std::collections::BTreeMap<String, ToolInfo>,
    pub files: Vec<BundleFile>,
    pub source_revision: String,
    pub source_dirty: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatched_tools_and_missing_runtime_features_are_rejected() {
        let mut info = ToolInfo::current(
            "kiln-editor",
            vec!["audio".into(), "fonts".into(), "gamepad".into()],
            true,
        );
        info.validate("kiln-editor").unwrap();
        assert!(info.validate("kiln-player").is_err());
        info.features.pop();
        assert!(info.validate("kiln-editor").is_err());
        info.version = "unknown".into();
        assert!(info.validate("kiln-editor").is_err());
    }
}
