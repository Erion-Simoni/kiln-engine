//! Saved atlas recipes. Outputs use the ordinary PNG and animation-clip formats.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtlasRecipe {
    pub output: String,
    pub size: u32,
    pub padding: u32,
    /// Stable labels determine packing order and generated clip identities.
    pub sources: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub built_hash: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub built_regions: BTreeMap<String, [u32; 4]>,
}

impl Default for AtlasRecipe {
    fn default() -> Self {
        Self {
            output: "art/atlas.png".into(),
            size: 512,
            padding: 2,
            sources: BTreeMap::new(),
            built_hash: String::new(),
            built_regions: BTreeMap::new(),
        }
    }
}

/// Generated clips retain their identity when a source asset moves.
pub fn clip_name(atlas: &str, label: &str) -> String {
    format!("atlas:{atlas}:{label}")
}

/// Static clip generated from a whole source image packed at `rect`.
pub fn clip(atlas: &str, label: &str, output: &str, rect: [u32; 4]) -> crate::Clip {
    let mut clip = crate::Clip::new(clip_name(atlas, label), output.into());
    clip.frames.push(crate::Frame {
        rect,
        pivot: [0.5; 2],
        duration: 0.1,
        events: vec![],
    });
    clip
}
