//! Browser input mapped through the actual canvas rectangle and game viewport.
use super::{App, WebInput};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::*, JsCast};

type SharedApp = Rc<RefCell<Option<App>>>;
fn listen<E: JsCast + 'static>(
    window: &web_sys::Window,
    name: &str,
    mut callback: impl FnMut(E) + 'static,
) {
    let closure = Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(event) = event.dyn_into::<E>() {
            callback(event);
        }
    }) as Box<dyn FnMut(JsValue)>);
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(false);
    let _ = window.add_event_listener_with_callback_and_add_event_listener_options(
        name,
        closure.as_ref().unchecked_ref(),
        &options,
    );
    closure.forget();
}
fn key_name(value: &str) -> String {
    match value {
        " " => "Space".into(),
        v if v.len() == 1 => v.to_ascii_lowercase(),
        v => v.into(),
    }
}
impl WebInput {
    pub(super) fn axes(&self) -> (f64, f64) {
        let has = |keys: &[&str]| keys.iter().any(|key| self.keys.contains(*key));
        let x = has(&["ArrowRight", "d"]) as i8 - has(&["ArrowLeft", "a"]) as i8;
        let y = has(&["ArrowUp", "w"]) as i8 - has(&["ArrowDown", "s"]) as i8;
        (
            (x as f64 + self.touch_axis[0]).clamp(-1.0, 1.0),
            (y as f64 + self.touch_axis[1]).clamp(-1.0, 1.0),
        )
    }
}
impl App {
    fn pointer(&mut self, x: f64, y: f64, edge: Option<bool>) {
        if let Some(canvas) = &self.canvas_element {
            let rect = canvas.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return;
            }
            self.input.mouse_x = (x - rect.left()) * self.canvas_width as f64 / rect.width();
            self.input.mouse_y = (y - rect.top()) * self.canvas_height as f64 / rect.height();
            if let Some(down) = edge {
                self.input.mouse_pressed = down;
                self.input.mouse_just_pressed = down;
                if self.input.pointer_edges.len() < 64 {
                    self.input
                        .pointer_edges
                        .push(([self.input.mouse_x as f32, self.input.mouse_y as f32], down));
                }
            }
        }
    }
    fn unlock_audio(&self) {
        if let Some(context) = &self.audio_context {
            let _ = context.resume();
        }
    }
}

