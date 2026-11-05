//! Decoded, polyphonic audio. Device output is opened explicitly, never by tests.

use kiln_core::{
    audio::{gain, AudioMixerSettings},
    project::AudioAsset,
};
use rodio::{
    buffer::SamplesBuffer,
    mixer::{mixer, Mixer, MixerSource},
    source::Zero,
    Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source,
};
use std::{collections::HashMap, io::Cursor};

/// Legacy category API; assets may route through any named bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCategory {
    Sfx,
    Music,
}

struct Clip {
    samples: SamplesBuffer,
    bus: String,
    volume: f32,
    music: bool,
    looped: bool,
}

struct Voice {
    name: String,
    bus: String,
    volume: f32,
    music: bool,
    player: Player,
}

/// Bounds simultaneous voices even when scripts trigger a sound every tick.
pub const MAX_AUDIO_VOICES: usize = 128;

/// A PCM mixer shared by native preview, gameplay and offline acceptance tests.
pub struct AudioEngine {
    mixer: Mixer,
    offline: Option<MixerSource>,
    output: Option<MixerDeviceSink>,
    clips: HashMap<String, Clip>,
    voices: Vec<Voice>,
    settings: AudioMixerSettings,
}

impl Default for AudioEngine {
    fn default() -> Self {
        let (mixer, source) = mixer(2.try_into().unwrap(), 48_000.try_into().unwrap());
        // Keep the mixer alive between sounds; an empty MixerSource terminates.
        mixer.add(Zero::new(2.try_into().unwrap(), 48_000.try_into().unwrap()));
        Self {
            mixer,
            offline: Some(source),
            output: None,
            clips: HashMap::new(),
            voices: Vec::new(),
            settings: AudioMixerSettings::default(),
        }
    }
}

