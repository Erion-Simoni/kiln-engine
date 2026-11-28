//! Bounded source-backed cutscene documents and deterministic script generation.
use kiln_core::{Component, Scene};
use kiln_runtime::tweens::{validate_request, TweenEase, TweenProperty};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt::Write};
const HEADER: &str = "// kiln-cutscene-v1 ";
// Decimal strings preserve exact f64 bits without changing workspace serde features.
// Numeric authoring JSON remains accepted for convenient hand-authored fixtures.
#[derive(Deserialize)]
#[serde(untagged)]
enum Scalar {
    Text(String),
    Number(f64),
}
impl Scalar {
    fn value<E: serde::de::Error>(self) -> Result<f64, E> {
        match self {
            Self::Text(text) => text.parse().map_err(E::custom),
            Self::Number(value) => Ok(value),
        }
    }
}
mod scalar {
    use super::*;
    pub fn serialize<S: serde::Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Scalar::deserialize(deserializer)?.value()
    }
}
mod values {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        values: &[f64],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<f64>, D::Error> {
        Vec::<Scalar>::deserialize(deserializer)?
            .into_iter()
            .map(Scalar::value)
            .collect()
    }
}

/// An explicit value at a timeline time; easing controls the incoming segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Key {
    #[serde(with = "scalar")]
    pub time: f64,
    #[serde(with = "values")]
    pub value: Vec<f64>,
    pub ease: TweenEase,
}
/// One property binding with strictly ordered keys beginning at time zero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub entity: String,
    pub property: TweenProperty,
    pub keys: Vec<Key>,
}
/// Versioned authoring data embedded in the generated source, not runtime metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cutscene {
    pub version: u32,
    pub tracks: Vec<Track>,
    /// Pause, restart, seek-to-midpoint, cancel; empty names disable a control.
    pub actions: [String; 4],
}
impl Default for Cutscene {
    fn default() -> Self {
        Self {
            version: 1,
            tracks: vec![],
            actions: [
                "pause".into(),
                "restart".into(),
                "seek".into(),
                "cancel".into(),
            ],
        }
    }
}
pub fn property_name(property: TweenProperty) -> &'static str {
    match property {
        TweenProperty::Position => "position",
        TweenProperty::Rotation => "rotation",
        TweenProperty::Scale => "scale",
        TweenProperty::Color => "color",
    }
}
fn number(value: f64) -> String {
    // KilnScript numeric literals do not accept exponent notation.
    let mut text = format!("{value}");
    if !text.contains('.') {
        text.push_str(".0");
    }
    text
}
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization")
}
impl Cutscene {
    pub fn duration(&self) -> f64 {
        self.tracks
            .iter()
            .filter_map(|t| t.keys.last())
            .map(|k| k.time)
            .fold(0.0, f64::max)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.tracks.is_empty()
            || self.tracks.len() > 16
            || self.tracks.iter().map(|t| t.keys.len()).sum::<usize>() > 64
        {
            return Err("Use version 1 with 1..16 tracks and at most 64 total keys".into());
        }
        let mut bindings = BTreeSet::new();
        for track in &self.tracks {
            if track.entity.is_empty()
                || track.entity.len() > 256
                || track.entity.chars().any(char::is_control)
                || !bindings.insert((&track.entity, track.property))
                || track.keys.is_empty()
                || track.keys[0].time != 0.0
            {
                return Err(
                    "Tracks need unique entity/property bindings and a key at time zero".into(),
                );
            }
            let mut previous = -1.0;
            for key in &track.keys {
                if !key.time.is_finite()
                    || key.time <= previous
                    || !(0.0..=60.0).contains(&key.time)
                {
                    return Err("Key times must strictly increase within 0..60 seconds".into());
                }
                validate_request(track.property, &key.value, 0.0)?;
                previous = key.time;
            }
        }
        let mut names = BTreeSet::new();
        for action in &self.actions {
            if action.len() > 64
                || action.chars().any(char::is_control)
                || (!action.is_empty() && !names.insert(action))
            {
                return Err("Control action names must be distinct and at most 64 bytes".into());
            }
        }
        if self.duration() <= 0.0 {
            return Err("Add a key after time zero".into());
        }
        Ok(())
    }
    pub fn validate_scene(&self, scene: &Scene) -> Result<(), String> {
        self.validate()?;
        for track in &self.tracks {
            let entity = scene
                .entities
                .iter()
                .find(|e| e.id == track.entity)
                .ok_or_else(|| format!("Missing cutscene entity {}", track.entity))?;
            let valid = if track.property == TweenProperty::Color {
                entity
                    .components
                    .iter()
                    .any(|c| matches!(c, Component::Sprite { .. }))
            } else {
                entity
                    .components
                    .iter()
                    .any(|c| matches!(c, Component::Transform { .. }))
                    && !entity
                        .components
                        .iter()
                        .any(|c| matches!(c, Component::Physics { .. }))
            };
            if !valid {
                return Err(format!(
                    "Ineligible {} binding on {}",
                    property_name(track.property),
                    track.entity
                ));
            }
        }
        Ok(())
    }
    /// Emit a self-contained script that retains its clock through logical saves.
    pub fn source(&self) -> Result<String, String> {
        self.validate()?;
        let duration = number(self.duration());
        let mut out = format!("{HEADER}{}\n// Generated cutscene. Reopen in Animation Editor to change keys.\nif script_phase()==\"update\" {{\n let time=state_get(\"cutscene_time\",0.0);\n let started=state_get(\"cutscene_started\",false);\n let paused=state_get(\"cutscene_paused\",false);\n let running=state_get(\"cutscene_running\",true);\n let apply=!started;\n",serde_json::to_string(self).map_err(|e|e.to_string())?);
        for (i, action) in self.actions.iter().enumerate() {
            let pressed = if action.is_empty() {
                "false".into()
            } else {
                format!("get_action({})", quoted(action))
            };
            writeln!(out," let pressed{i}={pressed};let edge{i}=pressed{i} && !state_get(\"cutscene_held{i}\",false);state_set(\"cutscene_held{i}\",pressed{i});").unwrap();
        }
        writeln!(out," if edge0 {{paused=!paused;}}\n if started && running && !paused && !edge1 && !edge2 && !edge3 {{let dt=get_delta_time();if dt>0.0 {{time=time+dt;}}if time>{duration} {{time={duration};}}apply=true;}}\n if edge1 {{time=0.0;paused=false;running=true;apply=true;}}\n if edge2 {{time={};running=true;apply=true;}}\n if edge3 {{running=false;apply=false;}}",number(self.duration()/2.0)).unwrap();
        out.push_str(" if apply {\n");
        for track in &self.tracks {
            out.push_str("  if true {\n");
            for (i, v) in track.keys[0].value.iter().enumerate() {
                writeln!(out, "   let v{i}={};", number(*v)).unwrap();
            }
            for pair in track.keys.windows(2) {
                let a = &pair[0];
                let b = &pair[1];
                writeln!(
                    out,
                    "   if time>={} && time<{} {{let t=(time-{})/{};",
                    number(a.time),
                    number(b.time),
                    number(a.time),
                    number(b.time - a.time)
                )
                .unwrap();
                let ease = match b.ease {
                    TweenEase::Linear => "t",
                    TweenEase::Smooth => "t*t*(3.0-2.0*t)",
                    TweenEase::In => "t*t",
                    TweenEase::Out => "t*(2.0-t)",
                };
                writeln!(out, "    let f={ease};").unwrap();
                for (i, (from, to)) in a.value.iter().zip(&b.value).enumerate() {
                    writeln!(
                        out,
                        "    v{i}={}+({}-{})*f;",
                        number(*from),
                        number(*to),
                        number(*from)
                    )
                    .unwrap();
                }
                out.push_str("   }\n");
            }
            let last = track.keys.last().unwrap();
            writeln!(out, "   if time>={} {{", number(last.time)).unwrap();
            for (i, v) in last.value.iter().enumerate() {
                writeln!(out, "    v{i}={};", number(*v)).unwrap();
            }
            out.push_str("   }\n");
            let args = (0..last.value.len())
                .map(|i| format!("v{i}"))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "   tween_{}({},{args},0.0,\"linear\");\n  }}",
                property_name(track.property),
                quoted(&track.entity)
            )
            .unwrap();
        }
        writeln!(out," }}\n if time>={duration} {{running=false;}}\n state_set(\"cutscene_time\",time);state_set(\"cutscene_started\",true);state_set(\"cutscene_paused\",paused);state_set(\"cutscene_running\",running);\n}}").unwrap();
        if out.len() > 1_048_576 {
            return Err("Generated source exceeds 1 MiB".into());
        }
        kiln_runtime::SliceSession::new(&out)
            .map_err(|e| format!("Generated cutscene does not compile: {e:?}"))?;
        Ok(out)
    }
    /// Reject custom edits rather than silently discarding them during regeneration.
    pub fn from_source(source: &str) -> Result<Self, String> {
        if source.len() > 1_048_576 {
            return Err("Cutscene source exceeds 1 MiB".into());
        }
        let data = source
            .lines()
            .next()
            .and_then(|line| line.strip_prefix(HEADER))
            .ok_or("Open a generated cutscene script first")?;
        let doc: Self = serde_json::from_str(data).map_err(|e| e.to_string())?;
        if doc.source()? != source {
            return Err("Source has manual edits; keep it and generate to a new file".into());
        }
        Ok(doc)
    }
}
/// Value preview uses the same segment boundaries and easing as emitted source.
pub fn sample(track: &Track, time: f64) -> Vec<f64> {
    for pair in track.keys.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        if time < b.time {
            let t = ((time - a.time) / (b.time - a.time)).clamp(0.0, 1.0);
            let f = match b.ease {
                TweenEase::Linear => t,
                TweenEase::Smooth => t * t * (3.0 - 2.0 * t),
                TweenEase::In => t * t,
                TweenEase::Out => t * (2.0 - t),
            };
            return a
                .value
                .iter()
                .zip(&b.value)
                .map(|(a, b)| a + (b - a) * f)
                .collect();
        }
    }
    track
        .keys
        .last()
        .map(|k| k.value.clone())
        .unwrap_or_default()
}