pub(super) fn install(app: &SharedApp, window: &web_sys::Window) {
    let shared = app.clone();
    listen(window, "keydown", move |event: web_sys::KeyboardEvent| {
        let mut app = shared.borrow_mut();
        let Some(app) = app.as_mut() else {
            return;
        };
        app.unlock_audio();
        let key = key_name(&event.key());
        if [
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Tab",
            "Space",
            "F5",
            "F9",
        ]
        .contains(&key.as_str())
        {
            event.prevent_default();
        }
        app.input.keys.insert(key.clone());
        if !event.repeat() {
            if matches!(key.as_str(), "F5" | "F9") {
                if let Some(session) = &mut app.slice {
                    session.host_mut().request_save(0, key == "F9");
                }
            } else if app.input.key_edges.len() < 64 {
                app.input
                    .key_edges
                    .push(if key == "Tab" && event.shift_key() {
                        "BackTab".into()
                    } else {
                        key
                    });
            }
        }
    });
    let shared = app.clone();
    listen(window, "keyup", move |event: web_sys::KeyboardEvent| {
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.input.keys.remove(&key_name(&event.key()));
        }
    });
    let shared = app.clone();
    listen(window, "mousedown", move |event: web_sys::MouseEvent| {
        if event.button() != 0 {
            return;
        }
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.unlock_audio();
            app.pointer(event.client_x() as f64, event.client_y() as f64, Some(true));
        }
    });
    let shared = app.clone();
    listen(window, "mouseup", move |event: web_sys::MouseEvent| {
        if event.button() != 0 {
            return;
        }
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.pointer(
                event.client_x() as f64,
                event.client_y() as f64,
                Some(false),
            );
        }
    });
    let shared = app.clone();
    listen(window, "mousemove", move |event: web_sys::MouseEvent| {
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.pointer(event.client_x() as f64, event.client_y() as f64, None);
        }
    });
    let shared = app.clone();
    listen(window, "touchstart", move |event: web_sys::TouchEvent| {
        if let (Some(app), Some(touch)) = (shared.borrow_mut().as_mut(), event.touches().item(0)) {
            if event
                .target()
                .is_none_or(|target| !target.is_instance_of::<web_sys::HtmlCanvasElement>())
            {
                return;
            }
            event.prevent_default();
            app.unlock_audio();
            app.pointer(touch.client_x() as f64, touch.client_y() as f64, Some(true));
            app.input.touch_origin = Some([app.input.mouse_x, app.input.mouse_y]);
        }
    });
    let shared = app.clone();
    listen(window, "touchmove", move |event: web_sys::TouchEvent| {
        if let (Some(app), Some(touch)) = (shared.borrow_mut().as_mut(), event.touches().item(0)) {
            if app.input.touch_origin.is_none() {
                return;
            }
            event.prevent_default();
            app.pointer(touch.client_x() as f64, touch.client_y() as f64, None);
            if let Some(origin) = app.input.touch_origin {
                app.input.touch_axis = [
                    ((app.input.mouse_x - origin[0]) / 60.0).clamp(-1.0, 1.0),
                    ((origin[1] - app.input.mouse_y) / 60.0).clamp(-1.0, 1.0),
                ];
            }
        }
    });
    for name in ["touchend", "touchcancel"] {
        let shared = app.clone();
        listen(window, name, move |event: web_sys::TouchEvent| {
            if let Some(app) = shared.borrow_mut().as_mut() {
                if app.input.touch_origin.is_none() {
                    return;
                }
                event.prevent_default();
                if let Some(touch) = event.changed_touches().item(0) {
                    app.pointer(
                        touch.client_x() as f64,
                        touch.client_y() as f64,
                        Some(false),
                    );
                }
                app.input.touch_origin = None;
                app.input.touch_axis = [0.0; 2];
            }
        });
    }
    let shared = app.clone();
    listen(window, "blur", move |_: web_sys::Event| {
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.focused = false;
            app.input = WebInput::default();
            if let Some(session) = &mut app.slice {
                session.host_mut().reset_ui_input();
            }
            if let Some(context) = &app.audio_context {
                let _ = context.suspend();
            }
        }
    });
    let shared = app.clone();
    listen(window, "focus", move |_: web_sys::Event| {
        if let Some(app) = shared.borrow_mut().as_mut() {
            app.focused = true;
            app.game_loop = kiln_runtime::GameLoop::new();
            app.unlock_audio();
        }
    });
    let shared = app.clone();
    listen(window, "resize", move |_: web_sys::Event| {
        if let Some(app) = shared.borrow_mut().as_mut() {
            if let Some(canvas) = &app.canvas_element {
                let width = canvas.client_width().max(1) as u32;
                let height = canvas.client_height().max(1) as u32;
                canvas.set_width(width);
                canvas.set_height(height);
                app.canvas_width = width;
                app.canvas_height = height;
                app.renderer.resize(&app.surface, width, height);
            }
        }
    });
}

pub(super) fn gamepad_keys() -> Vec<String> {
    let mut output = Vec::new();
    let Some(window) = web_sys::window() else {
        return output;
    };
    let Ok(pads) = window.navigator().get_gamepads() else {
        return output;
    };
    for value in pads.iter() {
        let Ok(pad) = value.dyn_into::<web_sys::Gamepad>() else {
            continue;
        };
        if !pad.connected() {
            continue;
        }
        let buttons = pad.buttons();
        let pressed = |index: u32| {
            buttons
                .get(index)
                .dyn_into::<web_sys::GamepadButton>()
                .is_ok_and(|b| b.pressed())
        };
        for (index, key) in [
            (12, "GamepadUp"),
            (13, "GamepadDown"),
            (14, "GamepadLeft"),
            (15, "GamepadRight"),
            (0, "GamepadAccept"),
            (1, "GamepadCancel"),
            (9, "GamepadPause"),
        ] {
            if pressed(index) {
                output.push(key.into());
            }
        }
        for (runtime, browser) in [0, 1, 3, 2, 6, 7, 4, 5, 10, 11, 12, 13, 14, 15, 9, 8, 16]
            .into_iter()
            .enumerate()
        {
            if pressed(browser) {
                output.push(format!("GamepadBtn{runtime}"));
            }
        }
        for (index, axis) in pad.axes().iter().enumerate() {
            if let Some(value) = axis.as_f64().filter(|v| v.is_finite() && v.abs() > 0.1) {
                output.push(format!("GamepadAxis{index}:{value:.2}"));
            }
        }
    }
    output
}
