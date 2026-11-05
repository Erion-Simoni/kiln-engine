//! Versioned per-player preferences, independent of project files and save slots.

use kiln_core::audio::AudioMixerSettings;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

/// Schema version for the standalone preferences document.
pub const PREFERENCES_VERSION: u32 = 1;

/// User audio overrides and arbitrary JSON settings for game-specific options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerPreferences {
    pub version: u32,
    #[serde(default)]
    pub audio: Option<AudioMixerSettings>,
    #[serde(default)]
    pub values: BTreeMap<String, serde_json::Value>,
}

impl Default for PlayerPreferences {
    fn default() -> Self {
        Self {
            version: PREFERENCES_VERSION,
            audio: None,
            values: BTreeMap::new(),
        }
    }
}

impl PlayerPreferences {
    /// Missing file means first launch; corrupt or newer files are errors.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let bytes = match fs::read(dir.join("preferences.json")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(error.to_string()),
        };
        let data: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if data.version != PREFERENCES_VERSION {
            return Err(format!("Unsupported preferences version {}", data.version));
        }
        Ok(data)
    }

    /// Write a full temporary document, flush it, then replace the old file.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        if self.version != PREFERENCES_VERSION {
            return Err("Unsupported preferences version".into());
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let temp = dir.join(format!(
            ".preferences-{}-{}.tmp",
            std::process::id(),
            kiln_core::next_id()
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, dir.join("preferences.json"))
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result.map_err(|error| error.to_string())
    }
}

/// Convert script values to JSON without losing integer precision. Null/objects
/// are intentionally absent from the current script language.
pub(crate) fn encode_value(value: &kiln_ir::Value) -> Result<serde_json::Value, String> {
    use kiln_ir::Value;
    match value {
        Value::I64(number) => Ok((*number).into()),
        Value::F64(number) => serde_json::Number::from_f64(*number)
            .map(serde_json::Value::Number)
            .ok_or("Settings require finite numbers".into()),
        Value::Bool(value) => Ok((*value).into()),
        Value::Str(value) => Ok(value.clone().into()),
        Value::Array(values) => values
            .iter()
            .map(encode_value)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
    }
}

pub(crate) fn decode_value(value: &serde_json::Value) -> Option<kiln_ir::Value> {
    use kiln_ir::Value;
    match value {
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(Value::I64)
            .or_else(|| number.as_f64().map(Value::F64)),
        serde_json::Value::Bool(value) => Some(Value::Bool(*value)),
        serde_json::Value::String(value) => Some(Value::Str(value.clone())),
        serde_json::Value::Array(values) => values
            .iter()
            .map(decode_value)
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_survive_restart_without_project_or_sample_state() {
        let dir = std::env::temp_dir().join(format!(
            "kiln-preferences-{}-{}",
            std::process::id(),
            kiln_core::next_id()
        ));
        let mut data = PlayerPreferences::load(&dir).unwrap();
        let mut mixer = AudioMixerSettings {
            master_volume: 0.35,
            ..Default::default()
        };
        mixer.buses.get_mut("Music").unwrap().muted = true;
        data.audio = Some(mixer);
        data.values.insert("language".into(), "sq".into());
        data.save(&dir).unwrap();
        assert_eq!(PlayerPreferences::load(&dir).unwrap(), data);
        data.values.insert("language".into(), "en".into());
        data.save(&dir).unwrap();
        assert_eq!(PlayerPreferences::load(&dir).unwrap(), data);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::write(dir.join("preferences.json"), "{\"version\":999}").unwrap();
        assert!(PlayerPreferences::load(&dir)
            .unwrap_err()
            .contains("Unsupported"));
        fs::write(dir.join("preferences.json"), "{broken").unwrap();
        assert!(PlayerPreferences::load(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
