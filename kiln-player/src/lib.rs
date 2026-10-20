//! Kiln Player library
//! Used by desktop binary, Android cdylib, and iOS staticlib

pub mod loader;
pub mod spawn;
pub mod textures;

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub use android::android_main;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "ios")]
pub use ios::start_winit_app;
