//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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
use sqlx::PgPool;
use uuid::Uuid;

/// Session store for database persistence
pub struct SessionStore {
    pool: PgPool,
}

impl SessionStore {
    /// Create a new session store
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Save a session to the database
    pub async fn save(&self, session: &Session) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO wyldlands.sessions (id, entity_id, created_at, last_activity, state, protocol, client_addr, metadata)
            VALUES ($1, $2, $3, $4, $5::session_state, $6::session_protocol, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                entity_id = EXCLUDED.entity_id,
                last_activity = EXCLUDED.last_activity,
                state = EXCLUDED.state,
                metadata = EXCLUDED.metadata
            "#
        )
        .bind(session.id)
        .bind(session.entity_id)
        .bind(session.created_at)
        .bind(session.last_activity)
        .bind(format!("{:?}", session.state))
        .bind(format!("{:?}", session.protocol))
        .bind(&session.client_addr)
        .bind(serde_json::to_value(&session.metadata).unwrap())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Load a session from the database
    pub async fn load(&self, id: Uuid) -> Result<Option<Session>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, Option<Uuid>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, String, String, String, serde_json::Value)>(
            r#"
            SELECT id, entity_id, created_at, last_activity, state, protocol, client_addr, metadata
            FROM wyldlands.sessions
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|(id, entity_id, created_at, last_activity, state, protocol, client_addr, metadata)| Session {
            id,
            entity_id,
            created_at,
            last_activity,
            state: match state.as_str() {
                "Connecting" => SessionState::Connecting,
                "Authenticating" => SessionState::Authenticating,
                "CharacterSelection" => SessionState::CharacterSelection,
                "Playing" => SessionState::Playing,
                "Disconnected" => SessionState::Disconnected,
                "Closed" => SessionState::Closed,
                _ => SessionState::Closed,
            },
            protocol: match protocol.as_str() {
                "Telnet" => ProtocolType::Telnet,
                "WebSocket" => ProtocolType::WebSocket,
                _ => ProtocolType::WebSocket,
            },
            client_addr,
            metadata: serde_json::from_value(metadata).unwrap_or_default(),
        }))
    }
    
    /// Delete a session from the database
    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM wyldlands.sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self, timeout_seconds: i64) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(timeout_seconds);
        
        let result = sqlx::query(
            r#"
            DELETE FROM wyldlands.sessions
            WHERE last_activity < $1
            AND state IN ('Disconnected', 'Closed')
            "#
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Queue a command for a disconnected session
    pub async fn queue_command(&self, session_id: Uuid, command: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO wyldlands.session_command_queue (session_id, command) VALUES ($1, $2)")
            .bind(session_id)
            .bind(command)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// Get queued commands for a session
    pub async fn get_queued_commands(&self, session_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT command FROM session_command_queue
            WHERE session_id = $1
            ORDER BY queued_at ASC
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
    
    /// Clear queued commands for a session
    pub async fn clear_queued_commands(&self, session_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM session_command_queue WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::test_utils::*;
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_save_and_load_session() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let session = create_test_session(ProtocolType::WebSocket, SessionState::Playing);
        
        // Save session
        store.save(&session).await.expect("Failed to save session");
        
        // Load session
        let loaded = store.load(session.id).await.expect("Failed to load session");
        assert!(loaded.is_some());
        
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.state, session.state);
        assert_eq!(loaded.protocol, session.protocol);
        assert_eq!(loaded.client_addr, session.client_addr);
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_save_updates_existing_session() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let mut session = create_test_session(ProtocolType::Telnet, SessionState::Connecting);
        
        // Save initial session
        store.save(&session).await.expect("Failed to save session");
        
        // Update session state
        session.state = SessionState::Playing;
        session.entity_id = Some(Uuid::new_v4());
        
        // Save updated session
        store.save(&session).await.expect("Failed to update session");
        
        // Load and verify
        let loaded = store.load(session.id).await.expect("Failed to load session").unwrap();
        assert_eq!(loaded.state, SessionState::Playing);
        assert_eq!(loaded.entity_id, session.entity_id);
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_delete_session() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let session = create_test_session(ProtocolType::WebSocket, SessionState::Playing);
        
        // Save and verify
        store.save(&session).await.expect("Failed to save session");
        let loaded = store.load(session.id).await.expect("Failed to load session");
        assert!(loaded.is_some());
        
        // Delete
        store.delete(session.id).await.expect("Failed to delete session");
        
        // Verify deleted
        let loaded = store.load(session.id).await.expect("Failed to load session");
        assert!(loaded.is_none());
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_cleanup_expired_sessions() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        
        // Create expired disconnected session
        let mut expired_session = create_test_session(ProtocolType::Telnet, SessionState::Disconnected);
        expired_session.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);
        store.save(&expired_session).await.expect("Failed to save expired session");
        
        // Create active session
        let active_session = create_test_session(ProtocolType::WebSocket, SessionState::Playing);
        store.save(&active_session).await.expect("Failed to save active session");
        
        // Cleanup with 300 second timeout
        let cleaned = store.cleanup_expired(300).await.expect("Failed to cleanup");
        assert_eq!(cleaned, 1);
        
        // Verify expired session is gone
        let loaded = store.load(expired_session.id).await.expect("Failed to load");
        assert!(loaded.is_none());
        
        // Verify active session remains
        let loaded = store.load(active_session.id).await.expect("Failed to load");
        assert!(loaded.is_some());
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_queue_and_get_commands() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let session = create_test_session(ProtocolType::Telnet, SessionState::Disconnected);
        
        // Save session first
        store.save(&session).await.expect("Failed to save session");
        
        // Queue commands
        store.queue_command(session.id, "look").await.expect("Failed to queue command");
        store.queue_command(session.id, "inventory").await.expect("Failed to queue command");
        store.queue_command(session.id, "north").await.expect("Failed to queue command");
        
        // Get commands
        let commands = store.get_queued_commands(session.id).await.expect("Failed to get commands");
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "look");
        assert_eq!(commands[1], "inventory");
        assert_eq!(commands[2], "north");
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_clear_queued_commands() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let session = create_test_session(ProtocolType::WebSocket, SessionState::Disconnected);
        
        // Save session and queue commands
        store.save(&session).await.expect("Failed to save session");
        store.queue_command(session.id, "look").await.expect("Failed to queue command");
        store.queue_command(session.id, "inventory").await.expect("Failed to queue command");
        
        // Verify commands exist
        let commands = store.get_queued_commands(session.id).await.expect("Failed to get commands");
        assert_eq!(commands.len(), 2);
        
        // Clear commands
        store.clear_queued_commands(session.id).await.expect("Failed to clear commands");
        
        // Verify commands are cleared
        let commands = store.get_queued_commands(session.id).await.expect("Failed to get commands");
        assert_eq!(commands.len(), 0);
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_cascade_delete_commands() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let session = create_test_session(ProtocolType::Telnet, SessionState::Disconnected);
        
        // Save session and queue commands
        store.save(&session).await.expect("Failed to save session");
        store.queue_command(session.id, "look").await.expect("Failed to queue command");
        store.queue_command(session.id, "inventory").await.expect("Failed to queue command");
        
        // Delete session
        store.delete(session.id).await.expect("Failed to delete session");
        
        // Verify commands are also deleted (CASCADE)
        let commands = store.get_queued_commands(session.id).await.expect("Failed to get commands");
        assert_eq!(commands.len(), 0);
        
        cleanup_test_db(&pool).await;
    }
    
    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_save_with_metadata() {
        let pool = create_test_pool().await;
        setup_test_db(&pool).await;
        
        let store = SessionStore::new(pool.clone());
        let metadata = create_test_metadata();
        let session = create_test_session_with_metadata(
            ProtocolType::WebSocket,
            SessionState::Playing,
            metadata.clone(),
        );
        
        // Save session
        store.save(&session).await.expect("Failed to save session");
        
        // Load and verify metadata
        let loaded = store.load(session.id).await.expect("Failed to load session").unwrap();
        assert_eq!(loaded.metadata.user_agent, metadata.user_agent);
        assert_eq!(loaded.metadata.terminal_type, metadata.terminal_type);
        assert_eq!(loaded.metadata.window_size, metadata.window_size);
        assert_eq!(loaded.metadata.supports_color, metadata.supports_color);
        assert_eq!(loaded.metadata.supports_compression, metadata.supports_compression);
        
        cleanup_test_db(&pool).await;
    }
}