impl AudioEngine {
    /// Create a device-independent mixer. Call open_output for audible playback.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::default())
    }

    /// Open output once; failure leaves decoding and offline mixing available.
    pub fn open_output(&mut self) -> Result<(), String> {
        if self.output.is_none() {
            let mut output =
                DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
            output.log_on_drop(false);
            if let Some(source) = self.offline.take() {
                output.mixer().add(source);
            }
            self.output = Some(output);
        }
        Ok(())
    }

    /// Decode bytes from disk or an archive. Failure leaves an existing clip intact.
    pub fn register_bytes(&mut self, asset: &AudioAsset, bytes: Vec<u8>) -> Result<(), String> {
        if asset.name.trim().is_empty() {
            return Err("Audio name is empty".into());
        }
        let decoder =
            Decoder::new(Cursor::new(bytes)).map_err(|error| format!("{}: {error}", asset.path))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let data: Vec<f32> = decoder.collect();
        if data.is_empty() || data.iter().any(|sample| !sample.is_finite()) {
            return Err(format!("{}: audio has no valid samples", asset.path));
        }
        let clip = Clip {
            samples: SamplesBuffer::new(channels, sample_rate, data),
            bus: asset.bus_name().into(),
            volume: gain(asset.volume),
            music: asset.is_music,
            looped: asset.is_music && asset.looped,
        };
        self.clips.insert(asset.name.clone(), clip);
        Ok(())
    }

    /// Register a file with the legacy API. Missing/invalid files are errors.
    pub fn register_sound(
        &mut self,
        name: &str,
        category: AudioCategory,
        path: &str,
    ) -> Result<(), String> {
        self.register_bytes(
            &AudioAsset {
                name: name.into(),
                path: path.into(),
                is_music: category == AudioCategory::Music,
                ..Default::default()
            },
            std::fs::read(path).map_err(|error| format!("{path}: {error}"))?,
        )
    }

    /// Apply all authored or player mixer controls, including already playing voices.
    pub fn set_settings(&mut self, mut settings: AudioMixerSettings) {
        settings.master_volume = gain(settings.master_volume);
        for bus in settings.buses.values_mut() {
            bus.volume = gain(bus.volume);
        }
        self.settings = settings;
        for voice in &self.voices {
            voice
                .player
                .set_volume(voice.volume * self.settings.bus_gain(&voice.bus));
        }
    }

    pub fn settings(&self) -> &AudioMixerSettings {
        &self.settings
    }

    /// A new voice for every trigger; repeated effects overlap.
    pub fn play_sound(&mut self, name: &str) {
        self.play_sound_with_volume(name, 1.0);
    }

    pub fn play_sound_with_volume(&mut self, name: &str, volume: f32) {
        self.start_voice(name, volume, false);
    }

    /// Replace previous music, using the asset's explicit loop setting.
    pub fn play_music(&mut self, name: &str) {
        if !self.clips.get(name).is_some_and(|clip| clip.music) {
            return;
        }
        self.stop_music();
        self.start_voice(name, 1.0, true);
    }

    fn start_voice(&mut self, name: &str, volume: f32, music: bool) {
        let Some(clip) = self.clips.get(name) else {
            return;
        };
        if clip.music != music {
            return;
        }
        self.voices.retain(|voice| !voice.player.empty());
        if self.voices.len() >= MAX_AUDIO_VOICES {
            let Some(index) = self.voices.iter().position(|voice| !voice.music) else {
                return;
            };
            self.voices.remove(index).player.stop();
        }
        let player = Player::connect_new(&self.mixer);
        let volume = gain(volume) * clip.volume;
        player.set_volume(volume * self.settings.bus_gain(&clip.bus));
        if clip.looped {
            player.append(clip.samples.clone().repeat_infinite());
        } else {
            player.append(clip.samples.clone());
        }
        self.voices.push(Voice {
            name: name.into(),
            bus: clip.bus.clone(),
            volume,
            music,
            player,
        });
    }

    pub fn stop_sound(&mut self, name: &str) {
        self.voices.retain(|voice| {
            if voice.name == name {
                voice.player.stop();
                false
            } else {
                true
            }
        });
    }
    pub fn pause_sound(&mut self, name: &str) {
        for voice in &self.voices {
            if voice.name == name {
                voice.player.pause();
            }
        }
    }
    pub fn resume_sound(&mut self, name: &str) {
        for voice in &self.voices {
            if voice.name == name {
                voice.player.play();
            }
        }
    }
    pub fn stop_music(&mut self) {
        self.voices.retain(|voice| {
            if voice.music {
                voice.player.stop();
                false
            } else {
                true
            }
        });
    }
    pub fn pause_music(&mut self) {
        for voice in &self.voices {
            if voice.music {
                voice.player.pause();
            }
        }
    }
    pub fn resume_music(&mut self) {
        for voice in &self.voices {
            if voice.music {
                voice.player.play();
            }
        }
    }
    pub fn stop_all(&mut self) {
        for voice in self.voices.drain(..) {
            voice.player.stop();
        }
    }
    pub fn is_playing(&self, name: &str) -> bool {
        self.voices
            .iter()
            .any(|voice| voice.name == name && !voice.player.empty() && !voice.player.is_paused())
    }
    pub fn is_music_playing(&self) -> bool {
        self.voices
            .iter()
            .any(|voice| voice.music && !voice.player.empty() && !voice.player.is_paused())
    }
    pub fn active_source_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| !voice.player.empty() && !voice.player.is_paused())
            .count()
    }
    pub fn registered_count(&self) -> usize {
        self.clips.len()
    }
    pub fn set_master_volume(&mut self, volume: f32) {
        let mut settings = self.settings.clone();
        settings.master_volume = gain(volume);
        self.set_settings(settings);
    }
    pub fn set_bus_volume(&mut self, name: &str, volume: f32) {
        if name == "Master" {
            self.set_master_volume(volume);
            return;
        }
        let mut settings = self.settings.clone();
        settings.buses.entry(name.into()).or_default().volume = gain(volume);
        self.set_settings(settings);
    }
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.set_bus_volume("SFX", volume);
    }
    pub fn set_music_volume(&mut self, volume: f32) {
        self.set_bus_volume("Music", volume);
    }
    pub fn master_volume(&self) -> f32 {
        gain(self.settings.master_volume)
    }
    pub fn sfx_volume(&self) -> f32 {
        self.settings
            .buses
            .get("SFX")
            .map_or(1.0, |bus| gain(bus.volume))
    }
    pub fn music_volume(&self) -> f32 {
        self.settings
            .buses
            .get("Music")
            .map_or(1.0, |bus| gain(bus.volume))
    }

    /// Advance headless playback so unavailable devices cannot accumulate voices
    /// or pending mixer sources forever. Device output advances itself.
    pub fn advance_silent(&mut self, seconds: f64) {
        if seconds.is_finite() && seconds > 0.0 {
            if let Some(source) = &mut self.offline {
                for _ in 0..(seconds.min(1.0) * 96_000.0) as usize {
                    let _ = source.next();
                }
            }
            self.voices.retain(|voice| !voice.player.empty());
        }
    }

    /// Pull real mixed samples without a device. Unavailable after open_output.
    pub fn render_samples(&mut self, count: usize) -> Result<Vec<f32>, String> {
        let source = self
            .offline
            .as_mut()
            .ok_or("Audio is running on a device")?;
        Ok(source.by_ref().take(count).collect())
    }
}

#[cfg(test)]
mod tests;
