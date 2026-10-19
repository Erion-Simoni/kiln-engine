//! UI renderer integration with kiln-render sprite batch.

use crate::layout::Rect;
use crate::render::NineSlice;
use crate::widget::{Button, Label, List, Panel};
use kiln_render::SpriteInstance;

/// UI renderer that converts widgets to sprite instances.
///
/// This is a stub that demonstrates the API for integrating UI widgets
/// with the kiln-render sprite batch. Full implementation will:
/// 1. Traverse the widget hierarchy
/// 2. Compute layout bounds
/// 3. Generate sprite instances for visible widgets
/// 4. Handle 9-slice sprites for scalable UI elements
/// 5. Add text quads for labels
pub struct UiRenderer {
    // Stub: holds render state
}

impl UiRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Renders a panel widget.
    pub fn render_panel(&self, panel: &Panel) -> Vec<SpriteInstance> {
        let _ = panel;
        Vec::new()
    }

    /// Renders a button widget.
    pub fn render_button(&self, button: &Button, hovered: bool, pressed: bool) -> Vec<SpriteInstance> {
        let _ = (button, hovered, pressed);
        Vec::new()
    }

    /// Renders a label widget.
    pub fn render_label(&self, label: &Label) -> Vec<SpriteInstance> {
        let _ = label;
        Vec::new()
    }

    /// Renders a list widget.
    pub fn render_list(&self, list: &List) -> Vec<SpriteInstance> {
        let _ = list;
        Vec::new()
    }

    /// Renders a 9-slice sprite to the given bounds.
    ///
    /// Generates 9 sprite instances (corners, edges, center) from a single texture.
    pub fn render_nine_slice(&self, nine_slice: &NineSlice, bounds: Rect) -> Vec<SpriteInstance> {
        let _ = (nine_slice, bounds);
        Vec::new()
    }
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_renderer_new() {
        let renderer = UiRenderer::new();
        let _ = renderer;
    }

    #[test]
    fn test_render_panel_empty() {
        let renderer = UiRenderer::new();
        let panel = Panel::new(crate::widget::WidgetId::new());
        let instances = renderer.render_panel(&panel);
        assert!(instances.is_empty());
    }

    #[test]
    fn test_render_button_empty() {
        let renderer = UiRenderer::new();
        let button = Button::new(crate::widget::WidgetId::new(), "test");
        let instances = renderer.render_button(&button, false, false);
        assert!(instances.is_empty());
    }

    #[test]
    fn test_render_label_empty() {
        let renderer = UiRenderer::new();
        let label = Label::new(crate::widget::WidgetId::new(), "test");
        let instances = renderer.render_label(&label);
        assert!(instances.is_empty());
    }

    #[test]
    fn test_render_list_empty() {
        let renderer = UiRenderer::new();
        let list = List::new(crate::widget::WidgetId::new());
        let instances = renderer.render_list(&list);
        assert!(instances.is_empty());
    }
}
