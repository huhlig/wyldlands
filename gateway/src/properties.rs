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

//! Gateway Properties system for storing mutable configuration properties
//!
//! Properties are stored in the settings table with keys:
//! - banner.welcome: Welcome banner shown on connection
//! - banner.motd: Message of the Day
//! - banner.login: Login screen
//! - banner.logout.txt: Disconnect message
//! - defaults.admin_html
//! - defaults.admin_css
//! - defaults.admin_js
//! - defaults.client_html
//! - defaults.client_css
//! - defaults.client_js
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use wyldlands_common::gateway::GatewayProperty;

/// Banner manager with caching
pub struct PropertiesManager {
    /// RPC client for server communication
    rpc_client: Arc<crate::grpc::RpcClientManager>,

    /// Cached banners
    cache: Arc<RwLock<Cache>>,

    /// Cache TTL
    cache_ttl: Duration,
}

impl PropertiesManager {
    /// Create a new banner manager
    pub fn new(rpc_client: Arc<crate::grpc::RpcClientManager>, cache_ttl_secs: u64) -> Self {
        Self {
            rpc_client,
            cache: Arc::new(RwLock::new(Cache {
                properties: HashMap::default(),
                last_update: SystemTime::now(),
            })),
            cache_ttl: Duration::from_secs(cache_ttl_secs),
        }
    }

    /// Get a banner by type
    #[tracing::instrument(skip(self))]
    pub async fn get_property(&self, property: GatewayProperty) -> Result<String, String> {
        // 1. Try to get from cache if it's not stale
        {
            let cache = self.cache.read().await;
            if let Some((value, last_update)) = cache.properties.get(&property) {
                if let Ok(elapsed) = last_update.elapsed() {
                    if elapsed < self.cache_ttl {
                        return Ok(value.clone());
                    }
                }
            }
        }

        // 2. Cache miss or expired, refresh from server
        // Note: refresh_cached_properties updates the cache internally
        let _ = self.refresh_cached_properties(&[property]).await;

        // 3. Try to get from cache again after refresh
        {
            let cache = self.cache.read().await;
            if let Some((value, last_update)) = cache.properties.get(&property) {
                if let Ok(elapsed) = last_update.elapsed() {
                    if elapsed < self.cache_ttl {
                        return Ok(value.clone());
                    }
                }
            }
        }

        // 4. Fallback to default values
        Ok(match property {
            GatewayProperty::BannerWelcome => BANNER_WELCOME_DEFAULT.to_string(),
            GatewayProperty::BannerMotd => BANNER_MOTD_DEFAULT.to_string(),
            GatewayProperty::BannerLogin => BANNER_LOGIN_DEFAULT.to_string(),
            GatewayProperty::BannerLogout => BANNER_LOGOUT_DEFAULT.to_string(),
            GatewayProperty::AdminHtml => WEBAPP_ADMIN_HTML_DEFAULT.to_string(),
            GatewayProperty::AdminCss => WEBAPP_ADMIN_CSS_DEFAULT.to_string(),
            GatewayProperty::AdminJs => WEBAPP_ADMIN_JS_DEFAULT.to_string(),
            GatewayProperty::ClientHtml => WEBAPP_CLIENT_HTML_DEFAULT.to_string(),
            GatewayProperty::ClientCss => WEBAPP_CLIENT_CSS_DEFAULT.to_string(),
            GatewayProperty::ClientJs => WEBAPP_CLIENT_JS_DEFAULT.to_string(),
        })
    }

