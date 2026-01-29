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

use crate::session::{Session, SessionState, ProtocolType};
use crate::session::store::SessionStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session manager for in-memory session tracking
pub struct SessionManager {
    /// Active sessions in memory
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    
    /// Database store for persistence
    store: SessionStore,
    
    /// Session timeout in seconds
    timeout_seconds: i64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(store: SessionStore, timeout_seconds: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            store,
            timeout_seconds,
        }
    }
    
    /// Create a new session
    pub async fn create_session(
        &self,
        protocol: ProtocolType,
        client_addr: String,
    ) -> Result<Uuid, String> {
        let session = Session::new(protocol, client_addr);
        let session_id = session.id;
        
        // Store in memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, session.clone());
        }
        
        // Persist to database
        self.store.save(&session).await
            .map_err(|e| format!("Failed to save session: {}", e))?;
        
        Ok(session_id)
    }
    
    /// Get a session by ID
    pub async fn get_session(&self, id: Uuid) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(&id).cloned()
    }
    
    /// Update a session
    pub async fn update_session(&self, session: Session) -> Result<(), String> {
        // Update in memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session.id, session.clone());
        }
        
        // Persist to database
        self.store.save(&session).await
            .map_err(|e| format!("Failed to update session: {}", e))?;
        
        Ok(())
    }
    
    /// Remove a session
    pub async fn remove_session(&self, id: Uuid) -> Result<(), String> {
        // Remove from memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(&id);
        }
        
        // Delete from database
        self.store.delete(id).await
            .map_err(|e| format!("Failed to delete session: {}", e))?;
        
        Ok(())
    }
    
    /// Delete a session (alias for remove_session)
    pub async fn delete_session(&self, id: Uuid) -> Result<(), String> {
        self.remove_session(id).await
    }
    
    /// Touch a session (update last activity)
    pub async fn touch_session(&self, id: Uuid) -> Result<(), String> {
        let mut session = self.get_session(id).await
            .ok_or_else(|| "Session not found".to_string())?;
        
        session.touch();
        self.update_session(session).await
    }
    
    /// Transition a session to a new state
    pub async fn transition_session(
        &self,
        id: Uuid,
        new_state: SessionState,
    ) -> Result<(), String> {
        let mut session = self.get_session(id).await
            .ok_or_else(|| "Session not found".to_string())?;
        
        session.transition(new_state)?;
        self.update_session(session).await
    }
    
    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self) -> Result<usize, String> {
        let mut expired_ids = Vec::new();
        
        // Find expired sessions in memory
        {
            let sessions = self.sessions.read().await;
            for (id, session) in sessions.iter() {
                if session.is_expired(self.timeout_seconds) {
                    expired_ids.push(*id);
                }
            }
        }
        
        // Remove expired sessions
        for id in &expired_ids {
            let _ = self.remove_session(*id).await;
        }
        
        // Cleanup database
        let db_cleaned = self.store.cleanup_expired(self.timeout_seconds).await
            .map_err(|e| format!("Failed to cleanup database: {}", e))?;
        
        Ok(expired_ids.len() + db_cleaned as usize)
    }
    
    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| s.state == SessionState::Playing)
            .cloned()
            .collect()
    }
    
    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
    
    /// Queue a command for a disconnected session
    pub async fn queue_command(&self, session_id: Uuid, command: &str) -> Result<(), String> {
        self.store.queue_command(session_id, command).await
            .map_err(|e| format!("Failed to queue command: {}", e))
    }
    
    /// Get and clear queued commands for a session
    pub async fn get_and_clear_queued_commands(&self, session_id: Uuid) -> Result<Vec<String>, String> {
        let commands = self.store.get_queued_commands(session_id).await
            .map_err(|e| format!("Failed to get queued commands: {}", e))?;
        
        self.store.clear_queued_commands(session_id).await
            .map_err(|e| format!("Failed to clear queued commands: {}", e))?;
        
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::test_utils::*;
    
    async fn create_test_manager() -> SessionManager {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        let store = SessionStore::new(pool);
        SessionManager::new(store, 300)
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
        assert_eq!(session.id, session_id);
        assert_eq!(session.state, SessionState::Connecting);
        assert_eq!(session.protocol, ProtocolType::WebSocket);
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
    #[ignore] // Requires test database
    async fn test_update_session() {
        let manager = create_test_manager().await;
        
        let session_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");
        
        // Get and modify session
        let mut session = manager.get_session(session_id).await.unwrap();
        session.entity_id = Some(Uuid::new_v4());
        session.state = SessionState::Playing;
        
        // Update session
        manager.update_session(session.clone()).await.expect("Failed to update session");
        
        // Verify update
        let updated = manager.get_session(session_id).await.unwrap();
        assert_eq!(updated.entity_id, session.entity_id);
        assert_eq!(updated.state, SessionState::Playing);
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_remove_session() {
        let manager = create_test_manager().await;
        
        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");
        
        // Verify session exists
        assert!(manager.get_session(session_id).await.is_some());
        
        // Remove session
        manager.remove_session(session_id).await.expect("Failed to remove session");
        
        // Verify session is gone
        assert!(manager.get_session(session_id).await.is_none());
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
        manager.touch_session(session_id).await.expect("Failed to touch session");
        
        // Verify timestamp updated
        let touched_session = manager.get_session(session_id).await.unwrap();
        assert!(touched_session.last_activity > initial_time);
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_transition_session() {
        let manager = create_test_manager().await;
        
        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");
        
        // Valid transitions
        manager.transition_session(session_id, SessionState::Authenticating)
            .await
            .expect("Failed to transition to Authenticating");
        
        let session = manager.get_session(session_id).await.unwrap();
        assert_eq!(session.state, SessionState::Authenticating);
        
        manager.transition_session(session_id, SessionState::CharacterSelection)
            .await
            .expect("Failed to transition to CharacterSelection");
        
        manager.transition_session(session_id, SessionState::Playing)
            .await
            .expect("Failed to transition to Playing");
        
        let session = manager.get_session(session_id).await.unwrap();
        assert_eq!(session.state, SessionState::Playing);
        
        // Invalid transition
        let result = manager.transition_session(session_id, SessionState::Connecting).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
        manager.update_session(expired_session).await.expect("Failed to update expired session");
        
        // Cleanup
        let cleaned = manager.cleanup_expired().await.expect("Failed to cleanup");
        assert!(cleaned > 0);
        
        // Verify expired session is gone
        assert!(manager.get_session(expired_id).await.is_none());
        
        // Verify active session remains
        assert!(manager.get_session(active_id).await.is_some());
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_get_active_sessions() {
        let manager = create_test_manager().await;
        
        // Create sessions in different states
        let playing_id = manager
            .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
            .await
            .expect("Failed to create session");
        manager.transition_session(playing_id, SessionState::Authenticating).await.unwrap();
        manager.transition_session(playing_id, SessionState::CharacterSelection).await.unwrap();
        manager.transition_session(playing_id, SessionState::Playing).await.unwrap();
        
        let _connecting_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");
        
        // Get active sessions
        let active = manager.get_active_sessions().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, playing_id);
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
    #[ignore] // Requires test database
    async fn test_queue_and_get_commands() {
        let manager = create_test_manager().await;
        
        let session_id = manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session");
        
        // Queue commands
        manager.queue_command(session_id, "look").await.expect("Failed to queue command");
        manager.queue_command(session_id, "inventory").await.expect("Failed to queue command");
        manager.queue_command(session_id, "north").await.expect("Failed to queue command");
        
        // Get and clear commands
        let commands = manager.get_and_clear_queued_commands(session_id)
            .await
            .expect("Failed to get commands");
        
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "look");
        assert_eq!(commands[1], "inventory");
        assert_eq!(commands[2], "north");
        
        // Verify commands are cleared
        let commands = manager.get_and_clear_queued_commands(session_id)
            .await
            .expect("Failed to get commands");
        assert_eq!(commands.len(), 0);
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
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
                manager_clone.touch_session(session_id).await.expect("Failed to touch session");
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


