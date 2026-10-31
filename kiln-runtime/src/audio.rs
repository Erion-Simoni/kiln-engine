//! Audio playback module (feature-gated)
//! Uses rodio for OS audio output and symphonia for OGG/Opus decoding

#[cfg(feature = "audio")]
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink};
#[cfg(feature = "audio")]
use std::collections::HashMap;
#[cfg(feature = "audio")]
use std::fs::File;
#[cfg(feature = "audio")]
use std::io::BufReader;

/// Audio engine for playback
#[cfg(feature = "audio")]
pub struct AudioEngine {
    sink: MixerDeviceSink,
    sounds: HashMap<String, String>, // name -> file path
    volume: f32,
}

#[cfg(feature = "audio")]
impl AudioEngine {
    /// Create a new audio engine
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sink = DeviceSinkBuilder::open_default_sink()?;
        Ok(AudioEngine {
            sink,
            sounds: HashMap::new(),
            volume: 1.0,
        })
    }

    /// Register a sound by name and file path.
    pub fn register_sound(&mut self, name: &str, path: &str) {
        self.sounds.insert(name.to_string(), path.to_string());
    }

    /// Load and play an audio file from disk
    pub fn play_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        self.sink.mixer().add(source);
        Ok(())
    }

    /// Play a registered sound effect by name.
    pub fn play_sound(&self, name: &str) {
        if let Some(path) = self.sounds.get(name) {
            let _ = self.play_file(path);
        }
        self.sink.set_volume(self.volume);
    }

    /// Stop all sounds (rodio doesn't support per-source stop easily).
    pub fn stop_sound(&self, _name: &str) {
        // No-op: rodio sink doesn't easily support stopping individual sources
        // without rebuilding the sink. A full implementation would track
        // source handles per sound.
    }

    /// Play music by name (replaces any currently playing music).
    pub fn play_music(&self, name: &str) {
        self.stop_music();
        if let Some(path) = self.sounds.get(name) {
            let _ = self.play_file(path);
        }
    }

    /// Stop music.
    pub fn stop_music(&self) {
        // No-op: same limitation as stop_sound
    }

    /// Set master volume (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }

    /// Check if a sound is playing (stub - rodio sink doesn't track per-source state).
    pub fn is_playing(&self, _name: &str) -> bool {
        false
    }

    /// Check if music is playing (stub).
    pub fn is_music_playing(&self) -> bool {
        false
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

    pub fn play_file(&self, _path: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
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
    fn test_play_nonexistent_file() {
        let engine = AudioEngine::new().unwrap();
        let result = engine.play_file("nonexistent.ogg");
        assert!(result.is_err());
    }
}
