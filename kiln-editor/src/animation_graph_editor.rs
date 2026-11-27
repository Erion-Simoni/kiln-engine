//! Inspector graph drafts commit through existing scene component transactions.
use kiln_core::{
    animation_graph::{AnimationGraph, AnimationTransition, Comparison},
    Clip, Component, Entity,
};
use std::collections::HashMap;
#[derive(Debug, Default)]
pub struct GraphEditorState {
    owner: String,
    source: String,
    draft: AnimationGraph,
    new_state: String,
    new_parameter: String,
    error: String,
}
/// Edit a persistent draft. Only explicit, validated Apply returns a graph.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut GraphEditorState,
    entity: &Entity,
    clips: &HashMap<String, Clip>,
    scope: &str,
) -> Option<AnimationGraph> {
    let mut names: Vec<_> = clips.keys().cloned().collect();
    names.sort();
    let authored = entity.components.iter().find_map(|c| match c {
        Component::AnimationGraph { graph } => Some(graph),
        _ => None,
    });
    let source = serde_json::to_string(&authored).unwrap_or_default();
    let owner = format!("{scope}:{}", entity.id);
    if state.owner != owner || state.source != source {
        state.owner = owner;
        state.source = source;
        state.draft = authored
            .cloned()
            .unwrap_or_else(|| AnimationGraph::idle(names.first().cloned().unwrap_or_default()));
        state.error.clear();
    }
    let mut applied = None;
    ui.collapsing("Animation States", |ui| {
        if names.is_empty() {
            ui.label("Create clips in Animation Editor first.");
            return;
        }
        if ui.button("Reset graph draft").clicked() {
            state.draft = authored
                .cloned()
                .unwrap_or_else(|| AnimationGraph::idle(names[0].clone()));
            state.error.clear();
        }
        let graph = &mut state.draft;
        let states: Vec<_> = graph.states.keys().cloned().collect();
        egui::ComboBox::from_id_salt("graph_entry")
            .selected_text(format!("Entry: {}", graph.entry))
            .show_ui(ui, |ui| {
                for name in &states {
                    ui.selectable_value(&mut graph.entry, name.clone(), name);
                }
            });
        let mut remove = None;
        for (name, clip) in &mut graph.states {
            ui.push_id(name, |ui| {
                ui.horizontal(|ui| {
                    ui.label(name);
                    egui::ComboBox::from_id_salt("clip")
                        .selected_text(clip.as_str())
                        .show_ui(ui, |ui| {
                            for name in &names {
                                ui.selectable_value(clip, name.clone(), name);
                            }
                        });
                    if ui.small_button("Remove").clicked() {
                        remove = Some(name.clone());
                    }
                });
            });
        }
        if let Some(name) = remove {
            graph.states.remove(&name);
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.new_state);
            if ui.button("Add state").clicked()
                && !state.new_state.trim().is_empty()
                && graph.states.len() < 64
            {
                graph
                    .states
                    .entry(state.new_state.trim().into())
                    .or_insert(names[0].clone());
                state.new_state.clear();
            }
        });
        ui.label("Parameters (boolean scripts use 0 / 1)");
        let mut remove = None;
        for (name, value) in &mut graph.parameters {
            ui.horizontal(|ui| {
                ui.label(name);
                ui.add(egui::DragValue::new(value).speed(0.05));
                if ui.small_button("Remove parameter").clicked() {
                    remove = Some(name.clone());
                }
            });
        }
        if let Some(name) = remove {
            graph.parameters.remove(&name);
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.new_parameter);
            if ui.button("Add parameter").clicked()
                && !state.new_parameter.trim().is_empty()
                && graph.parameters.len() < 64
            {
                graph
                    .parameters
                    .entry(state.new_parameter.trim().into())
                    .or_insert(0.0);
                state.new_parameter.clear();
            }
        });
        ui.label("Transitions: first match wins");
        let mut remove = None;
        let mut up = None;
        for (index, edge) in graph.transitions.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("from")
                            .selected_text(format!("From: {}", edge.from))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut edge.from, "*".into(), "Any state");
                                for name in &states {
                                    ui.selectable_value(&mut edge.from, name.clone(), name);
                                }
                            });
                        egui::ComboBox::from_id_salt("to")
                            .selected_text(format!("To: {}", edge.to))
                            .show_ui(ui, |ui| {
                                for name in &states {
                                    ui.selectable_value(&mut edge.to, name.clone(), name);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("comparison")
                            .selected_text(format!("{:?}", edge.comparison))
                            .show_ui(ui, |ui| {
                                for value in [
                                    Comparison::Always,
                                    Comparison::Greater,
                                    Comparison::Less,
                                    Comparison::Equal,
                                    Comparison::NotEqual,
                                ] {
                                    ui.selectable_value(
                                        &mut edge.comparison,
                                        value,
                                        format!("{value:?}"),
                                    );
                                }
                            });
                        egui::ComboBox::from_id_salt("parameter")
                            .selected_text(if edge.parameter.is_empty() {
                                "Parameter"
                            } else {
                                &edge.parameter
                            })
                            .show_ui(ui, |ui| {
                                for name in graph.parameters.keys() {
                                    ui.selectable_value(&mut edge.parameter, name.clone(), name);
                                }
                            });
                        ui.add(egui::DragValue::new(&mut edge.threshold).speed(0.05));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut edge.after_finished, "After clip ends");
                        if index > 0 && ui.small_button("Move up").clicked() {
                            up = Some(index);
                        }
                        if ui.small_button("Remove transition").clicked() {
                            remove = Some(index);
                        }
                    });
                });
            });
        }
        if let Some(index) = remove {
            graph.transitions.remove(index);
        } else if let Some(index) = up {
            graph.transitions.swap(index, index - 1);
        }
        if ui.button("Add transition").clicked() && graph.transitions.len() < 256 {
            graph.transitions.push(AnimationTransition {
                from: graph.entry.clone(),
                to: states
                    .iter()
                    .find(|name| **name != graph.entry)
                    .cloned()
                    .unwrap_or_else(|| graph.entry.clone()),
                ..Default::default()
            });
        }
        if ui.button("Apply Animation States").clicked() {
            match graph.validate(clips) {
                Ok(()) => {
                    applied = Some(graph.clone());
                    state.error.clear();
                }
                Err(error) => state.error = error,
            }
        }
        if !state.error.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, &state.error);
        }
    });
    applied
}
