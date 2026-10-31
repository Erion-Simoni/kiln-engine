//! UI widget types.

use crate::layout::{Layout, Rect};
use crate::localization::LocalizationKey;
use crate::navigation::Focusable;
use crate::render::NineSlice;
use serde::{Deserialize, Serialize};

/// Unique identifier for a widget instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn new() -> Self {
        Self(rand::random::<u64>())
    }

    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

/// Base widget trait.
pub trait Widget {
    /// Returns the widget's unique ID.
    fn id(&self) -> WidgetId;

    /// Returns the widget's layout.
    fn layout(&self) -> &Layout;

    /// Returns the widget's layout mutably.
    fn layout_mut(&mut self) -> &mut Layout;

    /// Returns the widget's computed bounds after layout.
    fn bounds(&self) -> Rect;

    /// Sets the widget's computed bounds.
    fn set_bounds(&mut self, bounds: Rect);

    /// Returns true if the widget is visible.
    fn visible(&self) -> bool;

    /// Sets the widget's visibility.
    fn set_visible(&mut self, visible: bool);

    /// Returns self as Any for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns self as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// A panel widget for grouping other widgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    id: WidgetId,
    layout: Layout,
    bounds: Rect,
    visible: bool,
    background: Option<NineSlice>,
}

impl Panel {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            layout: Layout::default(),
            bounds: Rect::default(),
            visible: true,
            background: None,
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_background(mut self, background: NineSlice) -> Self {
        self.background = Some(background);
        self
    }

    pub fn background(&self) -> Option<&NineSlice> {
        self.background.as_ref()
    }
}

impl Widget for Panel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A label widget for displaying text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    id: WidgetId,
    layout: Layout,
    bounds: Rect,
    visible: bool,
    text_key: LocalizationKey,
}

