# Web Export Testing Guide

This document describes how to test the Kiln web export (WASM) to ensure it works correctly in a browser.

## Overview

The Kiln web export uses WASM + WebGPU/WebGL2 to run Kiln 2D games in the browser. The kiln-web crate provides the runtime that loads a project (kiln.toml + PNG assets), runs the fixed-timestep game loop with the KilnScript slice runtime, and renders 2D sprites through the engine's sprite batcher.

## Prerequisites

- A built Kiln engine binary (kiln-cli)
- A sample Kiln project with at least one scene and script
- A modern web browser with WebGL2 or WebGPU support
- A local web server (for testing locally)

## Testing Steps

### 1. Build the Web Export

Build the web runtime:

cargo build --release -p kiln-web

Build your game project:

cargo run --bin kiln-cli -- export --platform web --output ./web-export

### 2. Serve the Web Export

The web export must be served over HTTP (not opened as a file:// URL) due to browser security restrictions:

python3 -m http.server 8000 --directory web-export

Or use any other HTTP server.

### 3. Open in Browser

Navigate to http://localhost:8000 in your browser.

### 4. Test Checklist

Verify the following:

- Game loads without console errors
- Sprite rendering works (textures load correctly)
- Sprite batching works (multiple sprites render correctly)
- Tilemap rendering works (if using tilemaps)
- Keyboard input works (arrow keys, WASD)
- Mouse input works (click, mouse movement)
- Touch input works (on touch devices)
- Script execution works (KilnScript runs correctly)
- Audio plays (if using audio)
- Post-processing effects work (bloom, fog, outline)
- Performance is acceptable (60 FPS target)

### 5. Browser Compatibility

Test on:
- Chrome (latest)
- Firefox (latest)
- Safari (latest)
- Edge (latest)

### 6. Known Limitations

- WebGL2 fallback may have reduced performance compared to WebGPU
- Some post-processing effects may not work on older browsers
- Audio may be blocked by browser autoplay policies (requires user interaction first)

## Troubleshooting

### White Screen / Nothing Renders

- Check browser console for WebGL errors
- Ensure textures are loaded correctly
- Check that the project was exported correctly

### Input Not Working

- Check that the canvas has focus (click on it)
- Verify keyboard event listeners are attached
- Check browser console for errors

### Audio Not Playing

- Browser autoplay policy may block audio (requires user interaction first)
- Check browser console for audio context errors
