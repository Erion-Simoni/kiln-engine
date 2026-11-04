//! Scene documents, open tabs, saved baselines, and isolated undo histories.

use crate::history::SceneHistory;
use kiln_core::{Project, Scene};
use std::collections::HashMap;

/// A visible scene tab. Closing a tab does not delete its document.
#[derive(Debug, Clone)]
pub struct SceneTab {
    /// Stable key of the scene document.
    pub scene_id: String,
    /// Whether this document differs from the last successful save.
    pub dirty: bool,
}

/// Editor-only state, independent of the game's startup scene.
#[derive(Debug, Default)]
pub struct SceneWorkspace {
    /// Visible scene documents in tab order.
    pub tabs: Vec<SceneTab>,
    /// Active tab index, or none for an empty project.
    pub active: Option<usize>,
    /// Undo/redo history of the active scene.
    pub history: SceneHistory,
    histories: HashMap<String, SceneHistory>,
    saved: HashMap<String, Scene>,
}

impl SceneWorkspace {
    /// Record resource-panel edits in the same scene-scoped command history.
    pub fn record_external_edit(&mut self, before: Scene) {
        if self.active_id() == Some(before.id.as_str()) {
            self.history.record(before);
        } else {
            self.histories
                .entry(before.id.clone())
                .or_default()
                .record(before);
        }
    }
    /// Use a disk-loaded project as the clean baseline for recovered documents.
    pub fn set_saved_baselines(&mut self, project: &Project) {
        self.saved = project.scenes.clone();
    }

    /// Open all documents, selecting the game's startup scene initially.
    pub fn open(project: &Project) -> Self {
        let mut ids: Vec<_> = project.scenes.keys().cloned().collect();
        ids.sort();
        let active = project
            .default_scene_id()
            .and_then(|id| ids.iter().position(|candidate| candidate == &id));
        Self {
            tabs: ids
                .into_iter()
                .map(|scene_id| SceneTab {
                    scene_id,
                    dirty: false,
                })
                .collect(),
            active,
            saved: project.scenes.clone(),
            ..Self::default()
        }
    }

    /// The scene whose entities are projected into the editor world.
    pub fn active_id(&self) -> Option<&str> {
        self.active
            .and_then(|index| self.tabs.get(index))
            .map(|tab| tab.scene_id.as_str())
    }

    /// Reconcile the outgoing projection before changing scene or entering Play.
    pub fn capture(&self, project: &mut Project, current: Scene) -> Result<(), String> {
        let id = self.active_id().ok_or("No active scene")?;
        if current.id != id || !project.scenes.contains_key(id) {
            return Err("The edited world does not match the active scene".into());
        }
        project.scenes.insert(id.into(), current);
        Ok(())
    }

    /// Store outgoing edits and activate another document with its own history.
    pub fn switch(
        &mut self,
        project: &mut Project,
        current: Scene,
        index: usize,
    ) -> Result<(), String> {
        let target = self
            .tabs
            .get(index)
            .ok_or("Unknown scene tab")?
            .scene_id
            .clone();
        if !project.scenes.contains_key(&target) {
            return Err("Scene document is missing".into());
        }
        self.capture(project, current)?;
        if self.active == Some(index) {
            return Ok(());
        }
        if let Some(id) = self.active_id().map(str::to_owned) {
            self.histories.insert(id, std::mem::take(&mut self.history));
        }
        self.history = self.histories.remove(&target).unwrap_or_default();
        self.active = Some(index);
        Ok(())
    }

    /// Reopen a closed document, restoring its retained history.
    pub fn open_scene(
        &mut self,
        project: &mut Project,
        current: Scene,
        id: &str,
    ) -> Result<(), String> {
        if !project.scenes.contains_key(id) {
            return Err("Unknown scene document".into());
        }
        if let Some(index) = self.tabs.iter().position(|tab| tab.scene_id == id) {
            return self.switch(project, current, index);
        }
        self.capture(project, current.clone())?;
        self.tabs.push(SceneTab {
            scene_id: id.into(),
            dirty: false,
        });
        self.switch(project, current, self.tabs.len() - 1)
    }

