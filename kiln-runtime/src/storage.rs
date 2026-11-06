//! Platform storage contract shared by native games and the browser host.

use kiln_save::{
    slot::{atomic_write, SaveSlotError},
    SaveSlotManager,
};
use std::path::PathBuf;

/// Persistent player data. Missing data is distinct from inaccessible/corrupt data.
pub trait PlayerStorage {
    fn read_slot(&self, slot: u32) -> Result<Option<String>, String>;
    fn write_slot(&self, slot: u32, data: &str) -> Result<(), String>;
    fn read_preferences(&self) -> Result<Option<String>, String>;
    fn write_preferences(&self, data: &str) -> Result<(), String>;
}

/// Native callbacks retain Send/Sync; browser callbacks may contain JS handles.
#[cfg(not(target_arch = "wasm32"))]
pub type SharedStorage = std::sync::Arc<dyn PlayerStorage + Send + Sync>;
#[cfg(target_arch = "wasm32")]
pub type SharedStorage = std::rc::Rc<dyn PlayerStorage>;

/// Native versioned slot files and independent player preferences.
pub struct FileStorage(pub PathBuf);

impl PlayerStorage for FileStorage {
    fn read_slot(&self, slot: u32) -> Result<Option<String>, String> {
        match SaveSlotManager::with_directory(self.0.clone()).load(slot) {
            Ok(data) => Ok(Some(data.data)),
            Err(SaveSlotError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(error.to_string()),
        }
    }
    fn write_slot(&self, slot: u32, data: &str) -> Result<(), String> {
        SaveSlotManager::with_directory(self.0.clone())
            .save(slot, data, "Gameplay save")
            .map_err(|e| e.to_string())
    }
    fn read_preferences(&self) -> Result<Option<String>, String> {
        let path = self.0.join("preferences.json");
        match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(meta) if meta.len() > 1_048_576 => return Err("Preferences exceed 1 MiB".into()),
            Ok(_) => {}
        }
        std::fs::read_to_string(path)
            .map(Some)
            .map_err(|e| e.to_string())
    }
    fn write_preferences(&self, data: &str) -> Result<(), String> {
        if data.len() > 1_048_576 {
            return Err("Preferences exceed 1 MiB".into());
        }
        atomic_write(&self.0.join("preferences.json"), data.as_bytes()).map_err(|e| e.to_string())
    }
}
