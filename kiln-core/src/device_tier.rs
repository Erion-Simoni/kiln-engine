//! Mobile device tier detection
//! Provides automatic tier selection (Low/Medium/High) based on device capabilities

use serde::{Deserialize, Serialize};

/// Render quality tier for device-based auto-selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceTier {
    /// Low-end devices (older phones, integrated GPUs)
    Low,
    /// Mid-range devices (most phones, mid-tier GPUs)
    #[default]
    Medium,
    /// High-end devices (flagship phones, discrete GPUs)
    High,
}

impl DeviceTier {
    /// Convert to string for external tier systems
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceTier::Low => "low",
            DeviceTier::Medium => "medium",
            DeviceTier::High => "high",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(DeviceTier::Low),
            "medium" => Some(DeviceTier::Medium),
            "high" => Some(DeviceTier::High),
            _ => None,
        }
    }
}

/// Device capability information for tier detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Total physical memory in MB
    pub memory_mb: u32,
    /// CPU core count
    pub cpu_cores: u32,
    /// GPU vendor name (if available)
    pub gpu_vendor: String,
    /// GPU model name (if available)
    pub gpu_model: String,
    /// Device model identifier (platform-specific)
    pub device_model: String,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            memory_mb: 2048,
            cpu_cores: 4,
            gpu_vendor: String::new(),
            gpu_model: String::new(),
            device_model: String::new(),
        }
    }
}

/// Detect device tier based on capabilities
pub fn detect_device_tier(capabilities: &DeviceCapabilities) -> DeviceTier {
    // Heuristic for tier selection based on device capabilities
    // These thresholds are conservative and can be adjusted based on testing

    let is_high_tier = capabilities.memory_mb >= 6144 // 6GB+ RAM
        && capabilities.cpu_cores >= 6
        && (capabilities.gpu_vendor.contains("Apple")
            || capabilities.gpu_vendor.contains("Adreno")
            || capabilities.gpu_vendor.contains("Mali"));

    let is_low_tier = capabilities.memory_mb < 3072 // <3GB RAM
        || capabilities.cpu_cores < 4;

    if is_high_tier {
        DeviceTier::High
    } else if is_low_tier {
        DeviceTier::Low
    } else {
        DeviceTier::Medium
    }
}

/// Get device capabilities (platform-specific)
#[cfg(target_os = "ios")]
pub fn get_device_capabilities() -> DeviceCapabilities {
    // iOS-specific implementation using sysinfo or platform APIs
    // For now, return defaults - actual implementation requires iOS-specific code
    DeviceCapabilities::default()
}

#[cfg(target_os = "android")]
pub fn get_device_capabilities() -> DeviceCapabilities {
    // Android-specific implementation using Android NDK APIs
    // For now, return defaults - actual implementation requires Android NDK
    DeviceCapabilities::default()
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn get_device_capabilities() -> DeviceCapabilities {
    // Desktop/web platforms use GPU-based detection from kiln-render
    // This is a stub for compatibility
    DeviceCapabilities::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_high_tier_device() {
        let capabilities = DeviceCapabilities {
            memory_mb: 8192,
            cpu_cores: 8,
            gpu_vendor: "Apple".to_string(),
            gpu_model: "A17 Pro".to_string(),
            device_model: "iPhone 15 Pro".to_string(),
        };

        let tier = detect_device_tier(&capabilities);
        assert_eq!(tier, DeviceTier::High);
    }

    #[test]
    fn detect_medium_tier_device() {
        let capabilities = DeviceCapabilities {
            memory_mb: 4096,
            cpu_cores: 6,
            gpu_vendor: "Adreno".to_string(),
            gpu_model: "Adreno 650".to_string(),
            device_model: "Pixel 5".to_string(),
        };

        let tier = detect_device_tier(&capabilities);
        assert_eq!(tier, DeviceTier::Medium);
    }

    #[test]
    fn detect_low_tier_device() {
        let capabilities = DeviceCapabilities {
            memory_mb: 2048,
            cpu_cores: 4,
            gpu_vendor: "Mali".to_string(),
            gpu_model: "Mali-G52".to_string(),
            device_model: "Low-end Android".to_string(),
        };

        let tier = detect_device_tier(&capabilities);
        assert_eq!(tier, DeviceTier::Low);
    }

    #[test]
    fn device_tier_string_conversion() {
        assert_eq!(DeviceTier::Low.as_str(), "low");
        assert_eq!(DeviceTier::Medium.as_str(), "medium");
        assert_eq!(DeviceTier::High.as_str(), "high");

        assert_eq!(DeviceTier::parse("low"), Some(DeviceTier::Low));
        assert_eq!(DeviceTier::parse("MEDIUM"), Some(DeviceTier::Medium));
        assert_eq!(DeviceTier::parse("High"), Some(DeviceTier::High));
        assert_eq!(DeviceTier::parse("invalid"), None);
    }

    #[test]
    fn device_capabilities_default() {
        let caps = DeviceCapabilities::default();
        assert_eq!(caps.memory_mb, 2048);
        assert_eq!(caps.cpu_cores, 4);
        assert!(caps.gpu_vendor.is_empty());
        assert!(caps.gpu_model.is_empty());
        assert!(caps.device_model.is_empty());
    }

    #[test]
    fn device_tier_default() {
        let tier = DeviceTier::default();
        assert_eq!(tier, DeviceTier::Medium);
    }
}
