//! Saved in-game screens. GPU handles and transient interaction stay outside Project.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Shared with the existing sprite renderer's per-batch capacity.
pub const MAX_UI_QUADS: usize = 16_384;

/// Responsive authoring canvas and reusable named screens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiDocument {
    pub reference_size: [f32; 2],
    pub initial_screen: String,
    pub pause_screen: String,
    pub scene_screens: BTreeMap<String, String>,
    pub screens: BTreeMap<String, UiScreen>,
    pub fonts: BTreeMap<String, UiFont>,
}
impl Default for UiDocument {
    fn default() -> Self {
        Self {
            reference_size: [640.0, 360.0],
            initial_screen: String::new(),
            pause_screen: String::new(),
            scene_screens: BTreeMap::new(),
            screens: BTreeMap::new(),
            fonts: BTreeMap::new(),
        }
    }
}

/// Font resource plus extra dynamic-text characters to bake alongside ASCII and authored text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiFont {
    pub path: String,
    #[serde(default)]
    pub extra_chars: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiScreen {
    pub name: String,
    pub pause_game: bool,
    pub modal: bool,
    pub allow_cancel: bool,
    pub default_focus: String,
    pub widgets: Vec<UiWidget>,
}
impl Default for UiScreen {
    fn default() -> Self {
        Self {
            name: "Screen".into(),
            pause_game: true,
            modal: true,
            allow_cancel: true,
            default_focus: String::new(),
            widgets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiKind {
    Panel,
    Label,
    #[default]
    Button,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAction {
    #[default]
    Event,
    ShowScreen(String),
    Back,
    Close,
}

/// Anchor and pivot are normalized; stretched size is an addition to parent size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiLayout {
    pub anchor: [f32; 2],
    pub pivot: [f32; 2],
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub stretch: [bool; 2],
}
impl Default for UiLayout {
    fn default() -> Self {
        Self {
            anchor: [0.0; 2],
            pivot: [0.0; 2],
            offset: [20.0; 2],
            size: [160.0, 36.0],
            stretch: [false; 2],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiWidget {
    pub id: String,
    /// Empty means screen root. Children are clipped by all ancestors.
    pub parent: String,
    pub kind: UiKind,
    pub layout: UiLayout,
    pub visible: bool,
    pub enabled: bool,
    /// Panels may block buttons behind them. Decorative labels do not block.
    pub blocks_pointer: bool,
    pub text: String,
    /// Empty selects the built-in 5x7 bitmap font.
    pub font: String,
    pub font_size: f32,
    /// Normalized alignment within the widget bounds.
    pub text_align: [f32; 2],
    pub color: [f32; 4],
    pub text_color: [f32; 4],
    pub focus_color: [f32; 4],
    pub event: String,
    pub action: UiAction,
    /// Optional stable target IDs, keyed by up/down/left/right/next/previous.
    pub navigation: BTreeMap<String, String>,
}
impl Default for UiWidget {
    fn default() -> Self {
        Self {
            id: String::new(),
            parent: String::new(),
            kind: UiKind::Button,
            layout: UiLayout::default(),
            visible: true,
            enabled: true,
            blocks_pointer: true,
            text: "Button".into(),
            font: String::new(),
            font_size: 18.0,
            text_align: [0.5; 2],
            color: [0.12, 0.18, 0.28, 1.0],
            text_color: [1.0; 4],
            focus_color: [0.25, 0.5, 0.85, 1.0],
            event: String::new(),
            action: UiAction::Event,
            navigation: BTreeMap::new(),
        }
    }
}

impl UiDocument {
    /// Reject broken references and unbounded/invalid layout before gameplay starts.
    pub fn validate(&self) -> Result<(), String> {
        if self
            .reference_size
            .iter()
            .any(|v| !v.is_finite() || *v < 1.0 || *v > 16384.0)
        {
            return Err("UI reference size must be within 1..16384".into());
        }
        if self.screens.len() > 128 || self.fonts.len() > 16 {
            return Err("UI screen/font limit exceeded".into());
        }
        for name in [&self.initial_screen, &self.pause_screen]
            .into_iter()
            .chain(self.scene_screens.values())
        {
            if !name.is_empty() && !self.screens.contains_key(name) {
                return Err(format!("Missing UI screen '{name}'"));
            }
        }
        for (id, screen) in &self.screens {
            if id.is_empty() || screen.widgets.len() > 1024 {
                return Err("UI screen ID or widget limit is invalid".into());
            }
            let ids: HashSet<_> = screen.widgets.iter().map(|w| w.id.as_str()).collect();
            if ids.len() != screen.widgets.len() || ids.contains("") {
                return Err(format!("Duplicate/empty widget ID in '{id}'"));
            }
            if !screen.default_focus.is_empty()
                && !screen
                    .widgets
                    .iter()
                    .any(|w| w.id == screen.default_focus && w.kind == UiKind::Button)
            {
                return Err(format!("Invalid default focus in '{id}'"));
            }
            // Bitmap glyphs use at most 3 runs in each of 7 rows; atlas fonts
            // use one quad per glyph. Include hidden widgets because scripts can show them.
            let cost: usize = screen
                .widgets
                .iter()
                .map(|w| {
                    usize::from(w.kind != UiKind::Label)
                        + if w.kind == UiKind::Panel {
                            0
                        } else {
                            w.text.chars().count() * if w.font.is_empty() { 21 } else { 1 }
                        }
                })
                .sum();
            if cost > MAX_UI_QUADS {
                return Err(format!(
                    "UI screen '{id}' exceeds the quad budget; use an atlas font or fewer widgets"
                ));
            }
            for widget in &screen.widgets {
                self.validate_widget(id, screen, widget)?;
            }
        }
        Ok(())
    }

    fn validate_widget(
        &self,
        screen_id: &str,
        screen: &UiScreen,
        w: &UiWidget,
    ) -> Result<(), String> {
        let bad = || format!("Invalid UI widget '{}' in '{screen_id}'", w.id);
        if w.text.chars().count() > 4096
            || !w.font_size.is_finite()
            || !(1.0..=256.0).contains(&w.font_size)
        {
            return Err(bad());
        }
        if !w.font.is_empty() && !self.fonts.contains_key(&w.font) {
            return Err(format!("Missing UI font '{}'", w.font));
        }
        for value in w
            .layout
            .anchor
            .iter()
            .chain(&w.layout.pivot)
            .chain(&w.text_align)
        {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(bad());
            }
        }
        for axis in 0..2 {
            if !w.layout.stretch[axis] && w.layout.size[axis] <= 0.0 {
                return Err(bad());
            }
        }
        for value in w.layout.offset.iter().chain(&w.layout.size) {
            if !value.is_finite() || value.abs() > 32768.0 {
                return Err(bad());
            }
        }
        for color in [&w.color, &w.text_color, &w.focus_color] {
            if color
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err(bad());
            }
        }
        if let UiAction::ShowScreen(target) = &w.action {
            if !self.screens.contains_key(target) {
                return Err(format!("Missing UI action screen '{target}'"));
            }
        }
        for (direction, target) in &w.navigation {
            if !["up", "down", "left", "right", "next", "previous"].contains(&direction.as_str())
                || !screen
                    .widgets
                    .iter()
                    .any(|w| w.id == *target && w.kind == UiKind::Button)
            {
                return Err(format!("Invalid navigation on '{}'", w.id));
            }
        }
        let mut parent = w.parent.as_str();
        let mut seen = HashSet::from([w.id.as_str()]);
        while !parent.is_empty() {
            if !seen.insert(parent) || seen.len() > 32 {
                return Err(format!("Cyclic/deep UI parent on '{}'", w.id));
            }
            parent = &screen
                .widgets
                .iter()
                .find(|w| w.id == parent)
                .ok_or_else(|| format!("Missing UI parent '{parent}'"))?
                .parent;
        }
        Ok(())
    }
}
