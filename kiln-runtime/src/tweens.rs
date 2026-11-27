//! Bounded, serializable cutscene interpolation, independent of renderer state.
use crate::{GameWorld, SourceId, Sprite};
use kiln_core::{physics_data::RigidBody2D, Transform};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
/// Shared scene-wide count, including queued new property keys.
pub const MAX_TWEENS: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweenProperty {
    Position,
    Color,
}
impl TweenProperty {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "position" => Ok(Self::Position),
            "color" => Ok(Self::Color),
            _ => Err("Tween property must be position or color".into()),
        }
    }
    fn dimensions(self) -> usize {
        match self {
            Self::Position => 2,
            Self::Color => 4,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweenEase {
    Linear,
    Smooth,
    In,
    Out,
}
impl TweenEase {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "linear" => Ok(Self::Linear),
            "smooth" => Ok(Self::Smooth),
            "in" => Ok(Self::In),
            "out" => Ok(Self::Out),
            _ => Err("Tween easing must be linear, smooth, in or out".into()),
        }
    }
    fn sample(self, t: f64) -> f64 {
        match self {
            Self::Linear => t,
            Self::Smooth => t * t * (3.0 - 2.0 * t),
            Self::In => t * t,
            Self::Out => t * (2.0 - t),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweenTrack {
    pub entity: String,
    pub property: TweenProperty,
    pub from: Vec<f64>,
    pub to: Vec<f64>,
    pub duration: f64,
    pub elapsed: f64,
    pub ease: TweenEase,
    pub paused: bool,
}
impl TweenTrack {
    pub fn validate(&self) -> Result<(), String> {
        validate_request(self.property, &self.to, self.duration)?;
        if self.entity.is_empty()
            || self.from.len() != self.property.dimensions()
            || self
                .from
                .iter()
                .any(|v| !v.is_finite() || v.abs() > f32::MAX as f64)
            || !self.elapsed.is_finite()
            || self.elapsed < 0.0
            || self.elapsed > self.duration
        {
            return Err("Invalid tween source/elapsed state".into());
        }
        Ok(())
    }
    pub(crate) fn sample(&self) -> Vec<f32> {
        if self.duration == 0.0 || self.elapsed >= self.duration {
            return self.to.iter().map(|v| *v as f32).collect();
        }
        let alpha = self.ease.sample(self.elapsed / self.duration);
        self.from
            .iter()
            .zip(&self.to)
            .map(|(a, b)| (a + (b - a) * alpha) as f32)
            .collect()
    }
}
pub fn validate_request(property: TweenProperty, to: &[f64], duration: f64) -> Result<(), String> {
    if to.len() != property.dimensions()
        || to
            .iter()
            .any(|v| !v.is_finite() || v.abs() > f32::MAX as f64)
        || (property == TweenProperty::Color && to.iter().any(|v| !(0.0..=1.0).contains(v)))
        || !duration.is_finite()
        || !(0.0..=60.0).contains(&duration)
    {
        return Err("Tween needs finite targets, duration 0..60 and color channels 0..1".into());
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq)]
pub enum TweenCommand {
    Start {
        entity: String,
        property: TweenProperty,
        to: Vec<f64>,
        duration: f64,
        ease: TweenEase,
    },
    Cancel {
        entity: String,
        property: TweenProperty,
    },
    Seek {
        entity: String,
        property: TweenProperty,
        time: f64,
    },
    Pause {
        entity: String,
        property: TweenProperty,
        paused: bool,
    },
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TweenSnapshot {
    pub tracks: Vec<TweenTrack>,
}
impl TweenSnapshot {
    pub fn track(&self, entity: &str, property: TweenProperty) -> Option<&TweenTrack> {
        self.tracks
            .iter()
            .find(|track| track.entity == entity && track.property == property)
    }
    pub fn cancel(&mut self, entity: &str, property: TweenProperty) {
        self.tracks
            .retain(|track| track.entity != entity || track.property != property);
    }
    pub fn remove_entity(&mut self, entity: &str) {
        self.tracks.retain(|track| track.entity != entity);
    }
    pub fn validate(&self, world: &GameWorld) -> Result<(), String> {
        if self.tracks.len() > MAX_TWEENS {
            return Err("Tween count exceeds 64".into());
        }
        let mut keys = BTreeSet::new();
        for track in &self.tracks {
            track.validate()?;
            if track.duration <= 0.0
                || track.elapsed >= track.duration
                || !keys.insert((&track.entity, track.property))
                || read_property(world, &track.entity, track.property).is_none()
            {
                return Err("Invalid active tween binding/duplicate/completed state".into());
            }
        }
        Ok(())
    }
}
pub fn handle(world: &GameWorld, entity: &str) -> Option<hecs::Entity> {
    world
        .world_ref()
        .query::<(hecs::Entity, &SourceId)>()
        .iter()
        .find(|(_, id)| id.0 == entity)
        .map(|(handle, _)| handle)
}
pub fn read_property(world: &GameWorld, entity: &str, property: TweenProperty) -> Option<Vec<f64>> {
    read_handle(world, handle(world, entity)?, property)
}
pub fn read_handle(
    world: &GameWorld,
    entity: hecs::Entity,
    property: TweenProperty,
) -> Option<Vec<f64>> {
    let values = match property {
        TweenProperty::Position => {
            if world.world_ref().get::<&RigidBody2D>(entity).is_ok() {
                return None;
            }
            world
                .world_ref()
                .get::<&Transform>(entity)
                .ok()?
                .position
                .to_array()
                .map(f64::from)
                .to_vec()
        }
        TweenProperty::Color => world
            .world_ref()
            .get::<&Sprite>(entity)
            .ok()?
            .color
            .map(f64::from)
            .to_vec(),
    };
    values.iter().all(|v| v.is_finite()).then_some(values)
}
pub fn write_handle(
    world: &mut GameWorld,
    entity: hecs::Entity,
    property: TweenProperty,
    values: &[f32],
) -> bool {
    if values.len() != property.dimensions()
        || values.iter().any(|v| !v.is_finite())
        || read_handle(world, entity, property).is_none()
    {
        return false;
    }
    match property {
        TweenProperty::Position => {
            let Ok(mut pose) = world.world_mut().get::<&mut Transform>(entity) else {
                return false;
            };
            pose.position = kiln_core::Vec2::new(values[0], values[1]);
        }
        TweenProperty::Color => {
            let Ok(mut sprite) = world.world_mut().get::<&mut Sprite>(entity) else {
                return false;
            };
            sprite.color.copy_from_slice(values);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_and_extreme_endpoints_stay_finite_and_exact() {
        let mut track = TweenTrack {
            entity: "actor".into(),
            property: TweenProperty::Position,
            from: vec![-f32::MAX as f64, 0.0],
            to: vec![f32::MAX as f64, 0.0],
            duration: 1.0,
            elapsed: 0.5,
            ease: TweenEase::Linear,
            paused: false,
        };
        assert_eq!(track.sample(), [0.0, 0.0]);
        track.elapsed = 1.0;
        assert_eq!(track.sample(), [f32::MAX, 0.0]);
        for (ease, expected) in [
            (TweenEase::Linear, 0.5),
            (TweenEase::Smooth, 0.5),
            (TweenEase::In, 0.25),
            (TweenEase::Out, 0.75),
        ] {
            track.from = vec![0.0, 0.0];
            track.to = vec![1.0, 1.0];
            track.elapsed = 0.5;
            track.ease = ease;
            assert_eq!(track.sample(), [expected, expected]);
        }
    }
}
