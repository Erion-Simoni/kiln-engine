//! Undo and redo for the editor.
//!
//! The editor edits the scene as a whole (see [`crate::scene_ops`]), so the
//! history stores whole scenes rather than individual commands. That keeps
//! undo correct for every kind of edit — moving an entity, adding a
//! component, deleting an entity — without a command per operation.

use crate::scene_ops::Scene;

/// The undo and redo stacks of the editor.
#[derive(Debug, Clone)]
pub struct History {
    /// Scenes that can be restored by undo, oldest first.
    undo: Vec<Scene>,
    /// Scenes that can be restored by redo, newest first.
    redo: Vec<Scene>,
    /// Whether the next edit is recorded.
    enabled: bool,
    /// The key of the last recorded edit, used to coalesce a drag.
    last_key: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            enabled: true,
            last_key: None,
        }
    }
}

impl History {
    /// A history with nothing recorded.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether undo can restore something.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether redo can restore something.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Record the scene as it was before an edit.
    ///
    /// Recording a new edit clears the redo stack, because the redo stack
    /// describes a future that no longer happens.
    pub fn record(&mut self, scene: &Scene) {
        self.record_with_key(None, scene);
    }

    /// Record the scene as it was before an edit, with a key.
    ///
    /// While the same key is recorded again in a row, the first snapshot is
    /// kept. That way a slider drag is one undo step and not one per frame.
    pub fn record_with_key(&mut self, key: Option<&str>, scene: &Scene) {
        if !self.enabled {
            return;
        }
        if key.is_some() && key == self.last_key.as_deref() {
            // The same edit is still going on, so the first snapshot stands.
            return;
        }
        self.last_key = key.map(str::to_string);
        self.undo.push(scene.clone());
        self.redo.clear();
    }

    /// Restore the previous scene, if there is one.
    ///
    /// The current scene is pushed onto the redo stack first, so the edit can
    /// be brought back.
    pub fn undo(&mut self, current: &Scene) -> Option<Scene> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(previous)
    }

    /// Restore the scene that was undone, if there is one.
    pub fn redo(&mut self, current: &Scene) -> Option<Scene> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(next)
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Stop recording edits.
    ///
    /// The editor uses this while the game is playing, so that the running
    /// game does not push the edited scene onto the undo stack.
    pub fn suspend(&mut self) {
        self.enabled = false;
    }

    /// Start recording edits again.
    pub fn resume(&mut self) {
        self.enabled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_ops::SceneEntity;

    fn scene_with(name: &str) -> Scene {
        let mut scene = Scene::new();
        scene.add_entity(SceneEntity::new(name, name));
        scene
    }

    #[test]
    fn a_new_history_has_nothing_to_undo() {
        let history = History::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_restores_the_previous_scene() {
        let mut history = History::new();
        let before = scene_with("hero");
        let after = scene_with("coin");

        history.record(&before);
        assert!(history.can_undo());

        let restored = history.undo(&after).expect("undo restores a scene");
        assert_eq!(restored, before);
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }

    #[test]
    fn redo_restores_the_scene_that_was_undone() {
        let mut history = History::new();
        let before = scene_with("hero");
        let after = scene_with("coin");

        history.record(&before);
        let restored = history.undo(&after).expect("undo");
        let redone = history.redo(&restored).expect("redo");

        assert_eq!(redone, after);
        assert!(history.can_undo());
    }

    #[test]
    fn undo_on_an_empty_history_does_nothing() {
        let mut history = History::new();
        assert!(history.undo(&scene_with("hero")).is_none());
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut history = History::new();
        let before = scene_with("hero");
        let after = scene_with("coin");

        history.record(&before);
        history.undo(&after);
        assert!(history.can_redo());

        history.record(&scene_with("gem"));
        assert!(!history.can_redo());
    }

    #[test]
    fn a_suspended_history_records_nothing() {
        let mut history = History::new();
        history.suspend();
        history.record(&scene_with("hero"));
        assert!(!history.can_undo());

        history.resume();
        history.record(&scene_with("hero"));
        assert!(history.can_undo());
    }

    #[test]
    fn clearing_forgets_everything() {
        let mut history = History::new();
        history.record(&scene_with("hero"));
        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn the_same_key_is_recorded_once() {
        let mut history = History::new();
        let before = scene_with("hero");

        // A drag records the same key on every frame.
        history.record_with_key(Some("transform.x"), &before);
        history.record_with_key(Some("transform.x"), &before);
        history.record_with_key(Some("transform.x"), &before);

        // One undo step for the whole drag.
        assert!(history.can_undo());
        let restored = history.undo(&before).expect("undo");
        assert!(!history.can_undo());
        assert_eq!(restored, before);
    }

    #[test]
    fn a_different_key_records_a_new_step() {
        let mut history = History::new();
        let before = scene_with("hero");

        history.record_with_key(Some("transform.x"), &before);
        history.record_with_key(Some("transform.y"), &before);

        assert!(history.can_undo());
        history.undo(&before).expect("undo");
        assert!(history.can_undo(), "the second edit is still undoable");
    }
}
