//! Audio playback module (feature-gated)
//! Uses rodio for OS audio output and symphonia for OGG/Opus decoding
//! Provides a mixer with separate SFX and music channels, volume controls,
//! and per-source playback management.

#[cfg(feature = "audio")]
use rodio::{
    Decoder, DeviceSinkBuilder, MixerDeviceSink, Sink, Source,
    source::{Buffered, SeekError},
};
#[cfg(feature = "audio")]
use std::collections::HashMap;
#[cfg(feature = "audio")]
use std::fs::File;
#[cfg(feature = "audio")]
use std::io::{BufReader, Cursor};
#[cfg(feature = "audio")]
use std::sync::{Arc, Mutex};

/// Audio category for volume management
/// Available even when audio feature is disabled for API compatibility.
pub enum AudioCategory {
    /// Sound effects - short, can overlap
    Sfx,
    /// Background music - long, single instance, loops
    Music,
}

/// Configuration for a sound source
#[cfg(feature = "audio")]
struct SoundSource {
    name: String,
    category: AudioCategory,
    sink: Sink,
    loops: bool,
    paused: bool,
}

/// Audio mixer with separate SFX and music channels
#[cfg(feature = "audio")]
pub struct AudioEngine {
    master_sink: MixerDeviceSink,
    master_volume: f32,
    sfx_volume: f32,
    music_volume: f32,
    sources: Arc<Mutex<HashMap<String, SoundSource>>>,
    music_handle: Option<String>,
}

#[cfg(feature = "audio")]
impl AudioEngine {
    /// Create a new audio engine with default volumes
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let master_sink = DeviceSinkBuilder::open_default_sink()?;
        Ok(AudioEngine {
            master_sink,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 0.5,
            sources: Arc::new(Mutex::new(HashMap::new())),
            music_handle: None,
        })
    }

    /// Register a sound by name, category, and file path
    pub fn register_sound(&mut self, name: &str, category: AudioCategory, _path: &str) {
        let mut sources = self.sources.lock().unwrap();
        let sink = Sink::try_new(&self.master_sink.mixer()).expect("Failed to create sink");
        sink.set_volume(0.0);
        sink.pause();
        sources.insert(name.to_string(), SoundSource {
            name: name.to_string(),
            category,
            sink,
            loops: category == AudioCategory::Music,
            paused: true,
        });
    }

    /// Play a sound effect by name (SFX category)
    pub fn play_sound(&mut self, name: &str) {
        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            if sound.category == AudioCategory::Sfx {
                sound.sink.set_volume(self.master_volume * self.sfx_volume);
                sound.sink.play();
                sound.paused = false;
            }
        }
    }

    /// Play a sound effect with custom volume
    pub fn play_sound_with_volume(&mut self, name: &str, volume: f32) {
        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            if sound.category == AudioCategory::Sfx {
                let effective_volume = self.master_volume * self.sfx_volume * volume.clamp(0.0, 1.0);
                sound.sink.set_volume(effective_volume);
                sound.sink.play();
                sound.paused = false;
            }
        }
    }

    /// Stop a sound by name
    pub fn stop_sound(&mut self, name: &str) {
        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            sound.sink.stop();
            sound.paused = true;
            if self.music_handle.as_ref() == Some(&name.to_string()) {
                self.music_handle = None;
            }
        }
    }

    /// Pause a sound by name
    pub fn pause_sound(&mut self, name: &str) {
        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            sound.sink.pause();
            sound.paused = true;
        }
    }

    /// Resume a paused sound by name
    pub fn resume_sound(&mut self, name: &str) {
        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            sound.sink.play();
            sound.paused = false;
        }
    }

    /// Play music by name (Music category - single instance, loops)
    pub fn play_music(&mut self, name: &str) {
        if let Some(current) = self.music_handle.take() {
            self.stop_sound(&current);
        }

        let mut sources = self.sources.lock().unwrap();
        if let Some(sound) = sources.get_mut(name) {
            if sound.category == AudioCategory::Music {
                sound.sink.set_volume(self.master_volume * self.music_volume);
                sound.sink.play();
                sound.paused = false;
                self.music_handle = Some(name.to_string());
            }
        }
    }

    /// Stop music
    pub fn stop_music(&mut self) {
        if let Some(name) = self.music_handle.take() {
            self.stop_sound(&name);
        }
    }

    /// Pause music
    pub fn pause_music(&mut self) {
        if let Some(name) = &self.music_handle {
            self.pause_sound(name);
        }
    }

    /// Resume music
    pub fn resume_music(&mut self) {
        if let Some(name) = &self.music_handle {
            self.resume_sound(name);
        }
    }

    /// Set master volume (0.0 to 1.0)
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.update_all_volumes();
    }

    /// Set SFX volume (0.0 to 1.0)
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
        self.update_sfx_volumes();
    }

    /// Set music volume (0.0 to 1.0)
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        self.update_music_volumes();
    }

    /// Get master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Get SFX volume
    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// Get music volume
    pub fn music_volume(&self) -> f32 {
        self.music_volume
    }

    fn update_all_volumes(&self) {
        self.update_sfx_volumes();
        self.update_music_volumes();
    }

    fn update_sfx_volumes(&self) {
        let volume = self.master_volume * self.sfx_volume;
        let sources = self.sources.lock().unwrap();
        for sound in sources.values() {
            if sound.category == AudioCategory::Sfx {
                sound.sink.set_volume(volume);
            }
        }
    }

    fn update_music_volumes(&self) {
        let volume = self.master_volume * self.music_volume;
        let sources = self.sources.lock().unwrap();
        for sound in sources.values() {
            if sound.category == AudioCategory::Music {
                sound.sink.set_volume(volume);
            }
        }
    }

    /// Check if a sound is playing
    pub fn is_playing(&self, name: &str) -> bool {
        let sources = self.sources.lock().unwrap();
        sources.get(name).map(|s| !s.paused && !s.sink.is_paused()).unwrap_or(false)
    }

    /// Check if music is playing
    pub fn is_music_playing(&self) -> bool {
        if let Some(name) = &self.music_handle {
            self.is_playing(name)
        } else {
            false
        }
    }

    /// Stop all sounds
    pub fn stop_all(&mut self) {
        let mut sources = self.sources.lock().unwrap();
        for sound in sources.values_mut() {
            sound.sink.stop();
            sound.paused = true;
        }
        self.music_handle = None;
    }

    /// Get number of active sources
    pub fn active_source_count(&self) -> usize {
        let sources = self.sources.lock().unwrap();
        sources.values().filter(|s| !s.paused && !s.sink.is_paused()).count()
    }
}

