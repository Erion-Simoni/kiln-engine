//! Undo and redo for scene edits.
//!
//! The editor keeps a stack of scene snapshots. A snapshot is taken before an
//! edit, so undo restores the scene as it was and redo applies the edit
//! again.

use kiln_core::{camera::CameraSettings, project::Scene};

#[derive(Debug)]
struct Snapshot {
    scene: Scene,
    // None means an entity edit that must preserve the current camera.
    camera: Option<Option<CameraSettings>>,
}

/// The history of scene edits.
#[derive(Debug, Default)]
pub struct SceneHistory {
    /// The states that can be undone, oldest first.
    undo: Vec<Snapshot>,
    /// The states that can be redone, most recent first.
    redo: Vec<Snapshot>,
}

impl SceneHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the state of the scene before an edit.
    ///
    /// This clears the redo stack, because a new edit makes the undone states
    /// unreachable.
    pub fn record(&mut self, scene: Scene) {
        self.undo.push(Snapshot {
            scene,
            camera: None,
        });
        self.redo.clear();
    }

    /// The state to undo to, given the current state.
    ///
    /// The current state is pushed onto the redo stack, so the edit can be
    /// applied again.
    pub fn undo(&mut self, current: Scene) -> Option<Scene> {
        self.undo_document(current, None).map(|(scene, _)| scene)
    }

    /// Redo a scene-only edit. Camera-aware callers must use `redo_document`.
    pub fn redo(&mut self, current: Scene) -> Option<Scene> {
        self.redo_document(current, None).map(|(scene, _)| scene)
    }

    /// Record settings including the exact presence/value of this scene's camera.
    pub fn record_document(&mut self, scene: Scene, camera: Option<CameraSettings>) {
        self.undo.push(Snapshot {
            scene,
            camera: Some(camera),
        });
        self.redo.clear();
    }

    /// Undo settings and return an optional camera replacement; entity edits
    /// leave the current camera unchanged. Capture the current value for Redo.
    pub fn undo_document(
        &mut self,
        current: Scene,
        camera: Option<CameraSettings>,
    ) -> Option<(Scene, Option<Option<CameraSettings>>)> {
        let previous = self.undo.pop()?;
        self.redo.push(Snapshot {
            scene: current,
            camera: previous.camera.as_ref().map(|_| camera),
        });
        Some((previous.scene, previous.camera))
    }

    /// Redo with the same camera replacement contract as `undo_document`.
    pub fn redo_document(
        &mut self,
        current: Scene,
        camera: Option<CameraSettings>,
    ) -> Option<(Scene, Option<Option<CameraSettings>>)> {
        let next = self.redo.pop()?;
        self.undo.push(Snapshot {
            scene: current,
            camera: next.camera.as_ref().map(|_| camera),
        });
        Some((next.scene, next.camera))
    }

    /// Whether there is a state to undo to.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is a state to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The number of states that can be undone.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::project::{Entity, Scene};

    fn scene_with(name: &str) -> Scene {
        Scene {
            id: "main".to_string(),
            name: "Main".to_string(),
            entities: vec![Entity {
                id: "e1".to_string(),
                name: name.to_string(),
                components: vec![],
            }],
            nested_scenes: Vec::new(),
            script: None,
        }
    }

    #[test]
    fn a_new_history_has_nothing_to_undo() {
        let history = SceneHistory::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_restores_the_recorded_state() {
        let mut history = SceneHistory::new();
        history.record(scene_with("before"));
        let restored = history.undo(scene_with("after")).expect("undo");
        assert_eq!(restored.entities[0].name, "before");
    }

    #[test]
    fn redo_applies_the_undone_edit() {
        let mut history = SceneHistory::new();
        history.record(scene_with("before"));
        history.undo(scene_with("after"));
        let redone = history.redo(scene_with("before")).expect("redo");
        assert_eq!(redone.entities[0].name, "after");
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut history = SceneHistory::new();
        history.record(scene_with("first"));
        history.undo(scene_with("second"));
        assert!(history.can_redo());
        history.record(scene_with("third"));
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_on_an_empty_history_does_nothing() {
        let mut history = SceneHistory::new();
        assert!(history.undo(scene_with("current")).is_none());
    }

    #[test]
    fn the_undo_stack_grows_with_every_recorded_edit() {
        let mut history = SceneHistory::new();
        history.record(scene_with("one"));
        history.record(scene_with("two"));
        assert_eq!(history.undo_depth(), 2);
    }
}
