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
        for edge in &self.transitions {
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
