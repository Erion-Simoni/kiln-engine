//! Shared callback command adapter and one-per-tick tween advancement.
use super::*;
use crate::tweens::{
    self, TweenCommand as Command, TweenEase, TweenProperty as Property, TweenTrack, MAX_TWEENS,
};
impl SliceHost {
    pub(super) fn call_tween(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        let error = VmError::HostCallError;
        let entity = args[0]
            .as_str()
            .ok_or_else(|| error("Expected tween entity ID".into()))?
            .to_owned();
        let number = |index: usize| {
            args[index]
                .to_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| error("Tween expects finite numbers".into()))
        };
        let property = match id {
            131 => Property::Position,
            132 => Property::Color,
            138 => Property::Rotation,
            139 => Property::Scale,
            _ => Property::parse(
                args[1]
                    .as_str()
                    .ok_or_else(|| error("Expected tween property".into()))?,
            )
            .map_err(error)?,
        };
        if id == 135 {
            return Ok(Value::Bool(self.tweens.track(&entity, property).is_some()));
        }
        if id == 136 {
            return Ok(Value::F64(
                self.tweens
                    .track(&entity, property)
                    .map(|track| track.duration - track.elapsed)
                    .unwrap_or(0.0),
            ));
        }
        let eligible = match property {
            Property::Position => self.facts.entity_tween_positions.contains(&entity),
            Property::Rotation => self.facts.entity_tween_rotations.contains(&entity),
            Property::Scale => self.facts.entity_tween_scales.contains(&entity),
            Property::Color => self
                .facts
                .entity_sprite_colors
                .get(&entity)
                .is_some_and(|color| color.iter().all(|v| v.is_finite())),
        };
        if !eligible {
            return Ok(Value::Bool(false));
        }
        let command = match id {
            131 | 132 | 138 | 139 => {
                let (to, duration, index) = if id == 131 || id == 139 {
                    (vec![number(1)?, number(2)?], number(3)?, 4)
                } else if id == 138 {
                    (vec![number(1)?], number(2)?, 3)
                } else {
                    (
                        (1..=4).map(number).collect::<Result<Vec<_>, _>>()?,
                        number(5)?,
                        6,
                    )
                };
                tweens::validate_request(property, &to, duration).map_err(error)?;
                let ease = TweenEase::parse(
                    args[index]
                        .as_str()
                        .ok_or_else(|| error("Expected easing name".into()))?,
                )
                .map_err(error)?;
                let mut keys: std::collections::BTreeSet<_> = self
                    .tweens
                    .tracks
                    .iter()
                    .map(|t| (t.entity.clone(), t.property))
                    .collect();
                for command in &self.commands {
                    match command {
                        SliceCommand::Tween(Command::Start {
                            entity,
                            property,
                            duration,
                            ..
                        }) => {
                            if *duration == 0.0 {
                                keys.remove(&(entity.clone(), *property));
                            } else {
                                keys.insert((entity.clone(), *property));
                            }
                        }
                        SliceCommand::Tween(Command::Cancel { entity, property }) => {
                            keys.remove(&(entity.clone(), *property));
                        }
                        _ => {}
                    }
                }
                if duration > 0.0
                    && !keys.contains(&(entity.clone(), property))
                    && keys.len() >= MAX_TWEENS
                {
                    return Ok(Value::Bool(false));
                }
                Command::Start {
                    entity,
                    property,
                    to,
                    duration,
                    ease,
                }
            }
            133 => Command::Cancel { entity, property },
            134 => {
                let time = number(2)?;
                if time < 0.0 {
                    return Err(error("Tween seek time must be nonnegative".into()));
                }
                Command::Seek {
                    entity,
                    property,
                    time,
                }
            }
            137 => Command::Pause {
                entity,
                property,
                paused: args[2]
                    .as_bool()
                    .ok_or_else(|| error("Expected tween pause boolean".into()))?,
            },
            _ => unreachable!(),
        };
        self.commands.push(SliceCommand::Tween(command));
        Ok(Value::Bool(true))
    }
}
impl SliceSession {
    pub(super) fn apply_tween(&mut self, command: Command, world: &mut GameWorld) {
        match command {
            Command::Start {
                entity,
                property,
                to,
                duration,
                ease,
            } => {
                let Some(from) = tweens::read_property(world, &entity, property) else {
                    return;
                };
                let track = TweenTrack {
                    entity: entity.clone(),
                    property,
                    from,
                    to,
                    duration,
                    elapsed: 0.0,
                    ease,
                    paused: false,
                };
                if track.validate().is_err() {
                    return;
                }
                if duration == 0.0 {
                    if let Some(handle) = tweens::handle(world, &entity) {
                        tweens::write_handle(world, handle, property, &track.sample());
                    }
                    self.host_mut().tweens.cancel(&entity, property);
                    return;
                }
                let existing = self.host_mut().tweens.track(&entity, property).is_some();
                if !existing && self.host_mut().tweens.tracks.len() >= MAX_TWEENS {
                    return;
                }
                self.host_mut().tweens.cancel(&entity, property);
                self.host_mut().tweens.tracks.push(track);
            }
            Command::Cancel { entity, property } => {
                self.host_mut().tweens.cancel(&entity, property)
            }
            Command::Pause {
                entity,
                property,
                paused,
            } => {
                if let Some(track) = self
                    .host_mut()
                    .tweens
                    .tracks
                    .iter_mut()
                    .find(|t| t.entity == entity && t.property == property)
                {
                    track.paused = paused;
                }
            }
            Command::Seek {
                entity,
                property,
                time,
            } => {
                let sample = {
                    let Some(track) = self
                        .host_mut()
                        .tweens
                        .tracks
                        .iter_mut()
                        .find(|t| t.entity == entity && t.property == property)
                    else {
                        return;
                    };
                    track.elapsed = time.min(track.duration);
                    track.sample()
                };
                if let Some(handle) = tweens::handle(world, &entity) {
                    tweens::write_handle(world, handle, property, &sample);
                }
                if self
                    .host_mut()
                    .tweens
                    .track(&entity, property)
                    .is_some_and(|t| t.elapsed >= t.duration)
                {
                    self.host_mut().tweens.cancel(&entity, property);
                }
            }
        }
    }
    pub(super) fn advance_tweens(&mut self, world: &mut GameWorld, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 || self.host_mut().tweens.tracks.is_empty() {
            return;
        }
        let handles: HashMap<_, _> = world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .map(|(handle, id)| (id.0.clone(), handle))
            .collect();
        let mut tracks = std::mem::take(&mut self.host_mut().tweens.tracks);
        tracks.retain_mut(|track| {
            if !track.paused {
                track.elapsed = (track.elapsed + dt).min(track.duration);
            }
            let valid = handles.get(&track.entity).is_some_and(|handle| {
                tweens::write_handle(world, *handle, track.property, &track.sample())
            });
            valid && track.elapsed < track.duration
        });
        self.host_mut().tweens.tracks = tracks;
    }
}
