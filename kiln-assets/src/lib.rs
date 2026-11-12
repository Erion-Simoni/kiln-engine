//! Kiln asset pipeline — content-hash cache and packed archives.

pub mod archive;
pub mod atlas;
pub mod cache;
pub mod hash;
pub mod image;
pub mod settings;

pub use archive::{Archive, ArchiveEntry};
pub use cache::{AssetCache, ImportResult};
pub use hash::content_hash;
pub use image::{decode_png, encode_png_rgba, is_png, Image, ImageError};
pub use settings::{ImportSettings, TextureFilter};

#[cfg(not(target_arch = "wasm32"))]
pub mod web_bundle;

#[cfg(not(target_arch = "wasm32"))]
pub mod distribution;

#[cfg(not(target_arch = "wasm32"))]
pub mod runtime_notices;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_notices_tests;
