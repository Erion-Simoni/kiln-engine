//! Cross-platform paths for save data, settings, and cache

use std::path::PathBuf;

/// Get the save data directory for the application
pub fn save_dir(app_name: &str) -> PathBuf {
    dirs::data_local_dir()
        .map(|dir| dir.join(app_name))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get the settings directory for the application
pub fn settings_dir(app_name: &str) -> PathBuf {
    dirs::config_dir()
        .map(|dir| dir.join(app_name))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get the cache directory for the application
pub fn cache_dir(app_name: &str) -> PathBuf {
    dirs::cache_dir()
        .map(|dir| dir.join(app_name))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_dir_returns_path() {
        let path = save_dir("TestApp");
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn settings_dir_returns_path() {
        let path = settings_dir("TestApp");
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn cache_dir_returns_path() {
        let path = cache_dir("TestApp");
        assert!(!path.as_os_str().is_empty());
    }
}