    /// Refresh the banner cache from database settings table via RPC
    #[tracing::instrument(skip(self))]
    pub async fn refresh_cached_properties(
        &self,
        properties: &[GatewayProperty],
    ) -> Result<HashMap<GatewayProperty, String>, String> {
        tracing::debug!("Refreshing banner cache from server via RPC");

        // Call fetch_gateway_properties RPC
        let request = wyldlands_common::proto::GatewayPropertiesRequest {
            properties: properties
                .iter()
                .map(|p| p.as_str().to_string())
                .collect(),
        };

        // Check if the RPC client is connected
        if !self.rpc_client.is_connected().await {
            tracing::warn!("RPC client not connected, unable to refresh properties");
            return Ok(HashMap::default());
        }

        // Get RPC client
        let mut client = match self.rpc_client.gateway_client().await {
            Some(c) => c,
            None => {
                tracing::warn!("No RPC client available, unable to refresh properties");
                return Ok(HashMap::default());
            }
        };

        match client.fetch_gateway_properties(request).await {
            Ok(response) => {
                let response = response.into_inner();
                let now = SystemTime::now();
                let mut out = HashMap::default();
                let mut cache = self.cache.write().await;

                for (key, value) in response.properties {
                    if let Some(property) = GatewayProperty::from_str(&key) {
                        cache.properties.insert(property, (value.clone(), now));
                        out.insert(property, value);
                    }
                }

                cache.last_update = now;
                tracing::info!("Gateway Property cache refreshed from server via RPC");
                Ok(out)
            }
            Err(e) => {
                tracing::error!("Unable to refresh gateway properties cache via RPC: {}", e);
                Err(format!("RPC error: {}", e))
            }
        }
    }
}

#[derive(Debug)]
struct Cache {
    properties: HashMap<GatewayProperty, (String, SystemTime)>,
    last_update: SystemTime,
}

pub const BANNER_WELCOME_DEFAULT: &str = include_str!("../defaults/welcome.txt");
pub const BANNER_MOTD_DEFAULT: &str = include_str!("../defaults/motd.txt");
pub const BANNER_LOGIN_DEFAULT: &str = include_str!("../defaults/login.txt");
pub const BANNER_LOGOUT_DEFAULT: &str = include_str!("../defaults/logout.txt");
pub const WEBAPP_ADMIN_HTML_DEFAULT: &str = include_str!("../defaults/admin.html");
pub const WEBAPP_ADMIN_CSS_DEFAULT: &str = include_str!("../defaults/admin.css");
pub const WEBAPP_ADMIN_JS_DEFAULT: &str = include_str!("../defaults/admin.js");
pub const WEBAPP_CLIENT_HTML_DEFAULT: &str = include_str!("../defaults/client.html");
pub const WEBAPP_CLIENT_CSS_DEFAULT: &str = include_str!("../defaults/client.css");
pub const WEBAPP_CLIENT_JS_DEFAULT: &str = include_str!("../defaults/client.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banner_type() {
        assert_eq!(
            GatewayProperty::BannerWelcome,
            GatewayProperty::BannerWelcome
        );
        assert_ne!(GatewayProperty::BannerWelcome, GatewayProperty::BannerMotd);
    }

    #[tokio::test]
    async fn test_properties_manager_cache_logic() {
        let rpc_client = Arc::new(crate::grpc::RpcClientManager::new(
            "127.0.0.1:9000",
            "test-key",
            5,
            30,
        ));
        let manager = PropertiesManager::new(rpc_client, 1); // 1 second TTL

        // Initially cache is empty
        {
            let cache = manager.cache.read().await;
            assert!(cache.properties.get(&GatewayProperty::BannerWelcome).is_none());
        }

        // Manually populate cache
        let now = SystemTime::now();
        {
            let mut cache = manager.cache.write().await;
            cache.properties.insert(
                GatewayProperty::BannerWelcome,
                ("Cached Welcome".to_string(), now),
            );
        }

        // Should get from cache
        let prop = manager.get_property(GatewayProperty::BannerWelcome).await;
        assert_eq!(prop.unwrap(), "Cached Welcome");

        // Make it stale
        let stale_time = now - Duration::from_secs(2);
        {
            let mut cache = manager.cache.write().await;
            cache.properties.insert(
                GatewayProperty::BannerWelcome,
                ("Stale Welcome".to_string(), stale_time),
            );
        }

        // Should attempt refresh (and fail since no server) and return default
        let prop = manager.get_property(GatewayProperty::BannerWelcome).await;
        assert_eq!(prop.unwrap(), BANNER_WELCOME_DEFAULT);
    }
}
