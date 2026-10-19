//! Data-driven per-tier pass enablement (not hard-coded in registration).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::tier::RenderTier;

/// Which quality tiers enable a named pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassTierEnable {
    pub name: String,
    /// Tier names: "low" | "medium" | "high"
    pub tiers: Vec<String>,
}

/// Table of pass enables loaded from TOML/RON or built-in defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PassEnableConfig {
    pub passes: Vec<PassTierEnable>,
}

impl PassEnableConfig {
    /// Built-in defaults matching M9 tier behaviour (data, not scattered ifs).
    pub fn default_2d() -> Self {
        Self {
            passes: vec![
                PassTierEnable {
                    name: "main".into(),
                    tiers: vec!["low".into(), "medium".into(), "high".into()],
                },
                PassTierEnable {
                    name: "bloom".into(),
                    tiers: vec!["medium".into(), "high".into()],
                },
                PassTierEnable {
                    name: "fog".into(),
                    tiers: vec!["medium".into(), "high".into()],
                },
                PassTierEnable {
                    name: "outline".into(),
                    tiers: vec!["high".into()],
                },
                PassTierEnable {
                    name: "lut".into(),
                    tiers: vec!["high".into()],
                },
                PassTierEnable {
                    name: "composite".into(),
                    tiers: vec!["low".into(), "medium".into(), "high".into()],
                },
            ],
        }
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    fn tier_key(tier: RenderTier) -> &'static str {
        match tier {
            RenderTier::Low => "low",
            RenderTier::Medium => "medium",
            RenderTier::High => "high",
        }
    }

    /// Pass names enabled for this tier.
    pub fn enabled_for(&self, tier: RenderTier) -> HashSet<String> {
        let key = Self::tier_key(tier);
        self.passes
            .iter()
            .filter(|p| p.tiers.iter().any(|t| t.eq_ignore_ascii_case(key)))
            .map(|p| p.name.clone())
            .collect()
    }

    pub fn is_enabled(&self, pass: &str, tier: RenderTier) -> bool {
        self.enabled_for(tier).contains(pass)
    }

    /// Lookup map name → tiers for editor display.
    pub fn as_map(&self) -> HashMap<String, Vec<String>> {
        self.passes
            .iter()
            .map(|p| (p.name.clone(), p.tiers.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_low_disables_post() {
        let cfg = PassEnableConfig::default_2d();
        assert!(cfg.is_enabled("main", RenderTier::Low));
        assert!(cfg.is_enabled("composite", RenderTier::Low));
        assert!(!cfg.is_enabled("bloom", RenderTier::Low));
        assert!(!cfg.is_enabled("fog", RenderTier::Low));
        assert!(!cfg.is_enabled("outline", RenderTier::Low));
        assert!(!cfg.is_enabled("lut", RenderTier::Low));
    }

    #[test]
    fn default_high_enables_all_post() {
        let cfg = PassEnableConfig::default_2d();
        assert!(cfg.is_enabled("bloom", RenderTier::High));
        assert!(cfg.is_enabled("outline", RenderTier::High));
        assert!(cfg.is_enabled("lut", RenderTier::High));
    }

    #[test]
    fn roundtrip_toml() {
        let cfg = PassEnableConfig::default_2d();
        let s = cfg.to_toml().unwrap();
        let back = PassEnableConfig::from_toml(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn custom_data_overrides_code_defaults() {
        let toml = r#"
[[passes]]
name = "main"
tiers = ["low", "medium", "high"]

[[passes]]
name = "bloom"
tiers = ["high"]
"#;
        let cfg = PassEnableConfig::from_toml(toml).unwrap();
        assert!(!cfg.is_enabled("bloom", RenderTier::Medium));
        assert!(cfg.is_enabled("bloom", RenderTier::High));
    }
}
