//! Kiln UI crate — in-game UI system (separate from editor's egui).

pub mod font;
pub mod layout;
pub mod localization;
pub mod navigation;
pub mod render;
pub mod text;
pub mod ui_renderer;
pub mod widget;

pub use layout::{Anchor, Layout, LayoutConstraints, Rect, Size};
pub use localization::{Localization, LocalizationKey};
pub use navigation::{FocusDirection, FocusManager, Focusable};
pub use render::{NineSlice, NineSliceRenderer};
pub use text::TextRenderer;
pub use ui_renderer::{UiQuad, UiRenderer};
pub use widget::{Button, Label, List, Panel, Widget, WidgetId};