impl Label {
    pub fn new(id: WidgetId, text_key: impl Into<LocalizationKey>) -> Self {
        Self {
            id,
            layout: Layout::default(),
            bounds: Rect::default(),
            visible: true,
            text_key: text_key.into(),
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn text_key(&self) -> &LocalizationKey {
        &self.text_key
    }

    pub fn set_text_key(&mut self, text_key: impl Into<LocalizationKey>) {
        self.text_key = text_key.into();
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A button widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    id: WidgetId,
    layout: Layout,
    bounds: Rect,
    visible: bool,
    text_key: LocalizationKey,
    normal: Option<NineSlice>,
    hovered: Option<NineSlice>,
    pressed: Option<NineSlice>,
    /// Optional action name to trigger when clicked (e.g., "pause", "restart", "menu").
    pub on_click: Option<String>,
}

impl Button {
    pub fn new(id: WidgetId, text_key: impl Into<LocalizationKey>) -> Self {
        Self {
            id,
            layout: Layout::default(),
            bounds: Rect::default(),
            visible: true,
            text_key: text_key.into(),
            normal: None,
            hovered: None,
            pressed: None,
            on_click: None,
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_normal(mut self, normal: NineSlice) -> Self {
        self.normal = Some(normal);
        self
    }

    pub fn with_hovered(mut self, hovered: NineSlice) -> Self {
        self.hovered = Some(hovered);
        self
    }

    pub fn with_pressed(mut self, pressed: NineSlice) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn with_on_click(mut self, action: impl Into<String>) -> Self {
        self.on_click = Some(action.into());
        self
    }

    pub fn on_click(&self) -> Option<&str> {
        self.on_click.as_deref()
    }

    pub fn set_on_click(&mut self, action: impl Into<String>) {
        self.on_click = Some(action.into());
    }

    /// Check if the given point is inside the button bounds.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        self.visible() && self.bounds().contains_point(x, y)
    }

    pub fn text_key(&self) -> &LocalizationKey {
        &self.text_key
    }

    pub fn set_text_key(&mut self, text_key: impl Into<LocalizationKey>) {
        self.text_key = text_key.into();
    }

    pub fn normal(&self) -> Option<&NineSlice> {
        self.normal.as_ref()
    }

    pub fn hovered(&self) -> Option<&NineSlice> {
        self.hovered.as_ref()
    }

    pub fn pressed(&self) -> Option<&NineSlice> {
        self.pressed.as_ref()
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Focusable for Button {
    fn can_focus(&self) -> bool {
        self.visible
    }
}

/// A list widget for scrollable item lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    id: WidgetId,
    layout: Layout,
    bounds: Rect,
    visible: bool,
    items: Vec<LocalizationKey>,
    selected_index: Option<usize>,
    scroll_offset: f32,
}

impl List {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            layout: Layout::default(),
            bounds: Rect::default(),
            visible: true,
            items: Vec::new(),
            selected_index: None,
            scroll_offset: 0.0,
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_items(mut self, items: Vec<LocalizationKey>) -> Self {
        self.items = items;
        self
    }

    pub fn items(&self) -> &[LocalizationKey] {
        &self.items
    }

    pub fn add_item(&mut self, item: impl Into<LocalizationKey>) {
        self.items.push(item.into());
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }
}

impl Widget for List {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Focusable for List {
    fn can_focus(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::NineSlice;

    #[test]
    fn test_widget_id_unique() {
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_widget_id_from_u64() {
        let id = WidgetId::from_u64(123);
        assert_eq!(id.as_u64(), 123);
    }

    #[test]
    fn test_panel_new() {
        let panel = Panel::new(WidgetId::new());
        assert!(panel.visible());
        assert_eq!(panel.background(), None);
    }

    #[test]
    fn test_panel_with_background() {
        let nine_slice = NineSlice::new(1, 8.0);
        let panel = Panel::new(WidgetId::new()).with_background(nine_slice);
        assert!(panel.background().is_some());
    }

    #[test]
    fn test_label_new() {
        let label = Label::new(WidgetId::new(), "test.key");
        assert!(label.visible());
        assert_eq!(label.text_key().as_str(), "test.key");
    }

    #[test]
    fn test_label_set_text_key() {
        let mut label = Label::new(WidgetId::new(), "test.key");
        label.set_text_key("new.key");
        assert_eq!(label.text_key().as_str(), "new.key");
    }

    #[test]
    fn test_button_new() {
        let button = Button::new(WidgetId::new(), "button.start");
        assert!(button.visible());
        assert_eq!(button.text_key().as_str(), "button.start");
        assert!(button.normal().is_none());
    }

    #[test]
    fn test_button_with_states() {
        let normal = NineSlice::new(1, 8.0);
        let hovered = NineSlice::new(2, 8.0);
        let pressed = NineSlice::new(3, 8.0);
        let button = Button::new(WidgetId::new(), "button.start")
            .with_normal(normal)
            .with_hovered(hovered)
            .with_pressed(pressed);
        assert!(button.normal().is_some());
        assert!(button.hovered().is_some());
        assert!(button.pressed().is_some());
    }

    #[test]
    fn test_list_new() {
        let list = List::new(WidgetId::new());
        assert!(list.visible());
        assert!(list.items().is_empty());
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn test_list_with_items() {
        let items = vec![
            LocalizationKey::new("item1"),
            LocalizationKey::new("item2"),
            LocalizationKey::new("item3"),
        ];
        let list = List::new(WidgetId::new()).with_items(items);
        assert_eq!(list.items().len(), 3);
    }

    #[test]
    fn test_list_add_item() {
        let mut list = List::new(WidgetId::new());
        list.add_item("item1");
        list.add_item("item2");
        assert_eq!(list.items().len(), 2);
    }

    #[test]
    fn test_list_selected_index() {
        let mut list = List::new(WidgetId::new());
        list.add_item("item1");
        list.add_item("item2");
        list.set_selected_index(Some(1));
        assert_eq!(list.selected_index(), Some(1));
    }
}

/// A Canvas widget for screen-space UI rendering.
///
/// A Canvas is a container widget that positions its children in screen space,
/// making it ideal for HUDs, menus, and other screen-space UI elements.
/// Unlike regular widgets which are positioned relative to their parent,
/// canvas children are positioned in absolute screen coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    id: WidgetId,
    layout: Layout,
    bounds: Rect,
    visible: bool,
    /// Background color for the canvas.
    pub background_color: [f32; 4],
    /// Whether the canvas captures input events.
    pub captures_input: bool,
    /// Child widgets contained in this canvas.
    #[serde(skip)]
    pub children: Vec<Box<dyn Widget>>,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            layout: Layout::default(),
            bounds: Rect::default(),
            visible: true,
            background_color: [0.0, 0.0, 0.0, 0.0],
            captures_input: false,
            children: Vec::new(),
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn with_background_color(mut self, color: [f32; 4]) -> Self {
        self.background_color = color;
        self
    }

    pub fn with_captures_input(mut self, captures: bool) -> Self {
        self.captures_input = captures;
        self
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    pub fn add_button(&mut self, button: Button) {
        self.children.push(Box::new(button));
    }

    pub fn add_label(&mut self, label: Label) {
        self.children.push(Box::new(label));
    }

    pub fn add_panel(&mut self, panel: Panel) {
        self.children.push(Box::new(panel));
    }

    pub fn add_list(&mut self, list: List) {
        self.children.push(Box::new(list));
    }
}

impl Widget for Canvas {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
