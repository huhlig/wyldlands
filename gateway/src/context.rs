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

use crate::auth::AuthManager;
use crate::banner::BannerManager;
use crate::pool::ConnectionPool;
use crate::session::{store::SessionStore, manager::SessionManager};
use crate::rpc_client::RpcClientManager;
use std::sync::Arc;

/// Server context containing shared resources
#[derive(Clone)]
pub struct ServerContext {
    /// Database connection pool
    pub database: sqlx::postgres::PgPool,
    
    /// Session manager for tracking sessions
    pub session_manager: Arc<SessionManager>,
    
    /// Connection pool for managing active connections
    pub connection_pool: Arc<ConnectionPool>,
    
    /// Authentication manager
    pub auth_manager: Arc<AuthManager>,
    
    /// Banner manager
    pub banner_manager: Arc<BannerManager>,
    
    /// RPC client for server communication
    pub rpc_client: Arc<RpcClientManager>,
}

impl ServerContext {
    /// Create a new server context
    pub fn new(
        database: sqlx::postgres::PgPool,
        session_timeout_seconds: i64,
        rpc_client: Arc<RpcClientManager>,
    ) -> Self {
        let store = SessionStore::new(database.clone());
        let session_manager = Arc::new(SessionManager::new(store, session_timeout_seconds));
        let connection_pool = Arc::new(ConnectionPool::new(Arc::clone(&session_manager)));
        let auth_manager = Arc::new(AuthManager::new(database.clone()));
        let banner_manager = Arc::new(BannerManager::new(database.clone(), 300)); // 5 min cache
        
        Self {
            database,
            session_manager,
            connection_pool,
            auth_manager,
            banner_manager,
            rpc_client,
        }
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
    pub fn auth_manager(&self) -> &Arc<AuthManager> {
        &self.auth_manager
    }
    
    /// Get the banner manager
    pub fn banner_manager(&self) -> &Arc<BannerManager> {
        &self.banner_manager
    }
    
    /// Get the RPC client
    pub fn rpc_client(&self) -> &Arc<RpcClientManager> {
        &self.rpc_client
    }
}