    /// Create a unique scene document and activate it without losing edits.
    pub fn create(&mut self, project: &mut Project, current: Option<Scene>) -> Result<(), String> {
        if self.active_id().is_some() {
            self.capture(project, current.ok_or("No edited scene to preserve")?)?;
        }
        let mut number = 1;
        while project.scenes.contains_key(&format!("scene_{number}")) {
            number += 1;
        }
        let id = format!("scene_{number}");
        if let Some(active) = self.active_id().map(str::to_owned) {
            self.histories
                .insert(active, std::mem::take(&mut self.history));
        }
        project.scenes.insert(
            id.clone(),
            Scene {
                id: id.clone(),
                name: format!("Scene {number}"),
                entities: Vec::new(),
                script: None,
                nested_scenes: Vec::new(),
            },
        );
        self.tabs.push(SceneTab {
            scene_id: id,
            dirty: true,
        });
        self.active = Some(self.tabs.len() - 1);
        self.history = SceneHistory::new();
        Ok(())
    }

    /// Compare documents with the last successful disk write, including live edits.
    pub fn refresh_dirty(&mut self, project: &Project, current: Option<&Scene>) {
        let active = self.active_id().map(str::to_owned);
        for tab in &mut self.tabs {
            let scene = current
                .filter(|s| active.as_deref() == Some(&tab.scene_id) && s.id == tab.scene_id)
                .or_else(|| project.scenes.get(&tab.scene_id));
            tab.dirty = match (scene, self.saved.get(&tab.scene_id)) {
                (Some(scene), Some(saved)) => {
                    serde_json::to_value(scene).ok() != serde_json::to_value(saved).ok()
                }
                _ => true,
            };
        }
    }

    /// Clear dirty indicators only after a successful project save.
    pub fn mark_saved(&mut self, project: &Project) {
        self.saved = project.scenes.clone();
        self.refresh_dirty(project, None);
    }

    /// Close a clean tab, retaining its scene and history. Never discard edits.
    pub fn close(
        &mut self,
        project: &mut Project,
        current: Scene,
        index: usize,
    ) -> Result<(), String> {
        if index >= self.tabs.len() {
            return Err("Unknown scene tab".into());
        }
        self.refresh_dirty(project, Some(&current));
        if self.tabs[index].dirty {
            return Err("Save the project before closing this scene tab".into());
        }
        if self.tabs.len() == 1 {
            return Err("Keep at least one scene tab open".into());
        }
        let active = self.active.ok_or("No active scene")?;
        if active == index {
            let target = if index == 0 { 1 } else { index - 1 };
            self.switch(project, current, target)?;
        } else {
            self.capture(project, current)?;
        }
        self.tabs.remove(index);
        if let Some(active) = self.active.as_mut() {
            if *active > index {
                *active -= 1;
            }
        }
        Ok(())
    }

    /// Discard a tab's edits to its last saved baseline and close it.
    /// Newly created scenes with no saved baseline are removed from the project.
    pub fn discard_and_close(
        &mut self,
        project: &mut Project,
        current: Scene,
        index: usize,
    ) -> Result<(), String> {
        if index >= self.tabs.len() {
            return Err("Unknown scene tab".into());
        }
        if self.tabs.len() == 1 {
            return Err("Keep at least one scene tab open".into());
        }
        let scene_id = self.tabs[index].scene_id.clone();
        let active_index = self.active.ok_or("No active scene")?;
        if active_index == index {
            let target = if index == 0 { 1 } else { index - 1 };
            self.switch(project, current, target)?;
        } else {
            self.capture(project, current)?;
        }

        if let Some(saved) = self.saved.get(&scene_id).cloned() {
            project.scenes.insert(scene_id.clone(), saved);
        } else {
            project.scenes.remove(&scene_id);
            self.saved.remove(&scene_id);
        }
        self.histories.remove(&scene_id);
        self.tabs.remove(index);
        if let Some(active) = self.active.as_mut() {
            if *active > index {
                *active -= 1;
            }
        }
        Ok(())
    }
}
