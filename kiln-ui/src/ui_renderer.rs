//! UI → sprite-instance conversion for the kiln-render sprite batch.

use crate::layout::Rect;
use crate::localization::Localization;
use crate::render::{NineSlice, NineSliceRenderer};
use crate::text::TextRenderer;
use crate::widget::{Button, Canvas, Label, List, Panel, Widget};
use kiln_render::{SpriteInstance, TextureId};

/// Default UI text height in pixels.
pub const FONT_SIZE: f32 = 14.0;
/// Default line height for list rows in pixels.
pub const ROW_HEIGHT: f32 = 24.0;

/// A quad to draw: which texture it samples plus its instance data.
///
/// White-texture quads (full UVs) are solid-coloured rects or glyph pixels;
/// nine-slice quads keep their own UV rect.
#[derive(Debug, Clone, Copy)]
pub struct UiQuad {
    /// Texture to sample.
    pub texture: TextureId,
    /// Sprite instance data (screen space, y-down, centre position).
    pub instance: SpriteInstance,
}

impl UiQuad {
    /// A solid rectangle using the built-in white texture.
    pub fn rect(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Self {
        Self {
            texture: TextureId::WHITE,
            instance: SpriteInstance::colored([x + w / 2.0, y + h / 2.0], [w, h], color),
        }
    }
}

/// Converts widgets into sprite instances for the sprite batch.
///
/// Widgets must already have bounds (from layout or `set_bounds`); invisible
/// widgets emit nothing. Pair with an ortho camera with a top-left origin so
/// the y-down screen coordinates line up.
pub struct UiRenderer {
    nine: NineSliceRenderer,
    text: TextRenderer,
    /// Optional localization for resolving widget text keys.
    pub localization: Option<Localization>,
    /// Text colour.
    pub text_color: [f32; 4],
    /// Colour behind list rows.
    pub row_color: [f32; 4],
    /// Colour behind the selected list row.
    pub selected_row_color: [f32; 4],
    /// Colour behind a panel with no nine-slice background.
    pub panel_color: [f32; 4],
    /// Font size in pixels.
    pub font_size: f32,
}

impl UiRenderer {
    /// Creates a renderer with default theme colours.
    pub fn new() -> Self {
        Self {
            nine: NineSliceRenderer::new(),
            text: TextRenderer::new(),
            localization: None,
            text_color: [1.0, 1.0, 1.0, 1.0],
            row_color: [0.0, 0.0, 0.0, 0.35],
            selected_row_color: [0.2, 0.45, 0.9, 0.7],
            panel_color: [0.1, 0.1, 0.12, 0.6],
            font_size: FONT_SIZE,
        }
    }

    /// Check if a point is inside a widget's bounds and return the widget's on_click action if it's a button.
    pub fn hit_test_button(&self, widget: &dyn Widget, x: f32, y: f32) -> Option<String> {
        if !widget.visible() {
            return None;
        }
        let bounds = widget.bounds();
        if !bounds.contains_point(x, y) {
            return None;
        }
        
        // Check if it's a button with an on_click action
        if let Some(button) = widget.as_any().downcast_ref::<Button>() {
            if let Some(action) = button.on_click() {
                return Some(action.to_string());
            }
        }
        
        // Check children if it's a Canvas
        if let Some(canvas) = widget.as_any().downcast_ref::<Canvas>() {
            for child in &canvas.children {
                if let Some(action) = self.hit_test_button(child.as_ref(), x, y) {
                    return Some(action);
                }
            }
        }
        
        None
    }

    /// Check if a point is inside any widget and return the topmost widget's on_click action.
    /// Widgets are checked in reverse order (last added = topmost).
    pub fn hit_test_canvas(&self, canvas: &Canvas, x: f32, y: f32) -> Option<String> {
        if !canvas.visible() {
            return None;
        }
        let bounds = canvas.bounds();
        if !bounds.contains_point(x, y) {
            return None;
        }
        
        // Check children in reverse order (topmost first)
        for child in canvas.children.iter().rev() {
            if let Some(action) = self.hit_test_button(child.as_ref(), x, y) {
                return Some(action);
            }
        }
        
        None
    }

