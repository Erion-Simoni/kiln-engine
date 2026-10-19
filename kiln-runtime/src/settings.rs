//! Graphics settings for runtime configuration.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Graphics resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    Custom { width: u32, height: u32 },
    HD,
    FullHD,
    QHD,
    UHD,
}

impl Resolution {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Resolution::Custom { width, height } => (*width, *height),
            Resolution::HD => (1280, 720),
            Resolution::FullHD => (1920, 1080),
            Resolution::QHD => (2560, 1440),
            Resolution::UHD => (3840, 2160),
        }
    }

    pub fn from_dimensions(width: u32, height: u32) -> Self {
        match (width, height) {
            (1280, 720) => Resolution::HD,
            (1920, 1080) => Resolution::FullHD,
            (2560, 1440) => Resolution::QHD,
            (3840, 2160) => Resolution::UHD,
            _ => Resolution::Custom { width, height },
        }
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::Custom { width, height } => write!(f, "{}x{}", width, height),
            Resolution::HD => write!(f, "HD (1280x720)"),
            Resolution::FullHD => write!(f, "Full HD (1920x1080)"),
            Resolution::QHD => write!(f, "QHD (2560x1440)"),
            Resolution::UHD => write!(f, "UHD (3840x2160)"),
        }
    }
}

/// Graphics settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: Resolution,
    pub vsync: bool,
    pub frame_cap: Option<u32>,
    pub tier_override: Option<String>,
}

impl GraphicsSettings {
    pub fn new() -> Self {
        Self {
            resolution: Resolution::FullHD,
            vsync: true,
            frame_cap: None,
            tier_override: None,
        }
    }

    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn with_frame_cap(mut self, frame_cap: Option<u32>) -> Self {
        self.frame_cap = frame_cap;
        self
    }

    pub fn with_tier_override(mut self, tier: Option<String>) -> Self {
        self.tier_override = tier;
        self
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_dimensions() {
        assert_eq!(Resolution::HD.dimensions(), (1280, 720));
        assert_eq!(Resolution::FullHD.dimensions(), (1920, 1080));
        assert_eq!(Resolution::Custom { width: 800, height: 600 }.dimensions(), (800, 600));
    }

    #[test]
    fn test_resolution_from_dimensions() {
        assert_eq!(Resolution::from_dimensions(1280, 720), Resolution::HD);
        assert_eq!(Resolution::from_dimensions(1920, 1080), Resolution::FullHD);
        assert_eq!(Resolution::from_dimensions(800, 600), Resolution::Custom { width: 800, height: 600 });
    }

    #[test]
    fn test_resolution_display() {
        assert!(Resolution::HD.to_string().contains("1280x720"));
        assert!(Resolution::FullHD.to_string().contains("1920x1080"));
    }

    #[test]
    fn test_graphics_settings_new() {
        let settings = GraphicsSettings::new();
        assert_eq!(settings.resolution, Resolution::FullHD);
        assert!(settings.vsync);
        assert!(settings.frame_cap.is_none());
        assert!(settings.tier_override.is_none());
    }

    #[test]
    fn test_graphics_settings_with_resolution() {
        let settings = GraphicsSettings::new().with_resolution(Resolution::HD);
        assert_eq!(settings.resolution, Resolution::HD);
    }

    #[test]
    fn test_graphics_settings_with_vsync() {
        let settings = GraphicsSettings::new().with_vsync(false);
        assert!(!settings.vsync);
    }

    #[test]
    fn test_graphics_settings_with_frame_cap() {
        let settings = GraphicsSettings::new().with_frame_cap(Some(60));
        assert_eq!(settings.frame_cap, Some(60));
    }

    #[test]
    fn test_graphics_settings_serialization() {
        let settings = GraphicsSettings::new().with_resolution(Resolution::HD).with_vsync(false);
        let json = settings.to_json().unwrap();
        assert!(json.contains("HD"));

        let deserialized = GraphicsSettings::from_json(&json).unwrap();
        assert_eq!(deserialized.resolution, Resolution::HD);
        assert!(!deserialized.vsync);
    }
}
