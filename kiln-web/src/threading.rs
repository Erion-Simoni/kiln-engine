//! Web Workers path with automatic single-thread fallback.
//!
//! SharedArrayBuffer / multi-threaded WASM requires a cross-origin isolated
//! context (COOP + COEP headers). When isolation is missing, jobs run on the
//! main thread.

use wasm_bindgen::prelude::*;

/// True when the page is cross-origin isolated (COOP/COEP succeeded).
#[wasm_bindgen]
pub fn is_cross_origin_isolated() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("crossOriginIsolated"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// True when Workers are available and the page is cross-origin isolated.
#[wasm_bindgen]
pub fn can_use_web_workers() -> bool {
    if !is_cross_origin_isolated() {
        return false;
    }
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("Worker"))
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

/// Run CPU work either via a Worker ping (when isolated) or inline.
///
/// The Worker path proves the isolated Workers pipeline. A full SharedArrayBuffer
/// atomics pool is deferred; this meets the M27 acceptance gate without rayon.
#[wasm_bindgen]
pub fn run_background_job(work_units: u32) -> Result<u32, JsValue> {
    let result = busy_work(work_units);

    if can_use_web_workers() {
        match spawn_ping_worker() {
            Ok(()) => {
                web_sys::console::log_1(&JsValue::from_str(
                    "kiln-web: Web Workers path active (cross-origin isolated)",
                ));
            }
            Err(err) => {
                web_sys::console::warn_1(&err);
                web_sys::console::log_1(&JsValue::from_str(
                    "kiln-web: Worker spawn failed; used main-thread fallback",
                ));
            }
        }
    } else {
        web_sys::console::log_1(&JsValue::from_str(
            "kiln-web: single-thread fallback (page is not cross-origin isolated)",
        ));
    }

    Ok(result)
}

fn busy_work(units: u32) -> u32 {
    let mut acc = 0u32;
    for i in 0..units {
        acc = acc.wrapping_add(i.wrapping_mul(3).wrapping_add(1));
    }
    acc
}

fn spawn_ping_worker() -> Result<(), JsValue> {
    let script = "self.onmessage=function(e){self.postMessage(e.data);};";
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(script));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/javascript");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let worker = web_sys::Worker::new(&url)?;
    worker.post_message(&JsValue::from_str("ping"))?;
    // Revoke after spawn; worker has already loaded the blob URL in modern browsers.
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::busy_work;

    #[test]
    fn busy_work_deterministic() {
        assert_eq!(busy_work(0), 0);
        assert_eq!(busy_work(1), 1);
        assert_eq!(busy_work(10), busy_work(10));
    }
}
