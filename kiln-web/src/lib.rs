//! Kiln web runtime
//! WASM export shell for browser

use wasm_bindgen::prelude::*;

// Initialize console error panic hook for better error messages in browser
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Placeholder for web runtime entry point
/// Full game loop and wgpu integration will be added in future milestones
#[wasm_bindgen]
pub fn run() {
    // Placeholder for web runtime
}
