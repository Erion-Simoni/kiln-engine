# Clean Machine Testing Guide

This document describes how to test Kiln on a clean machine (without a Rust toolchain installed) to ensure the engine exports work correctly for end users.

## Overview

Kiln games are exported as standalone applications that should run on any machine without requiring a Rust toolchain or development environment. This document describes how to verify that your exported game works correctly on a clean machine.

## Prerequisites

- A built Kiln engine (cargo build --release)
- A built game project (cargo run --bin kiln-cli -- export)
- A clean machine (no Rust toolchain installed)
- The target platform (Windows, macOS, Linux, or Web)

## Testing Steps

### 1. Build the Game

Build your game for the target platform:

# Windows
cargo run --bin kiln-cli -- export --platform windows --output ./my-game-win

# macOS
cargo run --bin kiln-cli -- export --platform macos --output ./my-game-mac

# Linux
cargo run --bin kiln-cli -- export --platform linux --output ./my-game-linux

# Web
cargo run --bin kiln-cli -- export --platform web --output ./my-game-web

### 2. Transfer to Clean Machine

Transfer the exported game to a clean machine that does not have:
- Rust toolchain installed
- Cargo or rustc
- Any Kiln development dependencies
- Any Kiln engine source code

### 3. Run the Game

Double-click the game executable (or open the web export in a browser) and verify:

- The game launches without errors
- The game window opens correctly
- All assets load correctly
- All gameplay works correctly
- All scripts run correctly
- All audio plays correctly
- All post-processing effects work

### 4. Test Checklist

Verify the following:

- Game launches without errors
- Game window opens correctly
- All assets load correctly (textures, sounds, etc.)
- All gameplay works correctly
- All scripts run correctly
- All audio plays correctly
- All post-processing effects work
- Performance is acceptable (60 FPS target)
- Save/load works correctly
- Settings are saved correctly

### 5. Troubleshooting

#### Game Won't Launch

- Check that all required DLLs/shared libraries are included
- Check that all assets are included in the export
- Check system requirements (Windows 7+, macOS 10.12+, Linux)

#### Game Crashes

- Check for missing dependencies
- Check for missing assets
- Check system requirements

#### Audio Doesn't Play

- Check system audio settings
- Check that audio files are included in the export

#### Graphics Issues

- Check graphics driver
- Check that the machine meets minimum requirements
- Check that textures are included in the export

## Expected Directory Structure

A typical exported game directory contains:

my-game/
├── my-game.exe (or my-game.app on macOS)
├── assets/
│   ├── textures/
│   ├── audio/
│   └── ...
├── kiln.toml
└── ... (other project files)

## Notes

- The exported game is self-contained and does not require a Rust toolchain
- All assets are packaged with the game
- The game runs as a standalone application
- No installation is required (just run the executable)
