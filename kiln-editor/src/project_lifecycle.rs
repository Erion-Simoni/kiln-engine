//! Unsaved-document protection, independent of the window and GPU.

use kiln_core::Project;
use std::path::PathBuf;

/// An operation that may replace the current authoring document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectAction {
    Open(PathBuf),
    Create {
        name: String,
        path: PathBuf,
        template: String,
    },
    Exit,
}

/// The explicit choice made in the unsaved-project dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveDecision {
    Save,
    Discard,
    Cancel,
}

/// Retains the last successful disk baseline and one pending transition.
#[derive(Debug, Default)]
pub struct ProjectLifecycle {
    saved: Option<Project>,
    pending: Option<ProjectAction>,
    save_error: Option<String>,
}

impl ProjectLifecycle {
    /// Record a successfully loaded or written authoring document.
    pub fn mark_saved(&mut self, project: &Project) {
        self.saved = Some(project.clone());
    }

    /// Compare every serialized property, independent of scene-tab visibility.
    pub fn is_dirty(&self, project: Option<&Project>) -> Result<bool, String> {
        let Some(project) = project else {
            return Ok(false);
        };
        let Some(saved) = &self.saved else {
            return Ok(true);
        };
        let current = serde_json::to_value(project).map_err(|e| e.to_string())?;
        let saved = serde_json::to_value(saved).map_err(|e| e.to_string())?;
        Ok(current != saved)
    }

    /// The operation awaiting a decision. Repeated requests cannot replace it.
    pub fn pending(&self) -> Option<&ProjectAction> {
        self.pending.as_ref()
    }

    /// The most recent failed Save attempt, shown in the same dialog.
    pub fn save_error(&self) -> Option<&str> {
        self.save_error.as_deref()
    }

    /// Return clean transitions immediately; otherwise retain them for a prompt.
    pub fn request(
        &mut self,
        action: ProjectAction,
        project: Option<&Project>,
    ) -> Result<Option<ProjectAction>, String> {
        if self.pending.is_some() {
            return Ok(None);
        }
        if self.is_dirty(project)? {
            self.pending = Some(action);
            self.save_error = None;
            Ok(None)
        } else {
            Ok(Some(action))
        }
    }

    /// Resolve a prompt. Save must finish successfully before releasing its action.
    /// Callers supply the actual write result; on failure the prompt stays pending.
    pub fn resolve(
        &mut self,
        decision: SaveDecision,
        save_result: Result<(), String>,
    ) -> Result<Option<ProjectAction>, String> {
        if self.pending.is_none() {
            return Ok(None);
        }
        if decision == SaveDecision::Cancel {
            self.pending = None;
            self.save_error = None;
            return Ok(None);
        }
        if decision == SaveDecision::Save {
            if let Err(error) = save_result {
                self.save_error = Some(error.clone());
                return Err(error);
            }
        }
        self.save_error = None;
        Ok(self.pending.take())
    }
}
