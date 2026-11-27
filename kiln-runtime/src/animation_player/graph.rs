//! Graph ownership, conditions and logical resume validation.
use super::*;
use kiln_core::animation_graph::{AnimationGraph, Comparison};
use std::collections::BTreeMap;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphSnapshot {
    pub definition: AnimationGraph,
    pub state: String,
    pub parameters: BTreeMap<String, f64>,
}
impl AnimationPlayer {
    /// Bind a validated graph and enter its initial state.
    pub fn bind_graph(&mut self, definition: AnimationGraph) -> Result<(), String> {
        definition.validate(&self.clips)?;
        for clip in definition.states.values() {
            let size = self
                .textures
                .get(clip)
                .ok_or("Animation graph clip has no texture binding")?
                .1;
            self.clips[clip].validate(size)?;
        }
        let state = definition.entry.clone();
        let clip = definition.states[&state].clone();
        let parameters = definition.parameters.clone();
        self.events.clear();
        self.enter_graph_clip(&clip);
        self.graph = Some(GraphSnapshot {
            definition,
            state,
            parameters,
        });
        Ok(())
    }
    /// Current authored state; None when manual play/stop owns the player.
    pub fn graph_state(&self) -> Option<&str> {
        self.graph.as_ref().map(|graph| graph.state.as_str())
    }
    /// Declared live parameters for script-fact snapshots.
    pub fn graph_parameters(&self) -> Option<&BTreeMap<String, f64>> {
        self.graph.as_ref().map(|graph| &graph.parameters)
    }
    /// Change a declared parameter without creating unbounded dynamic keys.
    pub fn set_graph_parameter(&mut self, name: &str, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Animation parameter must be finite".into());
        }
        let parameter = self
            .graph
            .as_mut()
            .and_then(|graph| graph.parameters.get_mut(name))
            .ok_or("Unknown animation parameter or detached graph")?;
        *parameter = value;
        Ok(())
    }
    fn enter_graph_clip(&mut self, clip: &str) {
        self.current = Some(clip.into());
        self.elapsed = 0.0;
        self.started = false;
    }
    pub(super) fn advance_graph(&mut self) {
        let Some(graph) = self.graph.as_ref() else {
            return;
        };
        let finished = self
            .current
            .as_ref()
            .and_then(|clip| self.clips.get(clip))
            .is_some_and(|clip| !clip.loops && self.elapsed >= clip.duration_seconds());
        let next = graph
            .definition
            .transitions
            .iter()
            .find(|edge| {
                (edge.from == "*" || edge.from == graph.state)
                    && edge.to != graph.state
                    && (!edge.after_finished || finished)
                    && (edge.comparison == Comparison::Always
                        || edge.comparison.matches(
                            *graph.parameters.get(&edge.parameter).unwrap_or(&0.0),
                            edge.threshold,
                        ))
            })
            .map(|edge| edge.to.clone());
        if let Some(state) = next {
            let clip = graph.definition.states[&state].clone();
            self.enter_graph_clip(&clip);
            self.graph.as_mut().unwrap().state = state;
        }
    }
    pub(super) fn restore_graph(
        &mut self,
        graph: Option<GraphSnapshot>,
        clip: Option<&str>,
    ) -> Result<(), String> {
        if let Some(graph) = &graph {
            graph.definition.validate(&self.clips)?;
            if !graph.definition.states.contains_key(&graph.state) || clip.is_none() {
                return Err("Invalid saved animation graph state".into());
            }
            if graph
                .parameters
                .keys()
                .ne(graph.definition.parameters.keys())
                || graph.parameters.values().any(|v| !v.is_finite())
                || graph
                    .definition
                    .states
                    .get(&graph.state)
                    .map(String::as_str)
                    != clip
            {
                return Err("Invalid saved animation graph state/parameters".into());
            }
        }
        self.graph = graph;
        Ok(())
    }
}
