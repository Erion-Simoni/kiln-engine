//! Preserve a gameplay tick's systems, events and callback cursor across source stops.
use super::*;
use std::collections::VecDeque;
#[derive(Clone)]
enum Event {
    None,
    Animation(usize),
    Signal(usize),
    Collision(String),
}
#[derive(Clone)]
struct Callback {
    phase: &'static str,
    event: Event,
}
pub(super) struct PendingTick {
    input: (f64, f64),
    dt: f64,
    paused: bool,
    root_done: bool,
    prepared: bool,
    index: usize,
    entered: bool,
    keys: std::collections::HashSet<String>,
    mouse: [f64; 2],
    save_dir: Option<std::path::PathBuf>,
    collisions: Vec<(String, String, bool)>,
    animations: Vec<(String, kiln_core::FrameEvent)>,
    signals: Vec<(String, Value)>,
    queue: VecDeque<Callback>,
}
impl SliceSession {
    pub(super) fn step_resumable(
        &mut self,
        world: &mut GameWorld,
        input: (f64, f64),
        dt: f64,
    ) -> Result<(), VmError> {
        if self.source_stop().is_some() {
            return Err(VmError::DebugSuspended);
        }
        let mut tick = if let Some(tick) = self.pending_tick.take() {
            tick
        } else {
            self.work_budget.reset();
            #[cfg(feature = "audio")]
            if let Some(audio) = &mut self.vm.host_mut().audio {
                audio.advance_silent(dt);
            }
            self.source_debug.mute(true);
            let result = self.dispatch_saved_ui(world);
            self.source_debug.mute(false);
            result?;
            let paused = self.vm.host().ui.pauses_game();
            let captures = self.vm.host().ui.captures_input();
            PendingTick {
                input: if captures { (0.0, 0.0) } else { input },
                dt,
                paused,
                root_done: paused,
                prepared: false,
                index: 0,
                entered: false,
                keys: if captures {
                    Default::default()
                } else {
                    self.vm.host().pressed_keys.clone()
                },
                mouse: self.vm.host().mouse_world,
                save_dir: self.vm.host().save_dir.clone(),
                collisions: Vec::new(),
                animations: Vec::new(),
                signals: Vec::new(),
                queue: VecDeque::new(),
            }
        };
        let result = self.continue_tick(world, &mut tick);
        if matches!(result, Err(VmError::DebugSuspended)) {
            self.pending_tick = Some(tick);
        }
        result
    }
    fn continue_tick(
        &mut self,
        world: &mut GameWorld,
        tick: &mut PendingTick,
    ) -> Result<(), VmError> {
        if !tick.root_done {
            let keys = std::mem::replace(&mut self.vm.host_mut().pressed_keys, tick.keys.clone());
            self.vm.host_mut().mouse_world = tick.mouse;
            let result = self.step_program(world, tick.input, tick.dt, true, Vec::new());
            self.vm.host_mut().pressed_keys = keys;
            result?;
            tick.root_done = true;
        }
        if !tick.prepared {
            tick.collisions = if tick.paused {
                Vec::new()
            } else {
                self.vm.host().facts.collision_events.clone()
            };
            tick.animations = world
                .world_mut()
                .query_mut::<(&SourceId, &mut crate::AnimationPlayer)>()
                .into_iter()
                .flat_map(|(id, animation)| {
                    animation
                        .take_events()
                        .into_iter()
                        .map(|event| (id.0.clone(), event))
                        .collect::<Vec<_>>()
                })
                .collect();
            tick.signals = std::mem::take(&mut self.signals);
            self.signals.append(&mut self.vm.host_mut().emitted_signals);
            tick.prepared = true;
        }
        while tick.index < self.attached.len() {
            let attached = &mut self.attached[tick.index];
            if !tick.entered {
                attached.frame = self.vm.host().frame_count;
                let present = world
                    .world_ref()
                    .query::<&SourceId>()
                    .iter()
                    .any(|id| id.0 == attached.entity_id);
                if !present {
                    if attached.started {
                        tick.queue.push_back(Callback {
                            phase: "stop",
                            event: Event::None,
                        });
                    }
                } else {
                    attached.session.vm.host_mut().pressed_keys = tick.keys.clone();
                    attached.session.vm.host_mut().mouse_world = tick.mouse;
                    attached.session.vm.host_mut().save_dir = tick.save_dir.clone();
                    if !attached.started {
                        attached.started = true;
                        tick.queue.push_back(Callback {
                            phase: "start",
                            event: Event::None,
                        });
                    }
                    for (index, (id, _)) in tick.animations.iter().enumerate() {
                        if id == &attached.entity_id {
                            tick.queue.push_back(Callback {
                                phase: "animation_event",
                                event: Event::Animation(index),
                            });
                        }
                    }
                    for index in 0..tick.signals.len() {
                        tick.queue.push_back(Callback {
                            phase: "signal",
                            event: Event::Signal(index),
                        });
                    }
                    for (a, b, started) in &tick.collisions {
                        let other = if a == &attached.entity_id {
                            b
                        } else if b == &attached.entity_id {
                            a
                        } else {
                            continue;
                        };
                        tick.queue.push_back(Callback {
                            phase: if *started {
                                "collision_enter"
                            } else {
                                "collision_exit"
                            },
                            event: Event::Collision(other.clone()),
                        });
                    }
                    tick.queue.push_back(Callback {
                        phase: "update",
                        event: Event::None,
                    });
                }
                tick.entered = true;
            }
            while let Some(callback) = tick.queue.front() {
                let host = attached.session.vm.host_mut();
                match &callback.event {
                    Event::None => {}
                    Event::Animation(index) => {
                        let event = &tick.animations[*index].1;
                        host.animation_event = (
                            event.name.clone(),
                            event.payload.clone().unwrap_or_default(),
                        )
                    }
                    Event::Signal(index) => {
                        let (name, value) = &tick.signals[*index];
                        host.signal_name = name.clone();
                        host.signal_value = Some(value.clone());
                    }
                    Event::Collision(other) => host.event_entity = other.clone(),
                }
                if callback.phase != "update" || !self.vm.host().ui.pauses_game() {
                    attached.run(
                        self.vm.host_mut(),
                        callback.phase,
                        world,
                        tick.input,
                        if callback.phase == "update" {
                            tick.dt
                        } else {
                            0.0
                        },
                        &tick.collisions,
                    )?;
                }
                if callback.phase == "stop" {
                    attached.started = false;
                }
                tick.queue.pop_front();
            }
            self.ui_commands.append(&mut attached.session.ui_commands);
            self.signals
                .append(&mut attached.session.vm.host_mut().emitted_signals);
            tick.index += 1;
            tick.entered = false;
        }
        if self.signals.len() > 256 {
            return Err(VmError::HostCallError(
                "scene signal queue limit reached".into(),
            ));
        }
        Ok(())
    }
}
