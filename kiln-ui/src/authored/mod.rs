//! Saved screens: layout, input, focus and quads shared by all native hosts.
mod draw;
pub use draw::{clip_quad, MAX_UI_QUADS};
pub mod fonts;
pub mod layout;
mod navigation;
mod save;
pub use fonts::UiFonts;
use kiln_core::ui::{UiAction, UiDocument, UiKind};
pub use layout::UiViewport;
use layout::{resolve, ResolvedWidget};
pub use save::UiSnapshot;

/// Digital state, not repeat events. The runtime detects edges itself.
#[derive(Debug, Clone, Default)]
pub struct UiInput {
    pub pointer: Option<[f32; 2]>,
    pub pointer_down: bool,
    /// Ordered transitions preserve quick clicks that press and release between ticks.
    pub pointer_events: Vec<(Option<[f32; 2]>, bool)>,
    /// One-shot tokens preserve quick keyboard taps between ticks.
    pub pressed_keys: Vec<String>,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub next: bool,
    pub previous: bool,
    pub accept: bool,
    pub cancel: bool,
}

impl UiInput {
    /// Normalized tokens produced by keyboard or kiln-input's gamepad adapter.
    pub fn from_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Self {
        let mut input = Self::default();
        for key in keys {
            match key {
                "ArrowUp" | "GamepadUp" => input.up = true,
                "ArrowDown" | "GamepadDown" => input.down = true,
                "ArrowLeft" | "GamepadLeft" => input.left = true,
                "ArrowRight" | "GamepadRight" => input.right = true,
                "Tab" => input.next = true,
                "BackTab" => input.previous = true,
                "Enter" | "Space" | "GamepadAccept" => input.accept = true,
                "Escape" | "GamepadCancel" | "GamepadPause" => input.cancel = true,
                _ => {}
            }
        }
        input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiEvent {
    pub screen: String,
    pub widget: String,
    pub name: String,
}

/// Runtime copy of authored data plus ephemeral focus, screen stack and input edges.
#[derive(Debug, Clone, Default)]
pub struct UiRuntime {
    document: UiDocument,
    active: String,
    stack: Vec<(String, Option<String>)>,
    focus: Option<String>,
    hovered: Option<String>,
    pressed: Option<String>,
    previous: UiInput,
    clicked: Vec<String>,
    logical_size: [f32; 2],
}

impl UiRuntime {
    pub fn new(document: &UiDocument, scene: &str) -> Result<Self, String> {
        document.validate()?;
        let mut ui = Self {
            document: document.clone(),
            logical_size: document.reference_size,
            ..Default::default()
        };
        let initial = document
            .scene_screens
            .get(scene)
            .unwrap_or(&document.initial_screen);
        ui.active = initial.clone();
        ui.repair_focus();
        Ok(ui)
    }
    /// Discard pointer capture and held edges when a host loses focus.
    pub fn reset_input(&mut self) {
        self.previous = UiInput::default();
        self.pressed = None;
        self.hovered = None;
    }
    pub fn document(&self) -> &UiDocument {
        &self.document
    }
    pub fn active_screen(&self) -> &str {
        &self.active
    }
    pub fn focused(&self) -> Option<&str> {
        self.focus.as_deref()
    }
    pub fn clicked(&self, id: &str) -> bool {
        self.clicked.iter().any(|key| key == id)
    }
    pub fn pauses_game(&self) -> bool {
        self.document
            .screens
            .get(&self.active)
            .is_some_and(|s| s.pause_game)
    }
    pub fn captures_input(&self) -> bool {
        self.document
            .screens
            .get(&self.active)
            .is_some_and(|s| s.modal)
    }
    pub fn resolved(&self) -> Vec<ResolvedWidget> {
        self.document
            .screens
            .get(&self.active)
            .map(|s| resolve(s, self.logical_size))
            .unwrap_or_default()
    }
    pub fn show(&mut self, screen: &str) -> Result<(), String> {
        if screen.is_empty() {
            self.close();
            return Ok(());
        }
        if !self.document.screens.contains_key(screen) {
            return Err(format!("Missing UI screen '{screen}'"));
        }
        if self.active != screen {
            if self.stack.len() >= 32 {
                return Err("UI screen stack limit exceeded".into());
            }
            self.stack.push((self.active.clone(), self.focus.clone()));
            self.active = screen.into();
            self.pressed = None;
            self.hovered = None;
            self.focus = None;
            self.repair_focus();
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.active.clear();
        self.stack.clear();
        self.focus = None;
        self.pressed = None;
        self.hovered = None;
    }
    pub fn back(&mut self) {
        let (screen, focus) = self.stack.pop().unwrap_or_default();
        self.active = screen;
        self.focus = focus;
        self.pressed = None;
        self.hovered = None;
        self.repair_focus();
    }
    pub fn set_text(&mut self, id: &str, text: &str) -> bool {
        if text.chars().count() > 4096 {
            return false;
        }
        let Some(widget) = self
            .document
            .screens
            .get_mut(&self.active)
            .and_then(|s| s.widgets.iter_mut().find(|w| w.id == id))
        else {
            return false;
        };
        let before = std::mem::replace(&mut widget.text, text.into());
        if self.document.validate().is_err() {
            self.document
                .screens
                .get_mut(&self.active)
                .unwrap()
                .widgets
                .iter_mut()
                .find(|w| w.id == id)
                .unwrap()
                .text = before;
            return false;
        }
        true
    }
    pub fn set_flag(&mut self, id: &str, value: bool, visibility: bool) -> bool {
        let Some(w) = self
            .document
            .screens
            .get_mut(&self.active)
            .and_then(|s| s.widgets.iter_mut().find(|w| w.id == id))
        else {
            return false;
        };
        if visibility {
            w.visible = value;
        } else {
            w.enabled = value;
        }
        self.repair_focus();
        true
    }
    /// Resolve input every host tick, including while the active screen pauses gameplay.
    pub fn update(&mut self, input: UiInput, size: [f32; 2]) -> Result<Vec<UiEvent>, String> {
        self.logical_size = size;
        self.clicked.clear();
        self.repair_focus();
        let mut events = Vec::new();
        let edges = UiInput::from_keys(input.pressed_keys.iter().map(String::as_str));
        if (input.cancel && !self.previous.cancel) || edges.cancel {
            if self.active.is_empty()
                || self
                    .document
                    .screens
                    .get(&self.active)
                    .is_some_and(|screen| !screen.modal && !screen.pause_game)
            {
                let pause = self.document.pause_screen.clone();
                if !pause.is_empty() {
                    self.show(&pause)?;
                }
            } else if self
                .document
                .screens
                .get(&self.active)
                .is_some_and(|s| s.allow_cancel)
            {
                self.back();
            }
            self.previous = input;
            return Ok(events);
        }
        for (direction, pressed, before) in [
            ("up", input.up, self.previous.up),
            ("down", input.down, self.previous.down),
            ("left", input.left, self.previous.left),
            ("right", input.right, self.previous.right),
            ("next", input.next, self.previous.next),
            ("previous", input.previous, self.previous.previous),
        ] {
            let edge = match direction {
                "up" => edges.up,
                "down" => edges.down,
                "left" => edges.left,
                "right" => edges.right,
                "next" => edges.next,
                _ => edges.previous,
            };
            if (pressed && !before) || edge {
                self.navigate(direction);
                break;
            }
        }
        let hit = input.pointer.and_then(|point| self.hit(point));
        // A stationary pointer must not steal focus back from keyboard/controller.
        if input.pointer != self.previous.pointer {
            if let Some(id) = &hit {
                self.focus = Some(id.clone());
            }
        }
        self.hovered = hit.clone();
        let mut activation = None;
        if input.pointer_events.is_empty() {
            if input.pointer_down && !self.previous.pointer_down {
                self.pressed = hit.clone();
                if hit.is_some() {
                    self.focus = hit.clone();
                }
            }
            if !input.pointer_down && self.previous.pointer_down {
                let pressed = self.pressed.take();
                if pressed == hit {
                    activation = pressed;
                }
            }
        } else {
            for (point, down) in &input.pointer_events {
                let target = point.and_then(|point| self.hit(point));
                if *down {
                    self.pressed = target.clone();
                    if target.is_some() {
                        self.focus = target;
                    }
                } else {
                    let pressed = self.pressed.take();
                    if pressed == target && activation.is_none() {
                        activation = pressed;
                    }
                }
            }
        }
        if activation.is_none() && ((input.accept && !self.previous.accept) || edges.accept) {
            activation = self.focus.clone();
        }
        self.previous = input;
        if let Some(id) = activation {
            if let Some(screen) = self.document.screens.get(&self.active) {
                if let Some(w) = screen.widgets.iter().find(|w| w.id == id) {
                    let event = UiEvent {
                        screen: self.active.clone(),
                        widget: id.clone(),
                        name: if w.event.is_empty() {
                            id.clone()
                        } else {
                            w.event.clone()
                        },
                    };
                    let action = w.action.clone();
                    self.clicked.push(id);
                    events.push(event);
                    match action {
                        UiAction::Event => {}
                        UiAction::ShowScreen(target) => self.show(&target)?,
                        UiAction::Back => self.back(),
                        UiAction::Close => self.close(),
                    }
                }
            }
        }
        Ok(events)
    }
    fn hit(&self, point: [f32; 2]) -> Option<String> {
        let screen = self.document.screens.get(&self.active)?;
        for row in self.resolved().iter().rev() {
            let w = &screen.widgets[row.index];
            if row.visible && row.clip.contains_point(point[0], point[1]) {
                if w.kind == UiKind::Button {
                    return row.enabled.then(|| w.id.clone());
                }
                if w.kind == UiKind::Panel && w.blocks_pointer {
                    return None;
                }
            }
        }
        None
    }
}
