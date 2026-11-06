//! Audible device check: cargo run --manifest-path samples/audio/Cargo.toml -- tone.wav
use kiln_runtime::{AudioCategory, AudioEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Pass a WAV, OGG Vorbis, MP3 or FLAC file to play")?;
    let mut engine = AudioEngine::new()?;
    engine.register_sound("sample", AudioCategory::Sfx, &path)?;
    engine.open_output()?;
    engine.play_sound("sample");
    println!("Playing {path}. Press Ctrl+C to stop.");
    while engine.is_playing("sample") {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Ok(())
}
