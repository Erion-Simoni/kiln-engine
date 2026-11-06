use crate::Rect;
use kiln_core::ui::{UiScreen, UiWidget};

/// The exact transform shared by authored UI rendering and hit testing.
#[derive(Debug, Clone, Copy)]
pub struct UiViewport {
    pub area: [f32; 4],
    pub scale: f32,
    pub logical_size: [f32; 2],
}
impl UiViewport {
    pub fn new(area: [f32; 4], reference: [f32; 2]) -> Self {
        let scale = (area[2] / reference[0].max(1.0))
            .min(area[3] / reference[1].max(1.0))
            .max(0.0001);
        Self {
            area,
            scale,
            logical_size: [area[2] / scale, area[3] / scale],
        }
    }
    /// Reject pointer coordinates outside the same viewport used for drawing.
    pub fn logical_point(&self, point: [f32; 2]) -> Option<[f32; 2]> {
        Rect::new(self.area[0], self.area[1], self.area[2], self.area[3])
            .contains_point(point[0], point[1])
            .then(|| {
                [
                    (point[0] - self.area[0]) / self.scale,
                    (point[1] - self.area[1]) / self.scale,
                ]
            })
    }
}

/// Resolved geometry in logical pixels, in deterministic parent-before-child paint order.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWidget {
    pub index: usize,
    pub bounds: Rect,
    pub clip: Rect,
    pub visible: bool,
    pub enabled: bool,
}

pub fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(
        x,
        y,
        (a.right().min(b.right()) - x).max(0.0),
        (a.bottom().min(b.bottom()) - y).max(0.0),
    )
}

pub fn widget_bounds(widget: &UiWidget, parent: Rect) -> Rect {
    let l = &widget.layout;
    let width = (l.size[0] + if l.stretch[0] { parent.width } else { 0.0 }).max(0.0);
    let height = (l.size[1] + if l.stretch[1] { parent.height } else { 0.0 }).max(0.0);
    Rect::new(
        parent.x + parent.width * l.anchor[0] + l.offset[0] - width * l.pivot[0],
        parent.y + parent.height * l.anchor[1] + l.offset[1] - height * l.pivot[1],
        width,
        height,
    )
}

/// The document is validated before resolving; the depth bound is retained defensively.
pub fn resolve(screen: &UiScreen, size: [f32; 2]) -> Vec<ResolvedWidget> {
    fn children(
        screen: &UiScreen,
        parent_id: &str,
        parent: Rect,
        clip: Rect,
        flags: [bool; 2],
        depth: usize,
        out: &mut Vec<ResolvedWidget>,
    ) {
        if depth > 32 {
            return;
        }
        for (index, w) in screen
            .widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| w.parent == parent_id)
        {
            let bounds = widget_bounds(w, parent);
            let clip = intersect(clip, bounds);
            let visible = flags[0] && w.visible && clip.width > 0.0 && clip.height > 0.0;
            let enabled = flags[1] && w.enabled;
            out.push(ResolvedWidget {
                index,
                bounds,
                clip,
                visible,
                enabled,
            });
            children(
                screen,
                &w.id,
                bounds,
                clip,
                [visible, enabled],
                depth + 1,
                out,
            );
        }
    }
    let bounds = Rect::new(0.0, 0.0, size[0], size[1]);
    let mut out = Vec::new();
    children(screen, "", bounds, bounds, [true, true], 0, &mut out);
    out
}
