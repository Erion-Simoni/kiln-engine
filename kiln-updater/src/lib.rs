//! Kiln updater: delta patching, game updates, and DLC support.
//!
//! This crate is behind the `updater` feature flag. Store builds (itch.io, Steam)
//! should disable this feature and rely on storefront updaters.
//!
//! # Example
//!
//! ```no_run
//! use kiln_updater::Updater;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let updater = Updater::new("https://example.com/updates", "1.0.0")?;
//! if updater.check_update().await? {
//!     updater.apply_update().await?;
//! }
//! # Ok(())
//! # }
//! ```

pub mod crash;
pub mod dlc;
pub mod patch;
pub mod updater;
pub mod version;

pub use patch::{apply_patch, generate_patch};
pub use updater::Updater;
pub use version::Version;
