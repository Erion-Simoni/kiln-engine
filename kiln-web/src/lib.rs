//! Kiln web runtime — WASM export for browser.
//! Stub implementation — surface creation needs wgpu wasm32 support.

mod threading;

use wasm_bindgen::prelude::*;

pub use threading::{can_use_web_workers, is_cross_origin_isolated, run_background_job};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Entry point called from JavaScript.
/// Currently a stub — wgpu surface creation from HTML canvas on wasm32
/// needs platform-specific setup not yet implemented.
#[wasm_bindgen]
pub async fn run(_canvas_id: &str, _project_path: &str) {
    web_sys::console::log_1(
        &"Kiln web runtime: surface creation stub — implement wgpu wasm32 surface creation to enable web export"
            .into(),
    );
}
