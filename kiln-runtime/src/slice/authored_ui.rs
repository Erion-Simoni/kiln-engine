//! Shared saved-screen scheduling; gameplay pauses independently from UI callbacks.
use super::*;
use kiln_ui::authored::{UiInput, UiRuntime, UiViewport};

impl SliceHost {
    /// Set a physical pointer/viewport pair. Letterbox bars never hit UI widgets.
    pub fn set_ui_pointer(&mut self, area: [f32; 4], pointer: Option<[f32; 2]>, down: bool) {
        let viewport = UiViewport::new(area, self.ui.document().reference_size);
        self.ui_pointer = pointer.and_then(|point| viewport.logical_point(point));
        self.ui_pointer_down = down;
        self.ui_size = viewport.logical_size;
    }
    /// Queue physical transitions before the next fixed tick (including rapid taps).
    pub fn queue_ui_pointer(&mut self, area: [f32; 4], point: [f32; 2], down: bool) {
        if self.ui_pointer_edges.len() < 64 {
            let point =
                UiViewport::new(area, self.ui.document().reference_size).logical_point(point);
            self.ui_pointer_edges.push((point, down));
        }
    }
    pub fn queue_ui_key(&mut self, key: &str) {
        if self.ui_key_edges.len() < 64 {
            self.ui_key_edges.push(key.into());
        }
    }
    pub fn reset_ui_input(&mut self) {
        self.ui.reset_input();
        self.ui_key_edges.clear();
        self.ui_pointer_edges.clear();
    }
    pub fn ui_runtime(&self) -> &UiRuntime {
        &self.ui
    }
    pub fn ui_runtime_mut(&mut self) -> &mut UiRuntime {
        &mut self.ui
    }
    pub(super) fn call_ui(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        match id {
            80 => {
                let screen = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("ui_show_screen expects a string".into())
                })?;
                self.ui.show(screen).map_err(VmError::HostCallError)?;
                Ok(Value::Bool(true))
            }
            81 => {
                self.ui.close();
                Ok(Value::I64(0))
            }
            82 => Ok(Value::Str(self.ui.active_screen().into())),
            83 => Ok(Value::Str(
                self.ui_event
                    .as_ref()
                    .map(|e| e.name.clone())
                    .unwrap_or_default(),
            )),
            84 => Ok(Value::Str(
                self.ui_event
                    .as_ref()
                    .map(|e| e.widget.clone())
                    .unwrap_or_default(),
            )),
            85 => {
                let key = args[0].as_str().ok_or_else(|| {
                    VmError::HostCallError("ui_set_text expects a widget ID".into())
                })?;
                let text = describe(&args[1]);
                Ok(Value::Bool(self.ui.set_text(key, &text)))
            }
            86 | 87 => {
                let key = args[0]
                    .as_str()
                    .ok_or_else(|| VmError::HostCallError("UI flag expects a widget ID".into()))?;
                let value = args[1]
                    .as_bool()
                    .ok_or_else(|| VmError::HostCallError("UI flag expects a boolean".into()))?;
                Ok(Value::Bool(self.ui.set_flag(key, value, id == 86)))
            }
            88 => Ok(Value::Str(
                self.ui_event
                    .as_ref()
                    .map(|e| e.screen.clone())
                    .unwrap_or_default(),
            )),
            _ => Err(VmError::HostCallError("Unknown saved UI builtin".into())),
        }
    }
}

impl SliceSession {
    /// Read the saved UI state used by the shared renderer.
    pub fn host_ui(&self) -> &UiRuntime {
        &self.vm.host().ui
    }

    /// Update screen input/start/UI callbacks before deciding whether to advance gameplay.
    pub(super) fn dispatch_saved_ui(&mut self, world: &mut GameWorld) -> Result<(), VmError> {
        let host = self.vm.host_mut();
        let mut input = UiInput::from_keys(host.pressed_keys.iter().map(String::as_str));
        if input.next && host.pressed_keys.contains("Shift") {
            input.next = false;
            input.previous = true;
        }
        input.pressed_keys = std::mem::take(&mut host.ui_key_edges);
        input.pointer_events = std::mem::take(&mut host.ui_pointer_edges);
        input.pointer = host.ui_pointer;
        input.pointer_down = host.ui_pointer_down;
        let size = if host.ui_size[0] > 0.0 {
            host.ui_size
        } else {
            host.ui.document().reference_size
        };
        let events = host
            .ui
            .update(input, size)
            .map_err(VmError::HostCallError)?;
        if host.ui.document().screens.is_empty() {
            return Ok(());
        }
        let frame = host.frame_count.saturating_add(1);
        let keys = host.pressed_keys.clone();
        let mouse_world = host.mouse_world;
        // Start UI controllers even on an initially paused title screen.
        if self.vm.host().ui.pauses_game() || !events.is_empty() {
            for attached in &mut self.attached {
                if !world
                    .world_ref()
                    .query::<&SourceId>()
                    .iter()
                    .any(|id| id.0 == attached.entity_id)
                {
                    continue;
                }
                attached.frame = frame;
                attached
                    .session
                    .vm
                    .host_mut()
                    .pressed_keys
                    .clone_from(&keys);
                attached.session.vm.host_mut().mouse_world = mouse_world;
                if !attached.started {
                    attached.run(self.vm.host_mut(), "start", world, (0.0, 0.0), 0.0, &[])?;
                    attached.started = true;
                    self.signals
                        .append(&mut attached.session.vm.host_mut().emitted_signals);
                }
            }
        }
        for event in events {
            log::debug!(target:"kiln_runtime::ui","screen={} widget={} event={}",event.screen,event.widget,event.name);
            let host = self.vm.host_mut();
            let old_frame = host.frame_count;
            let old_phase = host.phase.clone();
            host.phase = "ui_event".into();
            host.ui_event = Some(event.clone());
            let result = self.step_program(world, (0.0, 0.0), 0.0, false, Vec::new());
            let host = self.vm.host_mut();
            host.phase = old_phase;
            host.frame_count = old_frame;
            host.ui_event = None;
            result?;
            self.signals.append(&mut self.vm.host_mut().emitted_signals);
            for attached in &mut self.attached {
                if !attached.started
                    || !world
                        .world_ref()
                        .query::<&SourceId>()
                        .iter()
                        .any(|id| id.0 == attached.entity_id)
                {
                    continue;
                }
                attached
                    .session
                    .vm
                    .host_mut()
                    .pressed_keys
                    .clone_from(&keys);
                attached.session.vm.host_mut().mouse_world = mouse_world;
                attached.session.vm.host_mut().ui_event = Some(event.clone());
                let result =
                    attached.run(self.vm.host_mut(), "ui_event", world, (0.0, 0.0), 0.0, &[]);
                attached.session.vm.host_mut().ui_event = None;
                result?;
                self.ui_commands.append(&mut attached.session.ui_commands);
                self.signals
                    .append(&mut attached.session.vm.host_mut().emitted_signals);
            }
        }
        Ok(())
    }
}
