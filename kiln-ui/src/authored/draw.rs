use super::*;
use crate::{Rect, TextRenderer, UiQuad};
use kiln_render::TextureId;

/// Stay within the existing sprite renderer's 16384-quad budget.
pub use kiln_core::ui::MAX_UI_QUADS;

/// Clip geometry and UVs together; clipping a textured glyph must not stretch it.
pub fn clip_quad(mut q: UiQuad, clip: Rect) -> Option<UiQuad> {
    let i = &mut q.instance;
    let rect = Rect::new(
        i.position[0] - i.size[0] * 0.5,
        i.position[1] - i.size[1] * 0.5,
        i.size[0],
        i.size[1],
    );
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let cut = layout::intersect(rect, clip);
    if cut.width <= 0.0 || cut.height <= 0.0 {
        return None;
    }
    let uv = i.uv_rect;
    i.uv_rect = [
        uv[0] + (uv[2] - uv[0]) * (cut.x - rect.x) / rect.width,
        uv[1] + (uv[3] - uv[1]) * (cut.y - rect.y) / rect.height,
        uv[0] + (uv[2] - uv[0]) * (cut.right() - rect.x) / rect.width,
        uv[1] + (uv[3] - uv[1]) * (cut.bottom() - rect.y) / rect.height,
    ];
    i.position = [cut.center_x(), cut.center_y()];
    i.size = [cut.width, cut.height];
    Some(q)
}

impl UiRuntime {
    /// Draw into physical viewport coordinates, using the same layout as interaction.
    pub fn draw(&self, fonts: &UiFonts, viewport: UiViewport) -> Vec<UiQuad> {
        let Some(screen) = self.document.screens.get(&self.active) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in layout::resolve(screen, viewport.logical_size)
            .iter()
            .filter(|r| r.visible)
        {
            let w = &screen.widgets[row.index];
            let b = row.bounds;
            let mut quads = Vec::new();
            if w.kind != UiKind::Label {
                let active =
                    self.focus.as_deref() == Some(&w.id) || self.hovered.as_deref() == Some(&w.id);
                let mut color = if active && w.kind == UiKind::Button {
                    w.focus_color
                } else {
                    w.color
                };
                if !row.enabled {
                    color[3] *= 0.4;
                }
                quads.push(UiQuad::rect(b.x, b.y, b.width, b.height, color));
            }
            if w.kind != UiKind::Panel && !w.text.is_empty() {
                let font = fonts.fonts.get(&w.font);
                let measure = font
                    .map(|f| f.measure(&w.text, w.font_size))
                    .unwrap_or_else(|| {
                        let (x, y) = TextRenderer::new().measure(&w.text, w.font_size);
                        [x, y]
                    });
                let origin = [
                    b.x + (b.width - measure[0]) * w.text_align[0],
                    b.y + (b.height - measure[1]) * w.text_align[1],
                ];
                let text = if let Some(font) = font {
                    font.render(&w.text, origin, w.font_size, w.text_color)
                } else {
                    TextRenderer::new()
                        .render(&w.text, origin[0], origin[1], w.font_size, w.text_color)
                        .into_iter()
                        .map(|instance| UiQuad {
                            texture: TextureId::WHITE,
                            instance,
                        })
                        .collect()
                };
                quads.extend(text);
            }
            for mut q in quads.into_iter().filter_map(|q| clip_quad(q, row.clip)) {
                if out.len() >= MAX_UI_QUADS {
                    return out;
                }
                q.instance.position = [
                    viewport.area[0] + q.instance.position[0] * viewport.scale,
                    viewport.area[1] + q.instance.position[1] * viewport.scale,
                ];
                q.instance.size = q.instance.size.map(|v| v * viewport.scale);
                out.push(q);
            }
        }
        out
    }
}