    /// Resolves a localization key through `localization`, falling back to the key text.
    pub fn resolve(&self, key: &crate::localization::LocalizationKey) -> String {
        match &self.localization {
            Some(loc) => loc.get_or_key(key),
            None => key.as_str().to_string(),
        }
    }

    /// Panel background: nine-slice if set, otherwise a solid fallback rect.
    pub fn render_panel(&self, panel: &Panel) -> Vec<UiQuad> {
        if !panel.visible() {
            return Vec::new();
        }
        let b = panel.bounds();
        if b.width <= 0.0 || b.height <= 0.0 {
            return Vec::new();
        }
        match panel.background() {
            Some(ns) => nine_quads(&self.nine, ns, b, [1.0; 4]),
            None => vec![UiQuad::rect(b.x, b.y, b.width, b.height, self.panel_color)],
        }
    }

    /// Button background (state nine-slice, else a solid fallback) plus a
    /// vertically centred label.
    pub fn render_button(&self, button: &Button, hovered: bool, pressed: bool) -> Vec<UiQuad> {
        if !button.visible() {
            return Vec::new();
        }
        let b = button.bounds();
        if b.width <= 0.0 || b.height <= 0.0 {
            return Vec::new();
        }
        let state = if pressed {
            button.pressed().or(button.normal())
        } else if hovered {
            button.hovered().or(button.normal())
        } else {
            button.normal()
        };
        let mut out = match state {
            Some(ns) => nine_quads(&self.nine, ns, b, [1.0; 4]),
            None => vec![UiQuad::rect(
                b.x,
                b.y,
                b.width,
                b.height,
                button_tint(hovered, pressed),
            )],
        };
        out.extend(self.centered_text(&self.resolve(button.text_key()), b));
        out
    }

    /// Label: one quad per glyph run, left-aligned at the widget's top-left.
    pub fn render_label(&self, label: &Label) -> Vec<UiQuad> {
        if !label.visible() {
            return Vec::new();
        }
        let b = label.bounds();
        if b.width <= 0.0 && b.height <= 0.0 {
            return Vec::new();
        }
        text_quads(
            &self.text,
            &self.resolve(label.text_key()),
            b.x,
            b.y,
            self.font_size,
            self.text_color,
        )
    }

    /// List: a row background per item (selected row highlighted) with the
    /// item's text, offset by scroll. Rows outside bounds are skipped.
    pub fn render_list(&self, list: &List) -> Vec<UiQuad> {
        if !list.visible() {
            return Vec::new();
        }
        let b = list.bounds();
        if b.width <= 0.0 || b.height <= 0.0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let scroll = list.scroll_offset();
        for (i, key) in list.items().iter().enumerate() {
            let y = b.y + i as f32 * ROW_HEIGHT - scroll;
            if y + ROW_HEIGHT <= b.y || y >= b.bottom() {
                continue;
            }
            let selected = list.selected_index() == Some(i);
            out.push(UiQuad::rect(
                b.x,
                y,
                b.width,
                ROW_HEIGHT,
                if selected {
                    self.selected_row_color
                } else {
                    self.row_color
                },
            ));
            text_quads(
                &self.text,
                &self.resolve(key),
                b.x + 6.0,
                y + (ROW_HEIGHT - self.font_size) / 2.0,
                self.font_size,
                self.text_color,
            );
        }
        out
    }

    /// Nine-slice sprite stretched over `bounds`.
    pub fn render_nine_slice(&self, nine_slice: &NineSlice, bounds: Rect) -> Vec<UiQuad> {
        nine_quads(&self.nine, nine_slice, bounds, [1.0; 4])
    }

