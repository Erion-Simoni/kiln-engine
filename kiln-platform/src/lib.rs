//! Platform services abstraction for storefront integration
//!
//! This crate provides a trait-based abstraction for storefront-specific features
//! like achievements, leaderboards, cloud saves, and in-app purchases. This allows
//! games to be written without being locked to a specific storefront.

/// Platform services trait for storefront integration
///
/// Implementations provide access to storefront-specific features like
/// achievements, leaderboards, cloud saves, and in-app purchases.
pub trait PlatformServices {
    /// Check if the platform is initialized and ready
    fn is_initialized(&self) -> bool;

    /// Get the current user's display name
    fn get_user_name(&self) -> Option<String>;

    /// Get the current user's unique ID
    fn get_user_id(&self) -> Option<String>;

    /// Achievement support
    fn unlock_achievement(&self, achievement_id: &str) -> Result<(), PlatformError>;
    fn get_achievement_progress(&self, achievement_id: &str) -> Result<Option<f32>, PlatformError>;
    fn set_achievement_progress(&self, achievement_id: &str, progress: f32) -> Result<(), PlatformError>;

    /// Leaderboard support
    fn submit_score(&self, leaderboard_id: &str, score: i64) -> Result<(), PlatformError>;
    fn get_leaderboard(&self, leaderboard_id: &str) -> Result<Vec<LeaderboardEntry>, PlatformError>;

    /// Cloud save support
    fn save_cloud_data(&self, slot: &str, data: &[u8]) -> Result<(), PlatformError>;
    fn load_cloud_data(&self, slot: &str) -> Result<Option<Vec<u8>>, PlatformError>;
    fn delete_cloud_save(&self, slot: &str) -> Result<(), PlatformError>;

    /// In-app purchase support
    fn get_products(&self) -> Result<Vec<ProductInfo>, PlatformError>;
    fn purchase_product(&self, product_id: &str) -> Result<PurchaseResult, PlatformError>;
    fn restore_purchases(&self) -> Result<Vec<PurchaseResult>, PlatformError>;
}

/// Errors that can occur when using platform services
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    NotInitialized,
    FeatureNotSupported,
    NetworkError(String),
    InvalidInput(String),
    StoreError(String),
    AuthenticationFailed,
}

/// Leaderboard entry
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub user_name: String,
    pub score: i64,
    pub rank: Option<i32>,
}

/// Product information for in-app purchases
#[derive(Debug, Clone)]
pub struct ProductInfo {
    pub product_id: String,
    pub name: String,
    pub description: String,
    pub price: String,
    pub currency: String,
}

/// Result of a purchase operation
#[derive(Debug, Clone)]
pub struct PurchaseResult {
    pub product_id: String,
    pub success: bool,
    pub receipt: Option<String>,
}

/// Stub implementation for games without storefront integration
///
/// This implementation does nothing and returns appropriate errors,
/// allowing games to run without any storefront connected.
pub struct StubPlatformServices;

impl PlatformServices for StubPlatformServices {
    fn is_initialized(&self) -> bool {
        false
    }

    fn get_user_name(&self) -> Option<String> {
        None
    }

    fn get_user_id(&self) -> Option<String> {
        None
    }

    fn unlock_achievement(&self, _achievement_id: &str) -> Result<(), PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn get_achievement_progress(&self, _achievement_id: &str) -> Result<Option<f32>, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn set_achievement_progress(&self, _achievement_id: &str, _progress: f32) -> Result<(), PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn submit_score(&self, _leaderboard_id: &str, _score: i64) -> Result<(), PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn get_leaderboard(&self, _leaderboard_id: &str) -> Result<Vec<LeaderboardEntry>, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn save_cloud_data(&self, _slot: &str, _data: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn load_cloud_data(&self, _slot: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn delete_cloud_save(&self, _slot: &str) -> Result<(), PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn get_products(&self) -> Result<Vec<ProductInfo>, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn purchase_product(&self, _product_id: &str) -> Result<PurchaseResult, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }

    fn restore_purchases(&self) -> Result<Vec<PurchaseResult>, PlatformError> {
        Err(PlatformError::FeatureNotSupported)
    }
}

/// Steamworks integration (stub implementation)
///
/// When the `steam` feature is enabled, this provides a placeholder for
/// Steamworks SDK integration. The actual Steamworks SDK is not included
/// in this crate; this stub demonstrates the API shape.
#[cfg(feature = "steam")]
pub struct SteamPlatformServices;

#[cfg(feature = "steam")]
impl PlatformServices for SteamPlatformServices {
    fn is_initialized(&self) -> bool {
        // TODO: Initialize Steamworks API
        false
    }

