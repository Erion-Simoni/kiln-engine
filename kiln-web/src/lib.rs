//! Kiln web runtime
//! WASM export shell for browser

mod threading;

use wasm_bindgen::prelude::*;

pub use threading::{can_use_web_workers, is_cross_origin_isolated, run_background_job};

// Initialize console error panic hook for better error messages in browser
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Placeholder for web runtime entry point
/// Full game loop and wgpu integration will be added in future milestones
#[wasm_bindgen]
pub fn run() {
    let isolated = is_cross_origin_isolated();
    let workers = can_use_web_workers();
    web_sys::console::log_1(
        &format!(
            "Kiln web runtime: crossOriginIsolated={isolated}, workers={workers}"
        )
        .into(),
    );
    let _ = run_background_job(1000);
}
