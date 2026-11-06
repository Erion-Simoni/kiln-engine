//! Saved mixer defaults, shared by editor authoring and runtime playback.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Controls one flat bus routed through the master gain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioBus {
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
}

impl Default for AudioBus {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
            solo: false,
        }
    }
}

/// Project defaults or player overrides. SFX and Music are reserved bus names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioMixerSettings {
    pub master_volume: f32,
    pub buses: BTreeMap<String, AudioBus>,
}

impl Default for AudioMixerSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            buses: ["SFX", "Music"]
                .into_iter()
                .map(|name| (name.into(), AudioBus::default()))
                .collect(),
        }
    }
}

/// Finite, clamped gain; malformed numbers mute rather than poisoning output.
pub fn gain(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl AudioMixerSettings {
    /// Effective master/bus gain, respecting mute and exclusive solo routing.
    pub fn bus_gain(&self, name: &str) -> f32 {
        let solo = self.buses.values().any(|bus| bus.solo);
        let bus = self.buses.get(name);
        if bus.is_some_and(|bus| bus.muted) || (solo && !bus.is_some_and(|bus| bus.solo)) {
            return 0.0;
        }
        gain(self.master_volume) * bus.map_or(1.0, |bus| gain(bus.volume))
    }
}
