//! Localization system for UI strings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A localization key (e.g., "menu.start", "settings.audio_volume").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalizationKey(String);

impl LocalizationKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for LocalizationKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for LocalizationKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Localization data for a language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Localization {
    language: String,
    strings: HashMap<LocalizationKey, String>,
}

impl Localization {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            strings: HashMap::new(),
        }
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    /// Adds a localized string.
    pub fn add(&mut self, key: impl Into<LocalizationKey>, value: impl Into<String>) {
        self.strings.insert(key.into(), value.into());
    }

    /// Gets a localized string by key.
    pub fn get(&self, key: &LocalizationKey) -> Option<&String> {
        self.strings.get(key)
    }

    /// Gets a localized string by key, returning the key itself if not found.
    pub fn get_or_key(&self, key: &LocalizationKey) -> String {
        self.get(key).cloned().unwrap_or_else(|| key.as_str().to_string())
    }

    /// Returns the number of localized strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns true if there are no localized strings.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localization_key() {
        let key = LocalizationKey::new("menu.start");
        assert_eq!(key.as_str(), "menu.start");
    }

    #[test]
    fn test_localization_add_get() {
        let mut loc = Localization::new("en");
        loc.add("menu.start", "Start Game");
        loc.add("menu.settings", "Settings");

        assert_eq!(loc.get(&LocalizationKey::new("menu.start")), Some(&"Start Game".to_string()));
        assert_eq!(loc.get(&LocalizationKey::new("menu.settings")), Some(&"Settings".to_string()));
        assert_eq!(loc.get(&LocalizationKey::new("menu.quit")), None);
    }

    #[test]
    fn test_localization_get_or_key() {
        let mut loc = Localization::new("en");
        loc.add("menu.start", "Start Game");

        assert_eq!(loc.get_or_key(&LocalizationKey::new("menu.start")), "Start Game");
        assert_eq!(loc.get_or_key(&LocalizationKey::new("menu.quit")), "menu.quit");
    }

    #[test]
    fn test_localization_len() {
        let mut loc = Localization::new("en");
        assert_eq!(loc.len(), 0);
        assert!(loc.is_empty());

        loc.add("menu.start", "Start Game");
        assert_eq!(loc.len(), 1);
        assert!(!loc.is_empty());
    }
}
