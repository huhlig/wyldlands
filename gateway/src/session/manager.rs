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

use crate::session::{GatewaySession, ProtocolType, SessionState};
use metrics::{counter, gauge};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{Level, event};
use uuid::Uuid;

/// Session manager for in-memory session tracking
///
/// Sessions are stored in-memory only and are not persisted to the database.
/// This is appropriate for the gateway service which only needs to track
/// active connections.
pub struct SessionManager {
    /// Active sessions in memory
    sessions: Arc<RwLock<HashMap<Uuid, GatewaySession>>>,

    /// Queued commands for disconnected sessions
    queued_commands: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,

    /// Session timeout in seconds
    timeout_seconds: i64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(timeout_seconds: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            queued_commands: Arc::new(RwLock::new(HashMap::new())),
            timeout_seconds,
        }
    }

    /// Create a new session
    #[tracing::instrument(skip(self))]
    pub async fn create_session(
        &self,
        protocol: ProtocolType,
        client_addr: String,
    ) -> Result<Uuid, String> {
        let session = GatewaySession::new(protocol, client_addr);
        let session_id = session.session_id;

        // Store in memory
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        // Record metrics
        let protocol_str = match protocol {
            ProtocolType::Telnet => "telnet",
            ProtocolType::WebSocket => "websocket",
        };
        counter!("gateway_sessions_created_total", "server" => protocol_str).increment(1);
        gauge!("gateway_sessions_active").increment(1.0);
        event!(Level::INFO, protocol = %protocol_str, metric = "session_created", "Session created");

        Ok(session_id)
    }

    /// Get a session by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_session(&self, id: Uuid) -> Option<GatewaySession> {
        let sessions = self.sessions.read().await;
        sessions.get(&id).cloned()
    }

    /// Update a session
    #[tracing::instrument(skip(self))]
    pub async fn update_session(&self, session: GatewaySession) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id, session);
        Ok(())
    }

    /// Remove a session
    #[tracing::instrument(skip(self))]
    pub async fn remove_session(&self, id: Uuid) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&id);

        // Also remove any queued commands
        let mut queued = self.queued_commands.write().await;
        queued.remove(&id);

        // Record metrics
        gauge!("gateway_sessions_active").decrement(1.0);

        Ok(())
    }

    /// Delete a session (alias for remove_session)
    #[tracing::instrument(skip(self))]
    pub async fn delete_session(&self, id: Uuid) -> Result<(), String> {
        self.remove_session(id).await
    }

    /// Touch a session (update last activity)
    #[tracing::instrument(skip(self))]
    pub async fn touch_session(&self, id: Uuid) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&id) {
            session.touch();
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Transition a session to a new state
    #[tracing::instrument(skip(self))]
    pub async fn transition_session(
        &self,
        id: Uuid,
        new_state: SessionState,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        
        let lock_start = std::time::Instant::now();
        let mut sessions = self.sessions.write().await;
        let lock_duration = lock_start.elapsed();
        
        tracing::debug!(
            session_id = %id,
            lock_wait_ms = %lock_duration.as_millis(),
            "transition_session: acquired write lock"
        );
        
        if let Some(session) = sessions.get_mut(&id) {
            let old_metric_str = session.state.to_metric_str();
            let new_metric_str = new_state.to_metric_str();

            let transition_start = std::time::Instant::now();
            let result = session.transition(new_state);
            let transition_duration = transition_start.elapsed();

            if result.is_ok() {
                // Record metrics
                gauge!("gateway_sessions_by_state", "state" => old_metric_str).decrement(1.0);
                gauge!("gateway_sessions_by_state", "state" => new_metric_str).increment(1.0);
                event!(Level::DEBUG, old_state = %old_metric_str, new_state = %new_metric_str, metric = "session_state_change", "Session state changed");
            }

            let total_duration = start.elapsed();
            tracing::info!(
                session_id = %id,
                old_state = %old_metric_str,
                new_state = %new_metric_str,
                lock_wait_ms = %lock_duration.as_millis(),
                transition_ms = %transition_duration.as_millis(),
                total_ms = %total_duration.as_millis(),
                success = %result.is_ok(),
                "transition_session completed"
            );

            result
        } else {
            let total_duration = start.elapsed();
            tracing::warn!(
                session_id = %id,
                total_ms = %total_duration.as_millis(),
                "transition_session: session not found"
            );
            Err("Session not found".to_string())
        }
    }

    /// Cleanup expired sessions
    #[tracing::instrument(skip(self))]
    pub async fn cleanup_expired(&self) -> Result<usize, String> {
        let mut expired_ids = Vec::new();

        // Find expired sessions
        {
            let sessions = self.sessions.read().await;
            for (id, session) in sessions.iter() {
                if session.is_expired(self.timeout_seconds) {
                    expired_ids.push(*id);
                }
            }
        }

        // Remove expired sessions
        let count = expired_ids.len();
        for id in expired_ids {
            let _ = self.remove_session(id).await;

            // Record metrics for each expired session
            counter!("gateway_sessions_expired_total").increment(1);
            event!(Level::INFO, metric = "session_expired", "Session expired");
        }

        Ok(count)
    }

    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<GatewaySession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.state.is_authenticated())
            .cloned()
            .collect()
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Queue a command for a disconnected session
    #[tracing::instrument(skip(self))]
    pub async fn queue_command(&self, session_id: Uuid, command: &str) -> Result<(), String> {
        let mut queued = self.queued_commands.write().await;
        queued
            .entry(session_id)
            .or_insert_with(Vec::new)
            .push(command.to_string());
        Ok(())
    }

    /// Get and clear queued commands for a session
    #[tracing::instrument(skip(self))]
    pub async fn get_and_clear_queued_commands(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<String>, String> {
        let mut queued = self.queued_commands.write().await;
        Ok(queued.remove(&session_id).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{AuthenticatedState, UnauthenticatedState};

    async fn create_test_manager() -> SessionManager {
        SessionManager::new(300)
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        // Verify session exists
        let session = manager.get_session(session_id).await;
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.session_id, session_id);
        assert_eq!(
            session.state,
            SessionState::Unauthenticated(UnauthenticatedState::Welcome)
        );
        assert_eq!(session.protocol, ProtocolType::WebSocket);
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        // Get existing session
        let session = manager.get_session(session_id).await;
        assert!(session.is_some());

        // Get non-existent session
        let non_existent = manager.get_session(Uuid::new_v4()).await;
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_update_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        // Get and modify session
        let mut session = manager.get_session(session_id).await.unwrap();
        session.state = SessionState::Authenticated(AuthenticatedState::Playing);

        // Update session
        manager
            .update_session(session.clone())
            .await
            .expect("Failed to update session");

        // Verify update
        let updated = manager.get_session(session_id).await.unwrap();
        assert_eq!(
            updated.state,
            SessionState::Authenticated(AuthenticatedState::Playing)
        );
    }

    #[tokio::test]
    async fn test_remove_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        // Verify session exists
        assert!(manager.get_session(session_id).await.is_some());

        // Remove session
        manager
            .remove_session(session_id)
            .await
            .expect("Failed to remove session");

        // Verify session is gone
        assert!(manager.get_session(session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_touch_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        let initial_session = manager.get_session(session_id).await.unwrap();
        let initial_time = initial_session.last_activity;

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Touch session
        manager
            .touch_session(session_id)
            .await
            .expect("Failed to touch session");

        // Verify timestamp updated
        let touched_session = manager.get_session(session_id).await.unwrap();
        assert!(touched_session.last_activity > initial_time);
    }

    #[tokio::test]
    async fn test_transition_session() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        // Valid transition: Unauthenticated -> Authenticated
        manager
            .transition_session(
                session_id,
                SessionState::Authenticated(AuthenticatedState::Playing),
            )
            .await
            .expect("Failed to transition to Authenticated");

        let session = manager.get_session(session_id).await.unwrap();
        assert_eq!(
            session.state,
            SessionState::Authenticated(AuthenticatedState::Playing)
        );

        // Valid transition: Authenticated -> Disconnected
        manager
            .transition_session(session_id, SessionState::Disconnected)
            .await
            .expect("Failed to transition to Disconnected");

        let session = manager.get_session(session_id).await.unwrap();
        assert_eq!(session.state, SessionState::Disconnected);

        // Invalid transition: Disconnected -> Username (not allowed)
        let result = manager
            .transition_session(
                session_id,
                SessionState::Unauthenticated(UnauthenticatedState::Username),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let manager = create_test_manager().await;

        // Create active session
        let active_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create active session");

        // Create expired session
        let expired_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create expired session");

        // Make session expired
        let mut expired_session = manager.get_session(expired_id).await.unwrap();
        expired_session.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);
        manager
            .update_session(expired_session)
            .await
            .expect("Failed to update expired session");

        // Cleanup
        let cleaned = manager.cleanup_expired().await.expect("Failed to cleanup");
        assert_eq!(cleaned, 1);

        // Verify expired session is gone
        assert!(manager.get_session(expired_id).await.is_none());

        // Verify active session remains
        assert!(manager.get_session(active_id).await.is_some());
    }

    #[tokio::test]
    async fn test_get_active_sessions() {
        let manager = create_test_manager().await;

        // Create sessions in different states
        let connected_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");
        manager
            .transition_session(
                connected_id,
                SessionState::Authenticated(AuthenticatedState::Playing),
            )
            .await
            .unwrap();

        let _unauthenticated_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        // Get active sessions
        let active = manager.get_active_sessions().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].session_id, connected_id);
    }

    #[tokio::test]
    async fn test_session_count() {
        let manager = create_test_manager().await;

        assert_eq!(manager.session_count().await, 0);

        let _id1 = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        assert_eq!(manager.session_count().await, 1);

        let _id2 = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        assert_eq!(manager.session_count().await, 2);
    }

    #[tokio::test]
    async fn test_queue_and_get_commands() {
        let manager = create_test_manager().await;

        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");

        // Queue commands
        manager
            .queue_command(session_id, "look")
            .await
            .expect("Failed to queue command");
        manager
            .queue_command(session_id, "inventory")
            .await
            .expect("Failed to queue command");
        manager
            .queue_command(session_id, "north")
            .await
            .expect("Failed to queue command");

        // Get and clear commands
        let commands = manager
            .get_and_clear_queued_commands(session_id)
            .await
            .expect("Failed to get commands");

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "look");
        assert_eq!(commands[1], "inventory");
        assert_eq!(commands[2], "north");

        // Verify commands are cleared
        let commands = manager
            .get_and_clear_queued_commands(session_id)
            .await
            .expect("Failed to get commands");
        assert_eq!(commands.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_session_access() {
        let manager = Arc::new(create_test_manager().await);

        let session_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");

        // Spawn multiple tasks that access the session concurrently
        let mut handles = vec![];

        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                manager_clone
                    .touch_session(session_id)
                    .await
                    .expect("Failed to touch session");
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify session still exists and is valid
        let session = manager.get_session(session_id).await;
        assert!(session.is_some());
    }
}


