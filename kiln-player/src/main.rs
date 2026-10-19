//! Kiln Player - thin launcher for standalone games
//! Loads a game from a packed archive and runs it

use anyhow::Result;
use clap::Parser;
use kiln_assets::Archive;
use kiln_core::{cache_dir, save_dir, settings_dir};
use log::info;
use std::path::PathBuf;

/// Kiln Player - run standalone games
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the game archive (.kar file)
    #[arg(short, long)]
    archive: PathBuf,

    /// Game name (used for save/settings folders)
    #[arg(short, long, default_value = "KilnGame")]
    game_name: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    info!("Loading game from: {}", args.archive.display());

    // Initialize platform-specific paths
    let save_path = save_dir(&args.game_name);
    let settings_path = settings_dir(&args.game_name);
    let cache_path = cache_dir(&args.game_name);

    info!("Save directory: {}", save_path.display());
    info!("Settings directory: {}", settings_path.display());
    info!("Cache directory: {}", cache_path.display());

    // Create directories if they don't exist
    std::fs::create_dir_all(&save_path)?;
    std::fs::create_dir_all(&settings_path)?;
    std::fs::create_dir_all(&cache_path)?;

    // Load the archive
    let archive = Archive::load(&args.archive)?;
    info!("Archive loaded with {} files", archive.list().len());

    // TODO: Initialize runtime, load assets, run game loop
    // This is a stub for M14; full implementation will follow

    Ok(())
}