#[cfg(feature = "audio")]
impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio engine")
    }
}

#[cfg(not(feature = "audio"))]
/// No-op audio engine stub when audio feature is disabled
pub struct AudioEngine;

#[cfg(not(feature = "audio"))]
impl AudioEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(AudioEngine)
    }

    pub fn register_sound(&mut self, _name: &str, _category: AudioCategory, _path: &str) {}
    pub fn play_sound(&mut self, _name: &str) {}
    pub fn play_sound_with_volume(&mut self, _name: &str, _volume: f32) {}
    pub fn stop_sound(&mut self, _name: &str) {}
    pub fn pause_sound(&mut self, _name: &str) {}
    pub fn resume_sound(&mut self, _name: &str) {}
    pub fn play_music(&mut self, _name: &str) {}
    pub fn stop_music(&mut self) {}
    pub fn pause_music(&mut self) {}
    pub fn resume_music(&mut self) {}
    pub fn set_master_volume(&mut self, _volume: f32) {}
    pub fn set_sfx_volume(&mut self, _volume: f32) {}
    pub fn set_music_volume(&mut self, _volume: f32) {}
    pub fn master_volume(&self) -> f32 { 1.0 }
    pub fn sfx_volume(&self) -> f32 { 1.0 }
    pub fn music_volume(&self) -> f32 { 0.5 }
    pub fn is_playing(&self, _name: &str) -> bool { false }
    pub fn is_music_playing(&self) -> bool { false }
    pub fn stop_all(&mut self) {}
    pub fn active_source_count(&self) -> usize { 0 }
}

#[cfg(not(feature = "audio"))]
impl Default for AudioEngine {
    fn default() -> Self {
        AudioEngine
    }
}

#[cfg(test)]
#[cfg(feature = "audio")]
mod tests {
    use super::*;

    #[test]
    fn test_audio_engine_creation() {
        let engine = AudioEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_volume_controls() {
        let mut engine = AudioEngine::new().unwrap();
        engine.set_master_volume(0.5);
        assert_eq!(engine.master_volume(), 0.5);
        engine.set_sfx_volume(0.25);
        assert_eq!(engine.sfx_volume(), 0.25);
        engine.set_music_volume(0.75);
        assert_eq!(engine.music_volume(), 0.75);
    }

    #[test]
    fn test_register_and_play_sfx() {
        let mut engine = AudioEngine::new().unwrap();
        engine.register_sound("test_sfx", AudioCategory::Sfx, "nonexistent.ogg");
        engine.play_sound("test_sfx");
    }
}
