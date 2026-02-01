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

use crate::RpcClientManager;
//use crate::auth::AuthManager;
use crate::pool::ConnectionPool;
use crate::properties::PropertiesManager;
use crate::session::manager::SessionManager;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Server context containing shared resources
#[derive(Clone)]
pub struct ServerContext {
    /// When did the gateway Startup
    pub startup_time: SystemTime,

    /// Session manager for tracking sessions
    pub session_manager: Arc<SessionManager>,

    /// Connection pool for managing active connections
    pub connection_pool: Arc<ConnectionPool>,

    /// Authentication manager
    //  pub auth_manager: Arc<AuthManager>,

    /// Banner manager
    pub banner_manager: Arc<PropertiesManager>,

    /// RPC client for server communication
    pub rpc_client: Arc<RpcClientManager>,
}

impl ServerContext {
    /// Create a new server context
    pub fn new(session_timeout_seconds: i64, rpc_client: Arc<RpcClientManager>) -> Self {
        let session_manager = Arc::new(SessionManager::new(session_timeout_seconds));
        let connection_pool = Arc::new(ConnectionPool::new(Arc::clone(&session_manager)));
        //let auth_manager = Arc::new(AuthManager::new(Arc::clone(&rpc_client)));
        let banner_manager = Arc::new(PropertiesManager::new(Arc::clone(&rpc_client), 300)); // 5 min cache

        Self {
            startup_time: SystemTime::now(),
            session_manager,
            connection_pool,
            //            auth_manager,
            banner_manager,
            rpc_client,
        }
    }

    /// Get Gateway Uptime
    pub fn gateway_uptime(&self) -> Duration {
        self.startup_time.elapsed().unwrap()
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get the connection pool
    pub fn connection_pool(&self) -> &Arc<ConnectionPool> {
        &self.connection_pool
    }

    /// Get the authentication manager
    //pub fn auth_manager(&self) -> &Arc<AuthManager> { &self.auth_manager    }

    /// Get the banner manager
    pub fn properties_manager(&self) -> &Arc<PropertiesManager> {
        &self.banner_manager
    }

    /// Get the RPC client
    pub fn rpc_client(&self) -> &Arc<RpcClientManager> {
        &self.rpc_client
    }
}
