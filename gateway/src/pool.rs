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

use crate::session::ProtocolType;
use crate::session::manager::SessionManager;
use metrics::{counter, gauge};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{Level, event};
use uuid::Uuid;

/// Message types for connection pool communication
#[derive(Debug)]
pub enum PoolMessage {
    /// Register a new connection
    Register { session_id: Uuid },

    /// Unregister a connection
    Unregister { session_id: Uuid },

    /// Send data to a connection
    Send { session_id: Uuid, data: Vec<u8> },

    /// Broadcast data to all active connections
    Broadcast { data: Vec<u8> },

    /// Broadcast data to specific sessions
    BroadcastTo {
        session_ids: Vec<Uuid>,
        data: Vec<u8>,
    },

    /// Get connection count
    GetCount {
        response: tokio::sync::oneshot::Sender<usize>,
    },

    /// Shutdown the pool
    Shutdown,
}

/// Connection handle for managing individual connections
pub struct ConnectionHandle {
    session_id: Uuid,
    protocol: ProtocolType,
    sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl ConnectionHandle {
    /// Create a new connection handle
    pub fn new(
        session_id: Uuid,
        protocol: ProtocolType,
        sender: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            session_id,
            protocol,
            sender,
        }
    }

    /// Send data to this connection
    pub fn send(&self, data: Vec<u8>) -> Result<(), String> {
        self.sender
            .send(data)
            .map_err(|e| format!("Failed to send data: {}", e))
    }

    /// Get session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get server type
    pub fn protocol(&self) -> ProtocolType {
        self.protocol
    }
}

/// Connection pool for managing active connections
pub struct ConnectionPool {
    /// Active connection handles
    connections: Arc<RwLock<HashMap<Uuid, ConnectionHandle>>>,

    /// Session manager
    session_manager: Arc<SessionManager>,

    /// Message sender for pool operations
    sender: mpsc::UnboundedSender<PoolMessage>,

