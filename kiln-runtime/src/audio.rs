//! Audio playback module (feature-gated)
//! Uses rodio for OS audio output and symphonia for OGG/Opus decoding

#[cfg(feature = "audio")]
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink};
#[cfg(feature = "audio")]
use std::fs::File;
#[cfg(feature = "audio")]
use std::io::BufReader;

/// Audio engine for playback
#[cfg(feature = "audio")]
pub struct AudioEngine {
    _sink_handle: MixerDeviceSink,
}

#[cfg(feature = "audio")]
impl AudioEngine {
    /// Create a new audio engine
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sink_handle = DeviceSinkBuilder::open_default_sink()?;
        Ok(AudioEngine {
            _sink_handle: sink_handle,
        })
    }

    /// Load and play an audio file from disk
    pub fn play_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        self._sink_handle.mixer().add(source);
        Ok(())
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
