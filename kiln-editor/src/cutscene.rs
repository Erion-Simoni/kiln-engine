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
/// An optional second take, mixed with the first using a bounded weight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blend {
    pub keys: Vec<Key>,
    #[serde(with = "scalar")]
    pub weight: f64,
    /// Optional saved game variable overriding the authored weight at playback.
    pub parameter: String,
}
/// Discrete cues are chronological and fire once per restart, never on seeking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cue {
    #[serde(with = "scalar")]
    pub time: f64,
    pub kind: CueKind,
    pub name: String,
    #[serde(with = "scalar")]
    pub value: f64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CueKind {
    Signal,
    Sound,
}
/// One property binding with strictly ordered keys beginning at time zero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub entity: String,
    pub property: TweenProperty,
    pub keys: Vec<Key>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend: Option<Blend>,
}
/// A two-link skeletal chain, mixed with the animated FK pose by weight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IkBinding {
    pub bones: [String; 3],
    pub target: String,
    #[serde(with = "scalar")]
    pub bend: f64,
    #[serde(with = "scalar")]
    pub weight: f64,
}
/// Versioned authoring data embedded in the generated source, not runtime metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cutscene {
    pub version: u32,
    pub tracks: Vec<Track>,
    /// Pause, restart, seek-to-midpoint, cancel; empty names disable a control.
    pub actions: [String; 4],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<Cue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ik: Vec<IkBinding>,
}
impl Default for Cutscene {
    fn default() -> Self {
        Self {
            version: 1,
            cues: vec![],
            ik: vec![],
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
            .flat_map(|t| {
                t.keys
                    .iter()
                    .chain(t.blend.iter().flat_map(|b| b.keys.iter()))
            })
            .map(|k| k.time)
            .chain(self.cues.iter().map(|c| c.time))
            .fold(0.0, f64::max)
    }
    pub fn key_count(&self) -> usize {
        self.tracks
            .iter()
            .map(|t| t.keys.len() + t.blend.as_ref().map_or(0, |b| b.keys.len()))
            .sum()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.tracks.is_empty()
            || self.tracks.len() > 16
            || self.key_count() > 64
            || self.cues.len() > 32
        {
            return Err("Use version 1, 1..16 tracks, at most 64 keys and 32 cues".into());
        }
        if self.ik.len() > 8 {
            return Err("At most eight IK chains".into());
        }
        let mut bones = BTreeSet::new();
        for ik in &self.ik {
            kiln_runtime::rig::IkRequest {
                bones: ik.bones.clone(),
                target: [0.0, 0.0],
                bend: ik.bend,
                weight: ik.weight,
            }
            .validate()?;
            if ik.target.is_empty()
                || ik.target.len() > 256
                || ik.bones.contains(&ik.target)
                || !bones.insert(&ik.bones[0])
                || !bones.insert(&ik.bones[1])
            {
                return Err("IK needs independent writable bones and a separate target".into());
            }
        }
        let mut bindings = BTreeSet::new();
        for track in &self.tracks {
            if track.entity.is_empty()
                || track.entity.len() > 256
                || track.entity.chars().any(char::is_control)
                || !bindings.insert((&track.entity, track.property))
            {
                return Err("Tracks need unique valid entity/property bindings".into());
            }
            validate_keys(track.property, &track.keys)?;
            if let Some(blend) = &track.blend {
                validate_keys(track.property, &blend.keys)?;
                if !blend.weight.is_finite()
                    || !(0.0..=1.0).contains(&blend.weight)
                    || blend.parameter.len() > 64
                    || blend.parameter.chars().any(char::is_control)
                {
                    return Err(
                        "Blend needs finite weight 0..1 and a parameter up to 64 bytes".into(),
                    );
                }
            }
        }
        for cue in &self.cues {
            if !cue.time.is_finite()
                || !(0.0..=60.0).contains(&cue.time)
                || !cue.value.is_finite()
                || cue.name.trim().is_empty()
                || cue.name.len() > 64
                || cue.name.chars().any(char::is_control)
            {
                return Err("Cue needs time 0..60, finite value and a name up to 64 bytes".into());
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
            return Err("Add a key or cue after time zero".into());
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
        if !self.ik.is_empty() {
            let globals = kiln_core::hierarchy_transform::world_transforms(scene)?;
            for ik in &self.ik {
                for (index, id) in ik
                    .bones
                    .iter()
                    .chain(std::iter::once(&ik.target))
                    .enumerate()
                {
                    let entity = scene
                        .entities
                        .iter()
                        .find(|e| &e.id == id)
                        .ok_or("Missing IK binding")?;
                    if entity
                        .components
                        .iter()
                        .any(|c| matches!(c, Component::Physics { .. }))
                    {
                        return Err("IK bindings cannot be physics-owned".into());
                    }
                    let (x, y, scale) = entity
                        .components
                        .iter()
                        .find_map(|c| match c {
                            Component::Transform { x, y, scale, .. } => Some((*x, *y, *scale)),
                            _ => None,
                        })
                        .ok_or("IK binding needs Transform")?;
                    let parent = entity.components.iter().find_map(|c| match c {
                        Component::Parent { parent } => Some(parent),
                        _ => None,
                    });
                    if index == 3 {
                        if parent.is_some() {
                            return Err("IK target must be a scene root".into());
                        }
                    } else {
                        let global = globals.get(id).ok_or("Missing IK world transform")?;
                        if !scale.is_finite()
                            || scale <= 0.000001
                            || global.scale.x <= 0.000001
                            || (global.scale.x - global.scale.y).abs() > global.scale.x * 0.00001
                        {
                            return Err("IK needs positive uniform scale ancestry".into());
                        }
                        if index > 0
                            && (parent != Some(&ik.bones[index - 1]) || x <= 0.000001 || y != 0.0)
                        {
                            return Err("IK needs direct +X rest links".into());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn validate_assets(&self, project: &kiln_core::Project) -> Result<(), String> {
        for cue in &self.cues {
            if cue.kind == CueKind::Sound
                && !project
                    .audio_assets
                    .iter()
                    .any(|a| a.name == cue.name && !a.is_music)
            {
                return Err(format!("Cue needs an existing sound asset: {}", cue.name));
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
        if !self.cues.is_empty() {
            out.push_str(" let previous=state_get(\"cutscene_time\",0.0);\n");
        }
        out.push_str(" if apply {\n");
        for track in &self.tracks {
            out.push_str("  if true {\n");
            emit_keys(&mut out, &track.keys, "v");
            if let Some(blend) = &track.blend {
                emit_keys(&mut out, &blend.keys, "b");
                let weight = if blend.parameter.is_empty() {
                    number(blend.weight)
                } else {
                    format!(
                        "game_get({},{})",
                        quoted(&blend.parameter),
                        number(blend.weight)
                    )
                };
                writeln!(
                    out,
                    "   let w={weight};if w<0.0 {{w=0.0;}}if w>1.0 {{w=1.0;}}"
                )
                .unwrap();
                for i in 0..track.keys[0].value.len() {
                    writeln!(
                        out,
                        "   if w==1.0 {{v{i}=b{i};}}else {{if w>0.0 {{v{i}=v{i}+(b{i}-v{i})*w;}}}}"
                    )
                    .unwrap();
                }
            }
            let last = track.keys.last().unwrap();
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
        for ik in &self.ik {
            out.push_str("  if true {\n");
            if let Some(track) = self
                .tracks
                .iter()
                .find(|t| t.entity == ik.target && t.property == TweenProperty::Position)
            {
                emit_keys(&mut out, &track.keys, "target");
                if let Some(blend) = &track.blend {
                    emit_keys(&mut out, &blend.keys, "other");
                    let w = if blend.parameter.is_empty() {
                        number(blend.weight)
                    } else {
                        format!(
                            "game_get({},{})",
                            quoted(&blend.parameter),
                            number(blend.weight)
                        )
                    };
                    writeln!(out, "   let w={w};if w<0.0 {{w=0.0;}}if w>1.0 {{w=1.0;}}").unwrap();
                    for i in 0..2 {
                        writeln!(out, "   target{i}=target{i}+(other{i}-target{i})*w;").unwrap();
                    }
                }
            } else {
                writeln!(out,"   let position=get_entity_pos({});let target0=array_get(position,0);let target1=array_get(position,1);",quoted(&ik.target)).unwrap();
            }
            writeln!(
                out,
                "   solve_ik_2d({},{},{},target0,target1,{},{});\n  }}",
                quoted(&ik.bones[0]),
                quoted(&ik.bones[1]),
                quoted(&ik.bones[2]),
                number(ik.bend),
                number(ik.weight)
            )
            .unwrap();
        }
        if !self.cues.is_empty() {
            out.push_str(" }\n if !edge3 && !edge2 {\n");
            let mut cues: Vec<_> = self.cues.iter().enumerate().collect();
            cues.sort_by(|a, b| a.1.time.total_cmp(&b.1.time));
            for (index, cue) in cues {
                writeln!(
                    out,
                    "  if edge1 {{state_set(\"cutscene_cue_{index}\",false);}}"
                )
                .unwrap();
                let crossed = if cue.time == 0.0 {
                    "(!started || edge1)".into()
                } else {
                    format!(
                        "(started && running && !paused && !edge1 && previous<{} && time>={})",
                        number(cue.time),
                        number(cue.time)
                    )
                };
                writeln!(
                    out,
                    "  if !state_get(\"cutscene_cue_{index}\",false) && {crossed} {{"
                )
                .unwrap();
                match cue.kind {
                    CueKind::Sound => {
                        writeln!(out, "   queue_sound({});", quoted(&cue.name)).unwrap();
                    }
                    CueKind::Signal => {
                        writeln!(
                            out,
                            "   emit_signal({},{});",
                            quoted(&cue.name),
                            number(cue.value)
                        )
                        .unwrap();
                    }
                }
                writeln!(out, "   state_set(\"cutscene_cue_{index}\",true);\n  }}").unwrap();
            }
            // The final shared close below terminates this cue block instead of apply.
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
fn validate_keys(property: TweenProperty, keys: &[Key]) -> Result<(), String> {
    if keys.is_empty() || keys[0].time != 0.0 {
        return Err("Tracks need a key at time zero".into());
    }
    let mut previous = -1.0;
    for key in keys {
        if !key.time.is_finite() || key.time <= previous || !(0.0..=60.0).contains(&key.time) {
            return Err("Key times must strictly increase within 0..60".into());
        }
        validate_request(property, &key.value, 0.0)?;
        previous = key.time;
    }
    Ok(())
}
fn emit_keys(out: &mut String, keys: &[Key], prefix: &str) {
    for (i, v) in keys[0].value.iter().enumerate() {
        writeln!(out, "   let {prefix}{i}={};", number(*v)).unwrap();
    }
    for pair in keys.windows(2) {
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
        for (i, (a, b)) in a.value.iter().zip(&b.value).enumerate() {
            writeln!(
                out,
                "    {prefix}{i}={}+({}-{})*f;",
                number(*a),
                number(*b),
                number(*a)
            )
            .unwrap();
        }
        out.push_str("   }\n");
    }
    let last = keys.last().unwrap();
    writeln!(out, "   if time>={} {{", number(last.time)).unwrap();
    for (i, v) in last.value.iter().enumerate() {
        writeln!(out, "    {prefix}{i}={};", number(*v)).unwrap();
    }
    out.push_str("   }\n");
}
/// Preview uses the same blend and segment formulas as generated playback.
pub fn sample(track: &Track, time: f64) -> Vec<f64> {
    let base = sample_keys(&track.keys, time);
    if let Some(blend) = &track.blend {
        let b = sample_keys(&blend.keys, time);
        base.iter()
            .zip(b)
            .map(|(a, b)| {
                if blend.weight == 1.0 {
                    b
                } else {
                    a + (b - a) * blend.weight
                }
            })
            .collect()
    } else {
        base
    }
}
pub fn sample_keys(keys: &[Key], time: f64) -> Vec<f64> {
    for pair in keys.windows(2) {
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
    keys.last().map(|k| k.value.clone()).unwrap_or_default()
}
