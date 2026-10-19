//! Kiln Player library
//! Used by both desktop binary and Android cdylib

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::android_main;
