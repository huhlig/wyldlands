//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Welcome banner system for displaying messages to connecting clients
//!
//! Banners are stored in the settings table with keys:
//! - banner.welcome: Welcome banner shown on connection
//! - banner.motd: Message of the Day
//! - banner.login: Login screen
//! - banner.disconnect: Disconnect message

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

/// Banner type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BannerType {
    /// Welcome banner shown on connection
    Welcome,
    /// MOTD (Message of the Day)
    Motd,
    /// Login screen
    Login,
    /// Disconnect message
    Disconnect,
}

impl BannerType {
    /// Get the settings key for this banner type
    fn settings_key(&self) -> &'static str {
        match self {
            BannerType::Welcome => "banner.welcome",
            BannerType::Motd => "banner.motd",
            BannerType::Login => "banner.login",
            BannerType::Disconnect => "banner.disconnect",
        }
    }
}

/// Banner manager with caching
pub struct BannerManager {
    /// Database pool
    pool: PgPool,
    
    /// Cached banners
    cache: Arc<RwLock<BannerCache>>,
    
    /// Cache TTL
    cache_ttl: Duration,
}

/// Banner cache
struct BannerCache {
    /// Cached welcome banner
    welcome: Option<String>,
    
    /// Cached MOTD
    motd: Option<String>,
    
    /// Cached login banner
    login: Option<String>,
    
    /// Cached disconnect message
    disconnect: Option<String>,
    
    /// Last cache update time
    last_update: SystemTime,
}

impl BannerManager {
    /// Create a new banner manager
    pub fn new(pool: PgPool, cache_ttl_secs: u64) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(BannerCache {
                welcome: None,
                motd: None,
                login: None,
                disconnect: None,
                last_update: SystemTime::UNIX_EPOCH,
            })),
            cache_ttl: Duration::from_secs(cache_ttl_secs),
        }
    }
    
    /// Get welcome banner
    pub async fn get_welcome_banner(&self) -> Result<String, String> {
        self.get_banner(BannerType::Welcome).await
    }
    
    /// Get MOTD
    pub async fn get_motd(&self) -> Result<String, String> {
        self.get_banner(BannerType::Motd).await
    }
    
    /// Get login banner
    pub async fn get_login_banner(&self) -> Result<String, String> {
        self.get_banner(BannerType::Login).await
    }
    
    /// Get disconnect message
    pub async fn get_disconnect_message(&self) -> Result<String, String> {
        self.get_banner(BannerType::Disconnect).await
    }
    
    /// Get a banner by type
    async fn get_banner(&self, banner_type: BannerType) -> Result<String, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Ok(elapsed) = cache.last_update.elapsed() {
                if elapsed < self.cache_ttl {
                    // Cache is still valid
                    let cached = match banner_type {
                        BannerType::Welcome => &cache.welcome,
                        BannerType::Motd => &cache.motd,
                        BannerType::Login => &cache.login,
                        BannerType::Disconnect => &cache.disconnect,
                    };
                    
                    if let Some(content) = cached {
                        return Ok(content.clone());
                    }
                }
            }
        }
        
        // Cache miss or expired, load from database
        self.refresh_cache().await?;
        
        // Try again from cache
        let cache = self.cache.read().await;
        let cached = match banner_type {
            BannerType::Welcome => &cache.welcome,
            BannerType::Motd => &cache.motd,
            BannerType::Login => &cache.login,
            BannerType::Disconnect => &cache.disconnect,
        };
        
        cached.clone().ok_or_else(|| format!("No {:?} banner found", banner_type))
    }
    
    /// Refresh the banner cache from database settings table
    pub async fn refresh_cache(&self) -> Result<(), String> {
        tracing::debug!("Refreshing banner cache from settings table");
        
        // Update cache
        let mut cache = self.cache.write().await;
        
        // Load each banner type from settings
        cache.welcome = self.load_banner_from_settings(BannerType::Welcome).await.ok();
        cache.motd = self.load_banner_from_settings(BannerType::Motd).await.ok();
        cache.login = self.load_banner_from_settings(BannerType::Login).await.ok();
        cache.disconnect = self.load_banner_from_settings(BannerType::Disconnect).await.ok();
        
        cache.last_update = SystemTime::now();
        
        tracing::info!("Banner cache refreshed from settings");
        Ok(())
    }
    
    /// Load a banner from the settings table
    async fn load_banner_from_settings(&self, banner_type: BannerType) -> Result<String, String> {
        let key = banner_type.settings_key();
        
        let content: Option<String> = sqlx::query_scalar(
            "SELECT value FROM wyldlands.settings WHERE key = $1"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load banner from settings: {}", e))?;
        
        content.ok_or_else(|| format!("Banner not found in settings: {}", key))
    }
    
    /// Create or update a banner in settings table
    pub async fn upsert_banner(
        &self,
        banner_type: BannerType,
        content: String,
    ) -> Result<(), String> {
        let key = banner_type.settings_key();
        
        sqlx::query(
            "INSERT INTO wyldlands.settings (key, value, created_at, updated_at)
             VALUES ($1, $2, NOW(), NOW())
             ON CONFLICT (key)
             DO UPDATE SET
                value = EXCLUDED.value,
                updated_at = NOW()"
        )
        .bind(key)
        .bind(content)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to upsert banner: {}", e))?;
        
        // Invalidate cache
        self.refresh_cache().await?;
        
        Ok(())
    }
    
    /// Delete a banner from settings table
    pub async fn delete_banner(&self, banner_type: BannerType) -> Result<(), String> {
        let key = banner_type.settings_key();
        
        sqlx::query("DELETE FROM wyldlands.settings WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete banner: {}", e))?;
        
        // Invalidate cache
        self.refresh_cache().await?;
        
        Ok(())
    }
    
    /// Get a banner value directly from settings (bypassing cache)
    pub async fn get_banner_raw(&self, banner_type: BannerType) -> Result<Option<String>, String> {
        self.load_banner_from_settings(banner_type).await.map(Some).or(Ok(None))
    }
}

/// SQL migration for banner settings
pub const BANNER_MIGRATION: &str = r#"
-- Insert default welcome banner if none exists
INSERT INTO wyldlands.settings (key, value, created_at, updated_at)
VALUES ('banner.welcome',
'╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║              Welcome to Wyldlands MUD Server                 ║
║                                                              ║
║  A text-based multiplayer adventure game built with Rust    ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

', NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default MOTD if none exists
INSERT INTO wyldlands.settings (key, value, created_at, updated_at)
VALUES ('banner.motd',
'═══════════════════════════════════════════════════════════════
  Message of the Day
═══════════════════════════════════════════════════════════════

  • Server is running in BETA mode
  • Report bugs to the admin team
  • Have fun and be respectful!

═══════════════════════════════════════════════════════════════
', NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default login banner if none exists
INSERT INTO wyldlands.settings (key, value, created_at, updated_at)
VALUES ('banner.login',
'Please enter your username and password to continue.
', NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default disconnect message if none exists
INSERT INTO wyldlands.settings (key, value, created_at, updated_at)
VALUES ('banner.disconnect',
'Thank you for playing Wyldlands! Come back soon!
', NOW(), NOW())
ON CONFLICT (key) DO NOTHING;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_banner_type() {
        assert_eq!(BannerType::Welcome, BannerType::Welcome);
        assert_ne!(BannerType::Welcome, BannerType::Motd);
    }
}

