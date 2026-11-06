//! Logical menu state without pointer captures or device input.
use super::UiRuntime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSnapshot {
    pub active: String,
    pub stack: Vec<(String, Option<String>)>,
    pub focus: Option<String>,
    pub widgets: BTreeMap<String, Vec<WidgetSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSnapshot {
    pub id: String,
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
}

impl UiRuntime {
    /// Capture screen navigation and supported dynamic widget overrides.
    pub fn snapshot(&self) -> UiSnapshot {
        UiSnapshot {
            active: self.active.clone(),
            stack: self.stack.clone(),
            focus: self.focus.clone(),
            widgets: self
                .document
                .screens
                .iter()
                .map(|(id, screen)| {
                    (
                        id.clone(),
                        screen
                            .widgets
                            .iter()
                            .map(|w| WidgetSnapshot {
                                id: w.id.clone(),
                                text: w.text.clone(),
                                visible: w.visible,
                                enabled: w.enabled,
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }

    /// Validate all overrides before installing them. Held input never survives load.
    pub fn restore_snapshot(&mut self, saved: &UiSnapshot) -> Result<(), String> {
        let mut candidate = self.clone();
        if saved.stack.len() > 32 || saved.widgets.len() > candidate.document.screens.len() {
            return Err("Saved UI exceeds navigation/screen limits".into());
        }
        let valid_focus = |screen: &str, focus: &Option<String>| {
            (screen.is_empty() && focus.is_none())
                || candidate.document.screens.get(screen).is_some_and(|s| {
                    focus
                        .as_ref()
                        .is_none_or(|id| s.widgets.iter().any(|w| &w.id == id))
                })
        };
        if !valid_focus(&saved.active, &saved.focus)
            || saved.stack.iter().any(|(s, f)| !valid_focus(s, f))
        {
            return Err("Saved UI references a missing screen or widget".into());
        }
        for (id, widgets) in &saved.widgets {
            let screen = candidate
                .document
                .screens
                .get_mut(id)
                .ok_or("Saved UI screen is missing")?;
            if widgets.len() > screen.widgets.len() {
                return Err("Saved UI widget limit exceeded".into());
            }
            let mut seen = std::collections::HashSet::new();
            for saved in widgets {
                if !seen.insert(&saved.id) {
                    return Err("Duplicate saved UI widget".into());
                }
                let widget = screen
                    .widgets
                    .iter_mut()
                    .find(|w| w.id == saved.id)
                    .ok_or("Saved UI widget is missing")?;
                widget.text.clone_from(&saved.text);
                widget.visible = saved.visible;
                widget.enabled = saved.enabled;
            }
        }
        candidate.document.validate()?;
        candidate.active.clone_from(&saved.active);
        candidate.stack.clone_from(&saved.stack);
        candidate.focus.clone_from(&saved.focus);
        candidate.hovered = None;
        candidate.pressed = None;
        candidate.clicked.clear();
        candidate.reset_input();
        candidate.previous = super::UiInput {
            pointer_down: true,
            up: true,
            down: true,
            left: true,
            right: true,
            next: true,
            previous: true,
            accept: true,
            cancel: true,
            ..Default::default()
        };
        candidate.repair_focus();
        *self = candidate;
        Ok(())
    }
}
