//! Audio sample: OGG/Opus playback demo
//! Run with: cargo run --bin audio-sample

use kiln_runtime::AudioEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kiln Audio Sample");
    println!("==================");
    println!();

    let _engine = AudioEngine::new()?;
    println!("Audio engine initialized successfully");

    println!();
    println!("To test audio playback:");
    println!("1. Place an OGG/Opus file in this directory (e.g., test.ogg)");
    println!("2. Uncomment the play_file line below");
    println!("3. Run this sample again");

    // engine.play_file("test.ogg")?;

    Ok(())
}
