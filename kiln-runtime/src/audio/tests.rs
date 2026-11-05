use super::*;

// One second of real RIFF PCM at the mixer's rate, not a mock decoder.
fn wav() -> Vec<u8> {
    let data: Vec<u8> = (0..48_000).flat_map(|_| 8192_i16.to_le_bytes()).collect();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend(96_000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes
}
fn asset(name: &str, music: bool) -> AudioAsset {
    AudioAsset {
        name: name.into(),
        path: "tone.wav".into(),
        is_music: music,
        ..Default::default()
    }
}
fn tail(engine: &mut AudioEngine) -> f32 {
    let samples = engine.render_samples(4800).unwrap();
    *samples.last().unwrap()
}
#[test]
fn decoding_overlap_bus_gain_and_live_mute() {
    let mut engine = AudioEngine::default();
    let mut effect = asset("hit", false);
    effect.bus = "Combat".into();
    effect.volume = 0.5;
    engine.register_bytes(&effect, wav()).unwrap();
    engine.play_sound("hit");
    engine.play_sound("hit");
    assert_eq!(engine.active_source_count(), 2);
    assert!((tail(&mut engine) - 0.25).abs() < 0.001);
    let mut settings = engine.settings().clone();
    settings.master_volume = 0.5;
    settings.buses.insert(
        "Combat".into(),
        kiln_core::audio::AudioBus {
            volume: 0.5,
            ..Default::default()
        },
    );
    engine.set_settings(settings.clone());
    assert!((tail(&mut engine) - 0.0625).abs() < 0.001);
    settings.buses.get_mut("Combat").unwrap().muted = true;
    engine.set_settings(settings);
    assert_eq!(tail(&mut engine), 0.0);
}
#[test]
fn loops_replacement_pause_resume_and_stop_replay() {
    let mut engine = AudioEngine::default();
    engine.register_bytes(&asset("a", true), wav()).unwrap();
    let mut other = asset("b", true);
    other.looped = false;
    engine.register_bytes(&other, wav()).unwrap();
    engine.play_music("a");
    engine.render_samples(192_000).unwrap(); // Twice the clip length.
    assert!(engine.is_music_playing());
    assert!((tail(&mut engine) - 0.25).abs() < 0.001);
    engine.pause_music();
    assert_eq!(tail(&mut engine), 0.0);
    engine.resume_music();
    assert!((tail(&mut engine) - 0.25).abs() < 0.001);
    engine.play_music("b");
    assert!(!engine.is_playing("a"));
    engine.render_samples(100_000).unwrap();
    assert!(!engine.is_music_playing());
    engine.play_music("a");
    engine.stop_all();
    assert_eq!(tail(&mut engine), 0.0);
    engine.play_music("a");
    assert!((tail(&mut engine) - 0.25).abs() < 0.001);
}
#[test]
fn invalid_resources_fail_and_do_not_replace_a_decoded_clip() {
    let mut engine = AudioEngine::default();
    engine.register_bytes(&asset("hit", false), wav()).unwrap();
    assert!(engine
        .register_bytes(&asset("hit", false), b"broken".to_vec())
        .is_err());
    assert!(engine
        .register_sound("missing", AudioCategory::Sfx, "/missing.wav")
        .is_err());
    assert_eq!(engine.registered_count(), 1);
    engine.play_sound("hit");
    assert!((tail(&mut engine) - 0.25).abs() < 0.001);
}
#[test]
fn bounded_polyphony_and_solo_routing() {
    let mut engine = AudioEngine::default();
    engine.register_bytes(&asset("hit", false), wav()).unwrap();
    for _ in 0..200 {
        engine.play_sound("hit");
    }
    assert_eq!(engine.active_source_count(), MAX_AUDIO_VOICES);
    let mut settings = engine.settings().clone();
    settings.buses.get_mut("Music").unwrap().solo = true;
    engine.set_settings(settings);
    assert_eq!(tail(&mut engine), 0.0);
    engine.stop_sound("hit");
    assert_eq!(engine.active_source_count(), 0);
    engine.set_master_volume(f32::NAN);
    assert_eq!(engine.master_volume(), 0.0);
}

#[test]
fn headless_effects_complete_and_malformed_gains_are_safe_to_persist() {
    let mut engine = AudioEngine::default();
    engine.register_bytes(&asset("hit", false), wav()).unwrap();
    let mut settings = engine.settings().clone();
    settings.master_volume = f32::NAN;
    settings.buses.get_mut("SFX").unwrap().volume = f32::INFINITY;
    engine.set_settings(settings);
    let json = serde_json::to_string(engine.settings()).unwrap();
    assert!(serde_json::from_str::<AudioMixerSettings>(&json).is_ok());
    engine.play_sound("hit");
    assert_eq!(tail(&mut engine), 0.0);
    engine.advance_silent(0.6);
    engine.advance_silent(0.6);
    assert!(!engine.is_playing("hit"));
    assert_eq!(engine.active_source_count(), 0);
}