    /// Message receiver for pool operations
    receiver: Arc<RwLock<mpsc::UnboundedReceiver<PoolMessage>>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            session_manager,
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
        }
    }

    /// Get a sender for pool messages
    pub fn sender(&self) -> mpsc::UnboundedSender<PoolMessage> {
        self.sender.clone()
    }

    /// Register a new connection
    pub async fn register(
        &self,
        session_id: Uuid,
        protocol: ProtocolType,
    ) -> Result<mpsc::UnboundedSender<Vec<u8>>, String> {
        let (tx, _rx) = mpsc::unbounded_channel();

        let handle = ConnectionHandle::new(session_id, protocol, tx.clone());

        {
            let mut connections = self.connections.write().await;
            connections.insert(session_id, handle);
        }

        // Touch the session to update activity
        self.session_manager.touch_session(session_id).await?;

        // Record metrics
        counter!("gateway_pool_register_total").increment(1);
        gauge!("gateway_pool_size").increment(1.0);
        event!(
            Level::DEBUG,
            metric = "pool_register",
            "Connection registered in pool"
        );

        Ok(tx)
    }

    /// Unregister a connection
    pub async fn unregister(&self, session_id: Uuid) -> Result<(), String> {
        {
            let mut connections = self.connections.write().await;
            connections.remove(&session_id);
        }

        // Transition session to disconnected state
        self.session_manager
            .transition_session(session_id, crate::session::SessionState::Disconnected)
            .await?;

        // Record metrics
        counter!("gateway_pool_unregister_total").increment(1);
        gauge!("gateway_pool_size").decrement(1.0);
        event!(
            Level::DEBUG,
            metric = "pool_unregister",
            "Connection unregistered from pool"
        );

        Ok(())
    }

    /// Send data to a specific connection
    pub async fn send(&self, session_id: Uuid, data: Vec<u8>) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(handle) = connections.get(&session_id) {
            handle.send(data)?;

            // Touch the session to update activity
            drop(connections);
            self.session_manager.touch_session(session_id).await?;

            Ok(())
        } else {
            Err(format!("Connection not found for session {}", session_id))
        }
    }

    /// Broadcast data to all active connections
    pub async fn broadcast(&self, data: Vec<u8>) -> Result<usize, String> {
        let connections = self.connections.read().await;
        let mut sent_count = 0;

        for handle in connections.values() {
            if handle.send(data.clone()).is_ok() {
                sent_count += 1;
            }
        }

        // Record metrics
        counter!("gateway_messages_broadcast_total").increment(1);
        event!(Level::DEBUG, recipients = %sent_count, metric = "message_broadcast", "Message broadcast");

        Ok(sent_count)
    }

    /// Broadcast data to specific sessions
    pub async fn broadcast_to(&self, session_ids: &[Uuid], data: Vec<u8>) -> Result<usize, String> {
        let connections = self.connections.read().await;
        let mut sent_count = 0;

        for session_id in session_ids {
            if let Some(handle) = connections.get(session_id) {
                if handle.send(data.clone()).is_ok() {
                    sent_count += 1;
                }
            }
        }

        Ok(sent_count)
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Get all active session IDs
    pub async fn active_sessions(&self) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }

    /// Get connections by server type
    pub async fn connections_by_protocol(&self, protocol: ProtocolType) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections
            .values()
            .filter(|h| h.protocol() == protocol)
            .map(|h| h.session_id())
            .collect()
    }

    /// Count connections by server type
    pub async fn count_by_protocol(&self, protocol: ProtocolType) -> usize {
        let connections = self.connections.read().await;
        connections
            .values()
            .filter(|h| h.protocol() == protocol)
            .count()
    }

    /// Run the connection pool message handler
    pub async fn run(&self) {
        let mut receiver = self.receiver.write().await;

        while let Some(message) = receiver.recv().await {
            match message {
                PoolMessage::Register { session_id } => {
                    // Connection registration is handled by the register method
                    tracing::debug!("Received register message for session {}", session_id);
                }

                PoolMessage::Unregister { session_id } => {
                    if let Err(e) = self.unregister(session_id).await {
                        tracing::error!("Failed to unregister session {}: {}", session_id, e);
                    }
                }

                PoolMessage::Send { session_id, data } => {
                    if let Err(e) = self.send(session_id, data).await {
                        tracing::error!("Failed to send to session {}: {}", session_id, e);
                    }
                }

                PoolMessage::Broadcast { data } => {
                    if let Err(e) = self.broadcast(data).await {
                        tracing::error!("Failed to broadcast: {}", e);
                    }
                }

                PoolMessage::BroadcastTo { session_ids, data } => {
                    if let Err(e) = self.broadcast_to(&session_ids, data).await {
                        tracing::error!("Failed to broadcast to specific sessions: {}", e);
                    }
                }

                PoolMessage::GetCount { response } => {
                    let count = self.connection_count().await;
                    let _ = response.send(count);
                }

                PoolMessage::Shutdown => {
                    tracing::info!("Connection pool shutting down");
                    break;
                }
            }
        }
    }

    /// Cleanup disconnected sessions
    pub async fn cleanup_disconnected(&self) -> Result<usize, String> {
        let active_sessions = self.active_sessions().await;
        let all_sessions = self.session_manager.get_active_sessions().await;

        let mut cleaned = 0;

        for session in all_sessions {
            if !active_sessions.contains(&session.session_id) {
                // Session exists in manager but not in pool - it's disconnected
                if session.state.is_authenticated() {
                    self.session_manager
                        .transition_session(
                            session.session_id,
                            crate::session::SessionState::Disconnected,
                        )
                        .await?;
                    cleaned += 1;
                }
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> ConnectionPool {
        let manager = Arc::new(SessionManager::new(300));
        ConnectionPool::new(manager)
    }

    #[tokio::test]
    #[ignore = "Requires database setup"]
    async fn test_pool_creation() {
        let pool = create_test_pool().await;
        assert_eq!(pool.connection_count().await, 0);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_register_connection() {
        let pool = create_test_pool().await;

        // Create session first
        let session_id = pool
            .session_manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        // Register connection
        let _sender = pool
            .register(session_id, ProtocolType::WebSocket)
            .await
            .expect("Failed to register connection");

        assert_eq!(pool.connection_count().await, 1);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_unregister_connection() {
        let pool = create_test_pool().await;

        // Create and register session
        let session_id = pool
            .session_manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        let _sender = pool
            .register(session_id, ProtocolType::Telnet)
            .await
            .expect("Failed to register connection");

        assert_eq!(pool.connection_count().await, 1);

        // Unregister
        pool.unregister(session_id)
            .await
            .expect("Failed to unregister connection");

        assert_eq!(pool.connection_count().await, 0);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_send_to_connection() {
        let pool = create_test_pool().await;

        let session_id = pool
            .session_manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        let _sender = pool
            .register(session_id, ProtocolType::WebSocket)
            .await
            .expect("Failed to register connection");

        // Send data
        let data = b"Hello, World!".to_vec();
        pool.send(session_id, data)
            .await
            .expect("Failed to send data");
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_broadcast() {
        let pool = create_test_pool().await;

        // Create multiple sessions
        for i in 0..3 {
            let session_id = pool
                .session_manager
                .create_session(ProtocolType::WebSocket, format!("127.0.0.1:808{}", i))
                .await
                .expect("Failed to create session");

            pool.register(session_id, ProtocolType::WebSocket)
                .await
                .expect("Failed to register connection");
        }

        assert_eq!(pool.connection_count().await, 3);

        // Broadcast
        let data = b"Broadcast message".to_vec();
        let sent = pool.broadcast(data).await.expect("Failed to broadcast");

        assert_eq!(sent, 3);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_broadcast_to_specific() {
        let pool = create_test_pool().await;

        // Create multiple sessions
        let mut session_ids = Vec::new();
        for i in 0..5 {
            let session_id = pool
                .session_manager
                .create_session(ProtocolType::Telnet, format!("127.0.0.1:2{}", i))
                .await
                .expect("Failed to create session");

            pool.register(session_id, ProtocolType::Telnet)
                .await
                .expect("Failed to register connection");

            session_ids.push(session_id);
        }

        // Broadcast to first 3 sessions only
        let target_sessions = &session_ids[0..3];
        let data = b"Targeted message".to_vec();
        let sent = pool
            .broadcast_to(target_sessions, data)
            .await
            .expect("Failed to broadcast to specific sessions");

        assert_eq!(sent, 3);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_connections_by_protocol() {
        let pool = create_test_pool().await;

        // Create WebSocket sessions
        for i in 0..2 {
            let session_id = pool
                .session_manager
                .create_session(ProtocolType::WebSocket, format!("127.0.0.1:808{}", i))
                .await
                .expect("Failed to create session");

            pool.register(session_id, ProtocolType::WebSocket)
                .await
                .expect("Failed to register connection");
        }

        // Create Telnet sessions
        for i in 0..3 {
            let session_id = pool
                .session_manager
                .create_session(ProtocolType::Telnet, format!("127.0.0.1:2{}", i))
                .await
                .expect("Failed to create session");

            pool.register(session_id, ProtocolType::Telnet)
                .await
                .expect("Failed to register connection");
        }

        let ws_connections = pool.connections_by_protocol(ProtocolType::WebSocket).await;
        let telnet_connections = pool.connections_by_protocol(ProtocolType::Telnet).await;

        assert_eq!(ws_connections.len(), 2);
        assert_eq!(telnet_connections.len(), 3);
    }

    #[tokio::test]
    #[ignore] // Requires database setup with proper permissions
    async fn test_active_sessions() {
        let pool = create_test_pool().await;

        let mut expected_ids = Vec::new();
        for i in 0..3 {
            let session_id = pool
                .session_manager
                .create_session(ProtocolType::WebSocket, format!("127.0.0.1:808{}", i))
                .await
                .expect("Failed to create session");

            pool.register(session_id, ProtocolType::WebSocket)
                .await
                .expect("Failed to register connection");

            expected_ids.push(session_id);
        }

        let active = pool.active_sessions().await;
        assert_eq!(active.len(), 3);

        for id in expected_ids {
            assert!(active.contains(&id));
        }
    }
}
