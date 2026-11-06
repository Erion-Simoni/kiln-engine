//! Project-scoped browser storage using the native gameplay payload schema.
use kiln_runtime::storage::PlayerStorage;

pub struct BrowserStorage {
    pub project: String,
}
impl BrowserStorage {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        let storage = web_sys::window()
            .ok_or("No browser window")?
            .local_storage()
            .map_err(|e| format!("Browser storage denied: {e:?}"))?
            .ok_or("Browser storage unavailable")?;
        let result = storage
            .get_item(&format!("kiln:{}:{key}", self.project))
            .map_err(|e| format!("Browser storage read: {e:?}"))?;
        if result
            .as_ref()
            .is_some_and(|v| v.len() > kiln_runtime::MAX_GAME_SAVE_BYTES)
        {
            return Err("Browser storage document exceeds 8 MiB".into());
        }
        Ok(result)
    }
    fn write(&self, key: &str, data: &str) -> Result<(), String> {
        if data.len() > kiln_runtime::MAX_GAME_SAVE_BYTES {
            return Err("Browser storage document exceeds 8 MiB".into());
        }
        let storage = web_sys::window()
            .ok_or("No browser window")?
            .local_storage()
            .map_err(|e| format!("Browser storage denied: {e:?}"))?
            .ok_or("Browser storage unavailable")?;
        // localStorage.setItem is atomic: quota/access failure retains the old value.
        storage
            .set_item(&format!("kiln:{}:{key}", self.project), data)
            .map_err(|e| format!("Browser storage write (quota/access): {e:?}"))
    }
}
impl PlayerStorage for BrowserStorage {
    fn read_slot(&self, slot: u32) -> Result<Option<String>, String> {
        self.read(&format!("slot:{slot}"))
    }
    fn write_slot(&self, slot: u32, data: &str) -> Result<(), String> {
        self.write(&format!("slot:{slot}"), data)
    }
    fn read_preferences(&self) -> Result<Option<String>, String> {
        self.read("preferences")
    }
    fn write_preferences(&self, data: &str) -> Result<(), String> {
        self.write("preferences", data)
    }
}
