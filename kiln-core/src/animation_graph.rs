//! Serializable parameter-driven sprite animation state machines.
use crate::Clip;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
/// Comparison against a declared numeric parameter; booleans use zero/one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    #[default]
    Always,
    Greater,
    Less,
    Equal,
    NotEqual,
}
impl Comparison {
    /// Evaluate a finite parameter value against a finite threshold.
    pub fn matches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Always => true,
            Self::Greater => value > threshold,
            Self::Less => value < threshold,
            Self::Equal => value == threshold,
            Self::NotEqual => value != threshold,
        }
    }
}
/// Ordered edge; '*' as source matches any state except its own destination.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationTransition {
    pub from: String,
    pub to: String,
    pub parameter: String,
    pub comparison: Comparison,
    pub threshold: f64,
    pub after_finished: bool,
    pub blend_seconds: f64,
}
/// Adjacent clips are mixed automatically using a declared graph parameter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlendTree {
    pub parameter: String,
    pub points: Vec<BlendPoint>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlendPoint {
    pub threshold: f64,
    pub clip: String,
}
impl BlendTree {
    pub fn samples(&self, value: f64) -> (&str, &str, f32) {
        for pair in self.points.windows(2) {
            if value < pair[1].threshold {
                let w = ((value - pair[0].threshold) / (pair[1].threshold - pair[0].threshold))
                    .clamp(0.0, 1.0);
                return (&pair[0].clip, &pair[1].clip, w as f32);
            }
        }
        let last = self.points.last().expect("validated tree");
        (&last.clip, &last.clip, 0.0)
    }
}
/// Per-entity graph; clips and texture assets remain project resources.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationGraph {
    pub entry: String,
    /// State name to clip name.
    pub states: BTreeMap<String, String>,
    pub parameters: BTreeMap<String, f64>,
    pub transitions: Vec<AnimationTransition>,
    pub blend_trees: BTreeMap<String, BlendTree>,
}
impl AnimationGraph {
    /// A single idle state ready for editing with project clips.
    pub fn idle(clip: String) -> Self {
        Self {
            entry: "idle".into(),
            states: BTreeMap::from([("idle".into(), clip)]),
            ..Default::default()
        }
    }
    /// Validate before authoring, export, binding or restoring logical state.
    pub fn validate(&self, clips: &HashMap<String, Clip>) -> Result<(), String> {
        let name = |s: &str| !s.trim().is_empty() && s.len() <= 64 && s != "*";
        if self.states.is_empty()
            || self.states.len() > 64
            || self.parameters.len() > 64
            || self.transitions.len() > 256
            || !self.states.contains_key(&self.entry)
        {
            return Err("Animation graph needs 1..64 states, a valid entry, at most 64 parameters and 256 transitions".into());
        }
        for (state, clip) in &self.states {
            if !name(state) || !clips.contains_key(clip) || clips[clip].name != *clip {
                return Err(format!(
                    "State '{state}' needs a valid name and existing clip"
                ));
            }
            if clips[clip].next.is_some() {
                return Err(
                    "Graph clips cannot also use a next clip; author a finish transition".into(),
                );
            }
        }
        if self
            .parameters
            .iter()
            .any(|(key, value)| !name(key) || !value.is_finite())
        {
            return Err("Animation parameters need valid names and finite defaults".into());
        }
        if self.blend_trees.len() > 64 {
            return Err("At most 64 state blend trees".into());
        }
        for (state, tree) in &self.blend_trees {
            if !self.states.contains_key(state)
                || !self.parameters.contains_key(&tree.parameter)
                || tree.points.len() < 2
                || tree.points.len() > 16
            {
                return Err("Blend tree needs an existing state/parameter and 2..16 points".into());
            }
            let mut previous = f64::NEG_INFINITY;
            for point in &tree.points {
                if !point.threshold.is_finite()
                    || point.threshold <= previous
                    || !clips.contains_key(&point.clip)
                    || clips[&point.clip].name != point.clip
                    || clips[&point.clip].next.is_some()
                {
                    return Err("Blend thresholds must increase and reference existing clips without next edges".into());
                }
                previous = point.threshold;
            }
        }
        for edge in &self.transitions {
            if !edge.blend_seconds.is_finite() || !(0.0..=60.0).contains(&edge.blend_seconds) {
                return Err("Crossfade duration must be 0..60".into());
            }
            if edge.blend_seconds > 0.0
                && (self.blend_trees.contains_key(&edge.to)
                    || self.blend_trees.contains_key(&edge.from)
                    || (edge.from == "*" && !self.blend_trees.is_empty()))
            {
                return Err(
                    "Timed crossfades require plain clip states; trees already mix two textures"
                        .into(),
                );
            }
            if (edge.from != "*" && !self.states.contains_key(&edge.from))
                || !self.states.contains_key(&edge.to)
                || !edge.threshold.is_finite()
                || (edge.comparison != Comparison::Always
                    && !self.parameters.contains_key(&edge.parameter))
            {
                return Err(
                    "Animation transition needs existing states/parameter and a finite threshold"
                        .into(),
                );
            }
            if edge.after_finished && (edge.from == "*" || clips[&self.states[&edge.from]].loops) {
                return Err("Finish transitions require a specific one-shot source state".into());
            }
        }
        Ok(())
    }
}