    /// Canvas: renders background and all child widgets.
    /// Canvas children are positioned in absolute screen coordinates.
    pub fn render_canvas(&self, canvas: &Canvas) -> Vec<UiQuad> {
        if !canvas.visible() {
            return Vec::new();
        }
        let b = canvas.bounds();
        if b.width <= 0.0 || b.height <= 0.0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        
        // Render background if not transparent
        if canvas.background_color[3] > 0.0 {
            out.push(UiQuad::rect(b.x, b.y, b.width, b.height, canvas.background_color));
        }
        
        // Render children
        for child in &canvas.children {
            // Downcast to specific widget types and render
            // We use a trait object approach with type_id checks
            if let Some(panel) = child.as_any().downcast_ref::<Panel>() {
                out.extend(self.render_panel(panel));
            } else if let Some(button) = child.as_any().downcast_ref::<Button>() {
                // For buttons, we need to know hover/pressed state
                // For now, render as not hovered/pressed
                out.extend(self.render_button(button, false, false));
            } else if let Some(label) = child.as_any().downcast_ref::<Label>() {
                out.extend(self.render_label(label));
            } else if let Some(list) = child.as_any().downcast_ref::<List>() {
                out.extend(self.render_list(list));
            } else if let Some(canvas) = child.as_any().downcast_ref::<Canvas>() {
                out.extend(self.render_canvas(canvas));
            }
        }
        
        out
    }

    /// Bitmap text laid out with its top-left at `(x, y)`.
    pub fn render_text(&self, text: &str, x: f32, y: f32, color: [f32; 4]) -> Vec<UiQuad> {
        text_quads(&self.text, text, x, y, self.font_size, color)
    }

    /// Pushes quads into a [`kiln_render::SpriteRenderer`].
    ///
    /// Each quad calls `push_textured`; the renderer batches consecutive
    /// same-texture quads, so group them by texture first if you need fewer
    /// draw calls.
    pub fn submit(&self, quads: &[UiQuad], renderer: &mut kiln_render::SpriteRenderer) {
        for q in quads {
            renderer.push_textured(q.texture, q.instance);
        }
    }

    /// Text sized to fit `b` horizontally and centred vertically in it.
    fn centered_text(&self, s: &str, b: Rect) -> Vec<UiQuad> {
        let (w, h) = self.text.measure(s, self.font_size);
        let x = b.x + ((b.width - w) / 2.0).max(0.0);
        let y = b.y + ((b.height - h) / 2.0).max(0.0);
        text_quads(&self.text, s, x, y, self.font_size, self.text_color)
    }
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn button_tint(hovered: bool, pressed: bool) -> [f32; 4] {
    if pressed {
        [0.35, 0.35, 0.4, 1.0]
    } else if hovered {
        [0.3, 0.3, 0.35, 1.0]
    } else {
        [0.2, 0.2, 0.24, 1.0]
    }
}

fn nine_quads(
    nine: &NineSliceRenderer,
    ns: &NineSlice,
    bounds: Rect,
    color: [f32; 4],
) -> Vec<UiQuad> {
    nine.render(ns, bounds, color)
        .into_iter()
        .map(|instance| UiQuad {
            texture: TextureId(ns.texture_id as u32),
            instance,
        })
        .collect()
}

fn text_quads(
    text: &TextRenderer,
    s: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: [f32; 4],
) -> Vec<UiQuad> {
    text.render(s, x, y, font_size, color)
        .into_iter()
        .map(|instance| UiQuad {
            texture: TextureId::WHITE,
            instance,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Button, Label, List, Panel, Widget, WidgetId};

    fn set_bounds(w: &mut dyn Widget, x: f32, y: f32, width: f32, height: f32) {
        w.set_bounds(Rect::new(x, y, width, height));
    }

    #[test]
    fn invisible_and_unbounded_widgets_emit_nothing() {
        let r = UiRenderer::new();
        let mut panel = Panel::new(WidgetId::new());
        assert!(r.render_panel(&panel).is_empty()); // zero bounds
        panel.set_visible(false);
        set_bounds(&mut panel, 0.0, 0.0, 10.0, 10.0);
        assert!(r.render_panel(&panel).is_empty()); // invisible

        let mut label = Label::new(WidgetId::new(), "hi");
        assert!(r.render_label(&label).is_empty());
        set_bounds(&mut label, 0.0, 0.0, 10.0, 10.0);
        assert!(!r.render_label(&label).is_empty());
    }

    #[test]
    fn panel_without_background_is_one_solid_rect() {
        let r = UiRenderer::new();
        let mut panel = Panel::new(WidgetId::new());
        set_bounds(&mut panel, 5.0, 5.0, 20.0, 10.0);
        let quads = r.render_panel(&panel);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].texture, TextureId::WHITE);
        assert_eq!(quads[0].instance.size, [20.0, 10.0]);
    }

