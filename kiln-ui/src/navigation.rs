//! Focus navigation system for gamepad and keyboard.

use crate::widget::WidgetId;
use serde::{Deserialize, Serialize};

/// Direction for focus navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
    Next,
    Previous,
}

/// Trait for widgets that can receive focus.
pub trait Focusable {
    /// Returns true if the widget can currently receive focus.
    fn can_focus(&self) -> bool;
}

/// Focus manager for handling navigation between focusable widgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusManager {
    focusable_widgets: Vec<WidgetId>,
    focused_widget: Option<WidgetId>,
    navigation_map: HashMap<(WidgetId, FocusDirection), WidgetId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focusable_widgets: Vec::new(),
            focused_widget: None,
            navigation_map: HashMap::new(),
        }
    }

    /// Registers a focusable widget.
    pub fn register(&mut self, widget_id: WidgetId) {
        if !self.focusable_widgets.contains(&widget_id) {
            self.focusable_widgets.push(widget_id);
        }
    }

    /// Unregisters a focusable widget.
    pub fn unregister(&mut self, widget_id: WidgetId) {
        self.focusable_widgets.retain(|&id| id != widget_id);
        if self.focused_widget == Some(widget_id) {
            self.focused_widget = None;
        }
        self.navigation_map.retain(|(from, _), _| *from != widget_id);
    }

    /// Sets a navigation link between two widgets.
    pub fn set_navigation(&mut self, from: WidgetId, direction: FocusDirection, to: WidgetId) {
        self.navigation_map.insert((from, direction), to);
    }

    /// Sets the currently focused widget.
    pub fn set_focused(&mut self, widget_id: Option<WidgetId>) {
        self.focused_widget = widget_id;
    }

    /// Returns the currently focused widget.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Moves focus in the specified direction.
    pub fn move_focus(&mut self, direction: FocusDirection) -> Option<WidgetId> {
        let current = self.focused_widget?;

        // Check navigation map first
        if let Some(&target) = self.navigation_map.get(&(current, direction)) {
            self.focused_widget = Some(target);
            return Some(target);
        }

        // Fallback to linear navigation for Next/Previous
        match direction {
            FocusDirection::Next => {
                let index = self.focusable_widgets.iter().position(|&id| id == current)?;
                let next_index = (index + 1) % self.focusable_widgets.len();
                let target = self.focusable_widgets[next_index];
                self.focused_widget = Some(target);
                Some(target)
            }
            FocusDirection::Previous => {
                let index = self.focusable_widgets.iter().position(|&id| id == current)?;
                let prev_index = if index == 0 {
                    self.focusable_widgets.len() - 1
                } else {
                    index - 1
                };
                let target = self.focusable_widgets[prev_index];
                self.focused_widget = Some(target);
                Some(target)
            }
            _ => None,
        }
    }

    /// Returns true if the given widget is focused.
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.focused_widget == Some(widget_id)
    }

    /// Clears the current focus.
    pub fn clear_focus(&mut self) {
        self.focused_widget = None;
    }

    /// Returns the number of focusable widgets.
    pub fn len(&self) -> usize {
        self.focusable_widgets.len()
    }

    /// Returns true if there are no focusable widgets.
    pub fn is_empty(&self) -> bool {
        self.focusable_widgets.is_empty()
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(value: u64) -> WidgetId {
        WidgetId::from_u64(value)
    }

    #[test]
    fn test_focus_manager_register() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        manager.register(id1);
        manager.register(id2);

        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_focus_manager_unregister() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        manager.register(id1);
        manager.register(id2);
        manager.unregister(id1);

        assert_eq!(manager.len(), 1);
        assert!(!manager.focusable_widgets.contains(&id1));
    }

    #[test]
    fn test_focus_manager_set_focused() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);

        manager.register(id1);
        manager.set_focused(Some(id1));

        assert_eq!(manager.focused(), Some(id1));
        assert!(manager.is_focused(id1));
    }

    #[test]
    fn test_focus_manager_clear_focus() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);

        manager.register(id1);
        manager.set_focused(Some(id1));
        manager.clear_focus();

        assert_eq!(manager.focused(), None);
        assert!(!manager.is_focused(id1));
    }

    #[test]
    fn test_focus_manager_linear_navigation() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);
        let id2 = make_id(2);
        let id3 = make_id(3);

        manager.register(id1);
        manager.register(id2);
        manager.register(id3);
        manager.set_focused(Some(id1));

        let next = manager.move_focus(FocusDirection::Next);
        assert_eq!(next, Some(id2));

        let next = manager.move_focus(FocusDirection::Next);
        assert_eq!(next, Some(id3));

        let next = manager.move_focus(FocusDirection::Next);
        assert_eq!(next, Some(id1)); // Wraps around
    }

    #[test]
    fn test_focus_manager_directional_navigation() {
        let mut manager = FocusManager::new();
        let id1 = make_id(1);
        let id2 = make_id(2);

        manager.register(id1);
        manager.register(id2);
        manager.set_navigation(id1, FocusDirection::Down, id2);
        manager.set_focused(Some(id1));

        let target = manager.move_focus(FocusDirection::Down);
        assert_eq!(target, Some(id2));
    }
}
