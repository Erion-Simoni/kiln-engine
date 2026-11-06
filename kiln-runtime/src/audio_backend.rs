//! Platform audio adapter for hosts such as Web Audio that own device handles.
use kiln_core::audio::AudioMixerSettings;
use kiln_ir::Value;

pub trait PlatformAudio {
    /// Execute the shared audio builtin IDs; native hosts use AudioEngine directly.
    fn call(&mut self, id: u32, args: &[Value]) -> Result<Value, String>;
    fn settings(&self) -> AudioMixerSettings;
    fn set_settings(&mut self, settings: AudioMixerSettings);
}
#[cfg(target_arch = "wasm32")]
pub type AudioBackend = Box<dyn PlatformAudio>;
#[cfg(not(target_arch = "wasm32"))]
pub type AudioBackend = Box<dyn PlatformAudio + Send + Sync>;