    #[test]
    fn button_draws_background_then_label_and_responds_to_state() {
        let r = UiRenderer::new();
        let mut button = Button::new(WidgetId::new(), "menu.start");
        set_bounds(&mut button, 0.0, 0.0, 120.0, 32.0);
        let quads = r.render_button(&button, false, false);
        assert_eq!(quads[0].instance.size, [120.0, 32.0]); // background first
        assert!(quads.len() > 1, "label glyphs should be emitted");
        let normal = quads[0].instance.color;
        assert_ne!(
            r.render_button(&button, true, false)[0].instance.color,
            normal,
            "hover tint"
        );
        assert_ne!(
            r.render_button(&button, false, true)[0].instance.color,
            normal,
            "press tint"
        );
    }

    #[test]
    fn nine_slice_button_keeps_texture_and_uv() {
        let r = UiRenderer::new();
        let ns = NineSlice::new(7, 4.0).with_texture_size(16.0, 16.0);
        let mut button = Button::new(WidgetId::new(), "ok").with_normal(ns);
        set_bounds(&mut button, 0.0, 0.0, 64.0, 32.0);
        let quads = r.render_button(&button, false, false);
        assert_eq!(quads[0].texture, TextureId(7));
        assert_eq!(quads[0].instance.uv_rect, [0.0, 0.0, 0.25, 0.25]);
        let slices = quads.iter().filter(|q| q.texture == TextureId(7)).count();
        assert_eq!(slices, 9, "all nine slices");
        assert!(quads.len() > 9, "label glyphs follow the slices");
    }

    #[test]
    fn list_renders_rows_and_skips_scrolled_off() {
        let r = UiRenderer::new();
        let mut list =
            List::new(WidgetId::new()).with_items(vec!["a".into(), "b".into(), "c".into()]);
        set_bounds(&mut list, 0.0, 0.0, 100.0, 48.0);
        list.set_selected_index(Some(1));
        let count_rows = |qs: &[UiQuad]| {
            qs.iter()
                .filter(|q| q.instance.size == [100.0, ROW_HEIGHT])
                .count()
        };

        // bounds hold 48/24 = 2 full rows; the third sits below the box and is clipped
        let quads = r.render_list(&list);
        assert_eq!(count_rows(&quads), 2);
        assert!(
            quads.iter().any(|q| q.instance.size == [100.0, ROW_HEIGHT]
                && q.instance.color == r.selected_row_color),
            "selected row should be highlighted"
        );

        // widen the bounds to fit all three rows
        set_bounds(&mut list, 0.0, 0.0, 100.0, 72.0);
        assert_eq!(count_rows(&r.render_list(&list)), 3);

        // scrolling by one row drops the first row out of the box
        list.set_scroll_offset(ROW_HEIGHT);
        assert_eq!(count_rows(&r.render_list(&list)), 2);
    }

    #[test]
    fn localization_resolves_label_text() {
        let mut r = UiRenderer::new();
        let mut loc = crate::localization::Localization::new("en");
        loc.add("menu.start", "START");
        r.localization = Some(loc);
        assert_eq!(r.resolve(&"menu.start".into()), "START");
        assert_eq!(r.resolve(&"missing".into()), "missing");
    }
}