    fn get_user_name(&self) -> Option<String> {
        // TODO: Call SteamAPI::getUserName()
        None
    }

    fn get_user_id(&self) -> Option<String> {
        // TODO: Call SteamAPI::getSteamID()
        None
    }

    fn unlock_achievement(&self, achievement_id: &str) -> Result<(), PlatformError> {
        // TODO: Call SteamUserStats::SetAchievement()
        println!("Steam: Unlock achievement {}", achievement_id);
        Ok(())
    }

    fn get_achievement_progress(&self, achievement_id: &str) -> Result<Option<f32>, PlatformError> {
        // TODO: Call SteamUserStats::GetAchievement()
        println!("Steam: Get achievement progress {}", achievement_id);
        Ok(None)
    }

    fn set_achievement_progress(&self, achievement_id: &str, progress: f32) -> Result<(), PlatformError> {
        // TODO: Call SteamUserStats::SetAchievement() with progress
        println!("Steam: Set achievement progress {} to {}", achievement_id, progress);
        Ok(())
    }

    fn submit_score(&self, leaderboard_id: &str, score: i64) -> Result<(), PlatformError> {
        // TODO: Call SteamUserStats::UploadLeaderboardScore()
        println!("Steam: Submit score {} to leaderboard {}", score, leaderboard_id);
        Ok(())
    }

    fn get_leaderboard(&self, leaderboard_id: &str) -> Result<Vec<LeaderboardEntry>, PlatformError> {
        // TODO: Call SteamUserStats::DownloadLeaderboardEntries()
        println!("Steam: Get leaderboard {}", leaderboard_id);
        Ok(vec![])
    }

    fn save_cloud_data(&self, slot: &str, data: &[u8]) -> Result<(), PlatformError> {
        // TODO: Call SteamRemoteStorage::FileWrite()
        println!("Steam: Save cloud data to slot {} ({} bytes)", slot, data.len());
        Ok(())
    }

    fn load_cloud_data(&self, slot: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        // TODO: Call SteamRemoteStorage::FileRead()
        println!("Steam: Load cloud data from slot {}", slot);
        Ok(None)
    }

    fn delete_cloud_save(&self, slot: &str) -> Result<(), PlatformError> {
        // TODO: Call SteamRemoteStorage::FileDelete()
        println!("Steam: Delete cloud save slot {}", slot);
        Ok(())
    }

    fn get_products(&self) -> Result<Vec<ProductInfo>, PlatformError> {
        // TODO: Steam doesn't have IAP like mobile stores; this is for DLC
        println!("Steam: Get products (DLC)");
        Ok(vec![])
    }

    fn purchase_product(&self, product_id: &str) -> Result<PurchaseResult, PlatformError> {
        // TODO: Call SteamApps::RequestAppBetaPass() or DLC purchase
        println!("Steam: Purchase product {}", product_id);
        Ok(PurchaseResult {
            product_id: product_id.to_string(),
            success: false,
            receipt: None,
        })
    }

    fn restore_purchases(&self) -> Result<Vec<PurchaseResult>, PlatformError> {
        // TODO: Steam doesn't need restore; ownership is tied to account
        println!("Steam: Restore purchases (not needed on Steam)");
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_not_initialized() {
        let stub = StubPlatformServices;
        assert!(!stub.is_initialized());
    }

    #[test]
    fn stub_returns_none_for_user_info() {
        let stub = StubPlatformServices;
        assert!(stub.get_user_name().is_none());
        assert!(stub.get_user_id().is_none());
    }

    #[test]
    fn stub_returns_feature_not_supported() {
        let stub = StubPlatformServices;
        assert_eq!(
            stub.unlock_achievement("test"),
            Err(PlatformError::FeatureNotSupported)
        );
        assert_eq!(
            stub.submit_score("leaderboard", 100),
            Err(PlatformError::FeatureNotSupported)
        );
    }

    #[cfg(feature = "steam")]
    #[test]
    fn steam_stub_returns_ok_for_achievements() {
        let steam = SteamPlatformServices;
        assert!(steam.unlock_achievement("test").is_ok());
    }
}
