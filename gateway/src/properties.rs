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
    cache: Arc<RwLock<HashMap<GatewayProperty, (String, SystemTime)>>>,

    /// Cache TTL
    cache_ttl: Duration,
}

impl PropertiesManager {
    /// Create a new banner manager
    pub fn new(rpc_client: Arc<crate::grpc::RpcClientManager>, cache_ttl_secs: u64) -> Self {
        Self {
            rpc_client,
            cache: Arc::new(RwLock::new(HashMap::default())),
            cache_ttl: Duration::from_secs(cache_ttl_secs),
        }
    }

    /// Get a banner by type
    pub async fn get_property(&self, property: GatewayProperty) -> Result<String, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((value, last_update)) = cache.get(&property) {
                if let Ok(elapsed) = last_update.elapsed()
                    && elapsed < self.cache_ttl
                {
                    return Ok(value.clone());
                }
            }
        }

        // Cache miss or expired, refresh cache from database
        self.refresh_cached_properties(&[property]).await?;

        // Check Cache Again
        {
            let cache = self.cache.read().await;
            if let Some((value, last_update)) = cache.get(&property) {
                if let Ok(elapsed) = last_update.elapsed()
                    && elapsed < self.cache_ttl
                {
                    return Ok(value.clone());
                }
            }
        }

        // Else use the default
        {
            match property {
                GatewayProperty::BannerWelcome => Ok(BANNER_WELCOME_DEFAULT.to_string()),
                GatewayProperty::BannerMotd => Ok(BANNER_MOTD_DEFAULT.to_string()),
                GatewayProperty::BannerLogin => Ok(BANNER_LOGIN_DEFAULT.to_string()),
                GatewayProperty::BannerLogout => Ok(BANNER_LOGOUT_DEFAULT.to_string()),
                GatewayProperty::AdminHtml => Ok(WEBAPP_ADMIN_HTML_DEFAULT.to_string()),
                GatewayProperty::AdminCss => Ok(WEBAPP_ADMIN_CSS_DEFAULT.to_string()),
                GatewayProperty::AdminJs => Ok(WEBAPP_ADMIN_JS_DEFAULT.to_string()),
                GatewayProperty::ClientHtml => Ok(WEBAPP_CLIENT_HTML_DEFAULT.to_string()),
                GatewayProperty::ClientCss => Ok(WEBAPP_CLIENT_CSS_DEFAULT.to_string()),
                GatewayProperty::ClientJs => Ok(WEBAPP_CLIENT_JS_DEFAULT.to_string()),
            }
        }
    }

    /// Refresh the banner cache from database settings table via RPC
    pub async fn refresh_cached_properties(
        &self,
        properties: &[GatewayProperty],
    ) -> Result<HashMap<GatewayProperty, String>, String> {
        tracing::debug!("Refreshing banner cache from server via RPC");

        // Check if the RPC client is connected
        if !self.rpc_client.is_connected().await {
            tracing::warn!("RPC client not connected, unable to refresh properties");
            return Ok(HashMap::default());
        }

        // Get RPC client
        use wyldlands_common::proto::GatewayManagementClient;
        let mut client: GatewayManagementClient = match self.rpc_client.gateway_client().await {
            Some(c) => c,
            None => {
                tracing::warn!("No RPC client available, unable to refresh properties");
                return Ok(HashMap::default());
            }
        };

        // Call fetch_gateway_properties RPC
        let request = wyldlands_common::proto::GatewayPropertiesRequest {
            properties: Vec::from_iter(
                properties
                    .iter()
                    .map(|property| property.as_str().to_string()),
            ),
        };

        match client.fetch_gateway_properties(request).await {
            Ok(response) => {
                let response = response.into_inner();

                // Update cache with server banners
                let mut out = HashMap::default();
                let mut cache = self.cache.write().await;
                for (key, value) in response.properties {
                    if let Some(property) = GatewayProperty::from_str(&key) {
                        cache.insert(property, (value.clone(), SystemTime::now()));
                        out.insert(property, value);
                    }
                }

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

const BANNER_WELCOME_DEFAULT: &str = include_str!("../defaults/welcome.txt");
const BANNER_MOTD_DEFAULT: &str = include_str!("../defaults/motd.txt");
const BANNER_LOGIN_DEFAULT: &str = include_str!("../defaults/login.txt");
const BANNER_LOGOUT_DEFAULT: &str = include_str!("../defaults/logout.txt");
const WEBAPP_ADMIN_HTML_DEFAULT: &str = include_str!("../defaults/admin.html");
const WEBAPP_ADMIN_CSS_DEFAULT: &str = include_str!("../defaults/admin.css");
const WEBAPP_ADMIN_JS_DEFAULT: &str = include_str!("../defaults/admin.js");
const WEBAPP_CLIENT_HTML_DEFAULT: &str = include_str!("../defaults/client.html");
const WEBAPP_CLIENT_CSS_DEFAULT: &str = include_str!("../defaults/client.css");
const WEBAPP_CLIENT_JS_DEFAULT: &str = include_str!("../defaults/client.js");

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
}
