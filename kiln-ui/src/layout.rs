//! Layout system with flex-style layout, anchors, and constraints.

use serde::{Deserialize, Serialize};

/// A 2D rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }

    /// Check if a point is inside this rectangle.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// A 2D size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Anchor points for positioning widgets relative to their parent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    /// Returns the relative position (0.0 to 1.0) for horizontal alignment.
    pub fn horizontal(&self) -> f32 {
        match self {
            Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => 0.0,
            Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => 0.5,
            Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => 1.0,
        }
    }

    /// Returns the relative position (0.0 to 1.0) for vertical alignment.
    pub fn vertical(&self) -> f32 {
        match self {
            Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => 0.0,
            Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => 0.5,
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => 1.0,
        }
    }
}

/// Layout constraints for sizing and positioning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutConstraints {
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub preferred_width: Option<f32>,
    pub preferred_height: Option<f32>,
}

impl LayoutConstraints {
    pub fn new() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            preferred_width: None,
            preferred_height: None,
        }
    }

    pub fn with_min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn with_min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn with_preferred_width(mut self, width: f32) -> Self {
        self.preferred_width = Some(width);
        self
    }

    pub fn with_preferred_height(mut self, height: f32) -> Self {
        self.preferred_height = Some(height);
        self
    }

    /// Clamps a size to fit within the constraints.
    pub fn clamp_size(&self, mut size: Size) -> Size {
        if let Some(min_w) = self.min_width {
            size.width = size.width.max(min_w);
        }
        if let Some(max_w) = self.max_width {
            size.width = size.width.min(max_w);
        }
        if let Some(min_h) = self.min_height {
            size.height = size.height.max(min_h);
        }
        if let Some(max_h) = self.max_height {
            size.height = size.height.min(max_h);
        }
        size
    }
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self::new()
    }
}

/// Flex direction for layout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FlexDirection {
    Row,
    Column,
}

/// Main axis alignment for flex layout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MainAxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

/// Cross axis alignment for flex layout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CrossAxisAlignment {
    Start,
    Center,
    End,
    Stretch,
}

/// Layout configuration for a widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// Anchor point for positioning relative to parent.
    pub anchor: Anchor,
    /// Offset from anchor point.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Size constraints.
    pub constraints: LayoutConstraints,
    /// Flex layout (if this widget is a container).
    pub flex_direction: Option<FlexDirection>,
    pub main_axis_alignment: Option<MainAxisAlignment>,
    pub cross_axis_alignment: Option<CrossAxisAlignment>,
    /// Flex grow factor (0.0 = don't grow).
    pub flex_grow: f32,
    /// Flex shrink factor (1.0 = default shrink behavior).
    pub flex_shrink: f32,
    /// Margin (space outside the widget).
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            anchor: Anchor::TopLeft,
            offset_x: 0.0,
            offset_y: 0.0,
            constraints: LayoutConstraints::default(),
            flex_direction: None,
            main_axis_alignment: None,
            cross_axis_alignment: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
        }
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    pub fn with_constraints(mut self, constraints: LayoutConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_flex(
        mut self,
        direction: FlexDirection,
        main: MainAxisAlignment,
        cross: CrossAxisAlignment,
    ) -> Self {
        self.flex_direction = Some(direction);
        self.main_axis_alignment = Some(main);
        self.cross_axis_alignment = Some(cross);
        self
    }

    pub fn with_flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    pub fn with_flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }

    pub fn with_margin(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.margin_top = top;
        self.margin_right = right;
        self.margin_bottom = bottom;
        self.margin_left = left;
        self
    }

    /// Calculates the position based on anchor and parent bounds.
    pub fn calculate_position(&self, parent_bounds: Rect, child_size: Size) -> (f32, f32) {
        let parent_x = parent_bounds.x + parent_bounds.width * self.anchor.horizontal();
        let parent_y = parent_bounds.y + parent_bounds.height * self.anchor.vertical();

        let x = parent_x - child_size.width * self.anchor.horizontal() + self.offset_x;
        let y = parent_y - child_size.height * self.anchor.vertical() + self.offset_y;

        (x, y)
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_positions() {
        assert_eq!(Anchor::TopLeft.horizontal(), 0.0);
        assert_eq!(Anchor::TopLeft.vertical(), 0.0);
        assert_eq!(Anchor::Center.horizontal(), 0.5);
        assert_eq!(Anchor::Center.vertical(), 0.5);
        assert_eq!(Anchor::BottomRight.horizontal(), 1.0);
        assert_eq!(Anchor::BottomRight.vertical(), 1.0);
    }

    #[test]
    fn test_layout_constraints_clamp() {
        let constraints = LayoutConstraints::new()
            .with_min_width(10.0)
            .with_max_width(100.0)
            .with_min_height(5.0)
            .with_max_height(50.0);

        let size = constraints.clamp_size(Size::new(50.0, 25.0));
        assert_eq!(size.width, 50.0);
        assert_eq!(size.height, 25.0);

        let size = constraints.clamp_size(Size::new(5.0, 3.0));
        assert_eq!(size.width, 10.0);
        assert_eq!(size.height, 5.0);

        let size = constraints.clamp_size(Size::new(150.0, 75.0));
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 50.0);
    }

    #[test]
    fn test_layout_calculate_position() {
        let parent = Rect::new(0.0, 0.0, 800.0, 600.0);
        let child_size = Size::new(100.0, 50.0);

        let layout = Layout::new().with_anchor(Anchor::TopLeft);
        let (x, y) = layout.calculate_position(parent, child_size);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);

        let layout = Layout::new().with_anchor(Anchor::Center);
        let (x, y) = layout.calculate_position(parent, child_size);
        assert_eq!(x, 350.0); // 400 - 50
        assert_eq!(y, 275.0); // 300 - 25

        let layout = Layout::new().with_anchor(Anchor::BottomRight);
        let (x, y) = layout.calculate_position(parent, child_size);
        assert_eq!(x, 700.0); // 800 - 100
        assert_eq!(y, 550.0); // 600 - 50
    }

    #[test]
    fn test_rect_properties() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.top(), 20.0);
        assert_eq!(rect.bottom(), 70.0);
        assert_eq!(rect.center_x(), 60.0);
        assert_eq!(rect.center_y(), 45.0);
    }
}
