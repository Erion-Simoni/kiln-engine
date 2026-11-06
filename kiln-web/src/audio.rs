//! Bounded Web Audio voices, looping music, pause offsets and named gain buses.
use kiln_core::{
    audio::{gain, AudioMixerSettings},
    project::AudioAsset,
};
use kiln_runtime::audio_backend::PlatformAudio;
use kiln_runtime::ScriptValue as Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode};

struct Voice {
    name: String,
    bus: String,
    volume: f32,
    music: bool,
    looped: bool,
    buffer: AudioBuffer,
    source: Option<AudioBufferSourceNode>,
    gain: GainNode,
    offset: f64,
    started: f64,
}
impl Voice {
    fn finished(&self, now: f64) -> bool {
        self.source.is_some()
            && !self.looped
            && now - self.started + self.offset >= self.buffer.duration()
    }
    fn stop(&mut self) {
        if let Some(source) = self.source.take() {
            let _ =
                wasm_bindgen::JsCast::unchecked_ref::<web_sys::AudioScheduledSourceNode>(&source)
                    .stop();
            let _ = source.disconnect();
        }
    }
    fn start(&mut self, context: &AudioContext) -> Result<(), String> {
        let source = context.create_buffer_source().map_err(js_error)?;
        source.set_buffer(Some(&self.buffer));
        source.set_loop(self.looped);
        source
            .connect_with_audio_node(&self.gain)
            .map_err(js_error)?;
        source
            .start_with_when_and_grain_offset(0.0, self.offset)
            .map_err(js_error)?;
        self.started = context.current_time();
        self.source = Some(source);
        Ok(())
    }
    fn pause(&mut self, now: f64) {
        if self.source.is_none() {
            return;
        }
        self.offset += now - self.started;
        if self.looped && self.buffer.duration() > 0.0 {
            self.offset = self.offset.rem_euclid(self.buffer.duration());
        }
        self.stop();
    }
}
impl Drop for Voice {
    fn drop(&mut self) {
        self.stop();
        let _ = self.gain.disconnect();
    }
}

pub struct WebAudio {
    pub context: AudioContext,
    assets: HashMap<String, (AudioAsset, AudioBuffer)>,
    settings: AudioMixerSettings,
    voices: Vec<Voice>,
}
impl WebAudio {
    pub fn new(
        context: AudioContext,
        assets: HashMap<String, (AudioAsset, AudioBuffer)>,
        settings: AudioMixerSettings,
    ) -> Self {
        Self {
            context,
            assets,
            settings,
            voices: Vec::new(),
        }
    }
    pub fn prune(&mut self) {
        let now = self.context.current_time();
        self.voices.retain(|v| !v.finished(now));
    }
    fn gains(&mut self) {
        for v in &self.voices {
            v.gain
                .gain()
                .set_value(self.settings.bus_gain(&v.bus) * v.volume);
        }
    }
    fn play(&mut self, name: &str, music: bool) -> Result<(), String> {
        self.prune();
        let Some((asset, buffer)) = self.assets.get(name).cloned() else {
            return Err(format!("Missing audio clip: {name}"));
        };
        if asset.is_music != music {
            return Err("Audio clip kind does not match sound/music command".into());
        }
        if music {
            self.voices.retain(|v| !v.music);
        }
        if self.voices.len() >= 128 {
            return Ok(());
        }
        let node = self.context.create_gain().map_err(js_error)?;
        node.connect_with_audio_node(&self.context.destination())
            .map_err(js_error)?;
        let bus = asset.bus_name().to_owned();
        node.gain()
            .set_value(self.settings.bus_gain(&bus) * gain(asset.volume));
        let mut voice = Voice {
            name: name.into(),
            bus,
            volume: gain(asset.volume),
            music,
            looped: music && asset.looped,
            buffer,
            source: None,
            gain: node,
            offset: 0.0,
            started: 0.0,
        };
        voice.start(&self.context)?;
        self.voices.push(voice);
        Ok(())
    }
    fn call(&mut self, id: u32, args: &[Value]) -> Result<Value, String> {
        self.prune();
        let name = args.first().and_then(Value::as_str).unwrap_or_default();
        let volume = args.first().and_then(Value::to_f64).unwrap_or(0.0) as f32;
        match id {
            12 => self.play(name, false)?,
            14 => self.play(name, true)?,
            13 => self.voices.retain(|v| v.name != name),
            15 => self.voices.retain(|v| !v.music),
            23 => self.voices.clear(),
            16 => {
                self.settings.master_volume = gain(volume);
                self.gains();
            }
            17 | 18 => {
                self.settings
                    .buses
                    .entry(if id == 17 { "SFX" } else { "Music" }.into())
                    .or_default()
                    .volume = gain(volume);
                self.gains();
            }
            19 | 21 => {
                let now = self.context.current_time();
                for v in &mut self.voices {
                    if (id == 21 && v.music) || (id == 19 && v.name == name) {
                        v.pause(now);
                    }
                }
            }
            20 | 22 => {
                for v in &mut self.voices {
                    if v.source.is_none() && ((id == 22 && v.music) || (id == 20 && v.name == name))
                    {
                        v.start(&self.context)?;
                    }
                }
            }
            24 => {
                return Ok(Value::Bool(
                    self.voices
                        .iter()
                        .any(|v| v.name == name && v.source.is_some()),
                ))
            }
            25 => {
                return Ok(Value::Bool(
                    self.voices.iter().any(|v| v.music && v.source.is_some()),
                ))
            }
            76 => {
                let value = args.get(1).and_then(Value::to_f64).unwrap_or(0.0) as f32;
                self.settings.buses.entry(name.into()).or_default().volume = gain(value);
                self.gains();
            }
            77 => {
                return Ok(Value::F64(
                    self.settings.buses.get(name).map_or(0.0, |b| b.volume) as f64,
                ))
            }
            _ => return Err("Unknown audio operation".into()),
        }
        Ok(Value::I64(0))
    }
}
fn js_error(error: wasm_bindgen::JsValue) -> String {
    format!("Web Audio: {error:?}")
}

/// Shared manager survives scene loads while hosts receive one mutable adapter.
pub struct AudioAdapter(pub Rc<RefCell<WebAudio>>);
impl PlatformAudio for AudioAdapter {
    fn call(&mut self, id: u32, args: &[Value]) -> Result<Value, String> {
        self.0.borrow_mut().call(id, args)
    }
    fn settings(&self) -> AudioMixerSettings {
        self.0.borrow().settings.clone()
    }
    fn set_settings(&mut self, settings: AudioMixerSettings) {
        let mut audio = self.0.borrow_mut();
        audio.settings = settings;
        audio.gains();
    }
}
