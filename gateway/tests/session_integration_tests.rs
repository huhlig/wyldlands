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

use wyldlands_gateway::session::{Session, SessionState, ProtocolType, SessionMetadata};
use wyldlands_gateway::session::store::SessionStore;
use wyldlands_gateway::session::manager::SessionManager;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
use std::sync::Arc;

async fn setup_test_db() -> sqlx::PgPool {
    dotenv::from_filename(".env.test").ok();
    
    let database_url = std::env::var("WYLDLANDS_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/wyldlands".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");
    
    // Drop and recreate tables
    sqlx::query("DROP TABLE IF EXISTS session_command_queue CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to drop session_command_queue table");
    
    sqlx::query("DROP TABLE IF EXISTS sessions CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to drop sessions table");
    
    sqlx::query(
        r#"
        CREATE TABLE sessions (
            id UUID PRIMARY KEY,
            entity_id UUID,
            created_at TIMESTAMPTZ NOT NULL,
            last_activity TIMESTAMPTZ NOT NULL,
            state TEXT NOT NULL CHECK (state IN ('Connecting', 'Authenticating', 'CharacterSelection', 'Playing', 'Disconnected', 'Closed')),
            protocol TEXT NOT NULL CHECK (protocol IN ('Telnet', 'WebSocket')),
            client_addr TEXT NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");
    
    sqlx::query("CREATE INDEX idx_sessions_entity_id ON sessions(entity_id)")
        .execute(&pool)
        .await
        .expect("Failed to create entity_id index");
    
    sqlx::query("CREATE INDEX idx_sessions_state ON sessions(state)")
        .execute(&pool)
        .await
        .expect("Failed to create state index");
    
    sqlx::query("CREATE INDEX idx_sessions_last_activity ON sessions(last_activity)")
        .execute(&pool)
        .await
        .expect("Failed to create last_activity index");
    
    sqlx::query(
        r#"
        CREATE TABLE session_command_queue (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            command TEXT NOT NULL,
            queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create session_command_queue table");
    
    sqlx::query("CREATE INDEX idx_session_command_queue_session_id ON session_command_queue(session_id)")
        .execute(&pool)
        .await
        .expect("Failed to create session_id index");
    
    pool
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_full_session_lifecycle() {
    let pool = setup_test_db().await;
    let store = SessionStore::new(pool.clone());
    let manager = SessionManager::new(store, 300);
    
    // Create session
    let session_id = manager
        .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
        .await
        .expect("Failed to create session");
    
    // Verify initial state
    let session = manager.get_session(session_id).await.unwrap();
    assert_eq!(session.state, SessionState::Connecting);
    
    // Transition through states
    manager.transition_session(session_id, SessionState::Authenticating)
        .await
        .expect("Failed to transition to Authenticating");
    
    manager.transition_session(session_id, SessionState::CharacterSelection)
        .await
        .expect("Failed to transition to CharacterSelection");
    
    manager.transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition to Playing");
    
    // Verify final state
    let session = manager.get_session(session_id).await.unwrap();
    assert_eq!(session.state, SessionState::Playing);
    
    // Simulate disconnect
    manager.transition_session(session_id, SessionState::Disconnected)
        .await
        .expect("Failed to transition to Disconnected");
    
    // Queue commands while disconnected
    manager.queue_command(session_id, "look").await.expect("Failed to queue command");
    manager.queue_command(session_id, "inventory").await.expect("Failed to queue command");
    
    // Reconnect
    manager.transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition back to Playing");
    
    // Get queued commands
    let commands = manager.get_and_clear_queued_commands(session_id)
        .await
        .expect("Failed to get commands");
    assert_eq!(commands.len(), 2);
    
    // Close session
    manager.transition_session(session_id, SessionState::Closed)
        .await
        .expect("Failed to transition to Closed");
    
    // Cleanup
    manager.remove_session(session_id).await.expect("Failed to remove session");
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_session_persistence_across_restarts() {
    let pool = setup_test_db().await;
    
    // Create first manager and session
    let store1 = SessionStore::new(pool.clone());
    let manager1 = SessionManager::new(store1, 300);
    
    let session_id = manager1
        .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
        .await
        .expect("Failed to create session");
    
    manager1.transition_session(session_id, SessionState::Authenticating)
        .await
        .expect("Failed to transition");
    
    // Simulate restart by creating new manager
    let store2 = SessionStore::new(pool.clone());
    
    // Load session from database
    let loaded_session = store2.load(session_id).await.expect("Failed to load session");
    assert!(loaded_session.is_some());
    
    let loaded_session = loaded_session.unwrap();
    assert_eq!(loaded_session.id, session_id);
    assert_eq!(loaded_session.state, SessionState::Authenticating);
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_concurrent_session_operations() {
    let pool = setup_test_db().await;
    let store = SessionStore::new(pool.clone());
    let manager = Arc::new(SessionManager::new(store, 300));
    
    // Create multiple sessions concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            manager_clone
                .create_session(
                    ProtocolType::WebSocket,
                    format!("127.0.0.1:{}", 8000 + i),
                )
                .await
                .expect("Failed to create session")
        });
        handles.push(handle);
    }
    
    // Collect session IDs
    let mut session_ids = vec![];
    for handle in handles {
        let session_id = handle.await.expect("Task panicked");
        session_ids.push(session_id);
    }
    
    assert_eq!(session_ids.len(), 10);
    assert_eq!(manager.session_count().await, 10);
    
    // Touch all sessions concurrently
    let mut handles = vec![];
    for session_id in &session_ids {
        let manager_clone = Arc::clone(&manager);
        let session_id = *session_id;
        let handle = tokio::spawn(async move {
            manager_clone.touch_session(session_id).await.expect("Failed to touch session");
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.expect("Task panicked");
    }
    
    // Verify all sessions still exist
    for session_id in &session_ids {
        let session = manager.get_session(*session_id).await;
        assert!(session.is_some());
    }
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_command_queue_ordering() {
    let pool = setup_test_db().await;
    let store = SessionStore::new(pool.clone());
    let manager = SessionManager::new(store, 300);
    
    let session_id = manager
        .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
        .await
        .expect("Failed to create session");
    
    // Queue commands in specific order
    let commands = vec!["north", "east", "south", "west", "look"];
    for cmd in &commands {
        manager.queue_command(session_id, cmd).await.expect("Failed to queue command");
    }
    
    // Retrieve and verify order
    let retrieved = manager.get_and_clear_queued_commands(session_id)
        .await
        .expect("Failed to get commands");
    
    assert_eq!(retrieved.len(), commands.len());
    for (i, cmd) in commands.iter().enumerate() {
        assert_eq!(retrieved[i], *cmd);
    }
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_expired_session_cleanup() {
    let pool = setup_test_db().await;
    let store = SessionStore::new(pool.clone());
    let manager = SessionManager::new(store, 300);
    
    // Create active session
    let active_id = manager
        .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
        .await
        .expect("Failed to create active session");
    
    // Create session that will be expired
    let expired_id = manager
        .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
        .await
        .expect("Failed to create expired session");
    
    // Make session expired by updating its timestamp
    let mut expired_session = manager.get_session(expired_id).await.unwrap();
    expired_session.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);
    manager.update_session(expired_session).await.expect("Failed to update session");
    
    // Run cleanup
    let cleaned = manager.cleanup_expired().await.expect("Failed to cleanup");
    assert!(cleaned > 0);
    
    // Verify expired session is removed
    assert!(manager.get_session(expired_id).await.is_none());
    
    // Verify active session remains
    assert!(manager.get_session(active_id).await.is_some());
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_session_metadata_persistence() {
    let pool = setup_test_db().await;
    let store = SessionStore::new(pool.clone());
    let manager = SessionManager::new(store, 300);
    
    let session_id = manager
        .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
        .await
        .expect("Failed to create session");
    
    // Update session with metadata
    let mut session = manager.get_session(session_id).await.unwrap();
    session.metadata = SessionMetadata {
        user_agent: Some("TestBrowser/1.0".to_string()),
        terminal_type: Some("xterm-256color".to_string()),
        window_size: Some((120, 40)),
        supports_color: true,
        supports_compression: true,
        custom: std::collections::HashMap::from([
            ("theme".to_string(), "dark".to_string()),
            ("language".to_string(), "en".to_string()),
        ]),
    };
    
    manager.update_session(session.clone()).await.expect("Failed to update session");
    
    // Reload and verify metadata
    let loaded = manager.get_session(session_id).await.unwrap();
    assert_eq!(loaded.metadata.user_agent, session.metadata.user_agent);
    assert_eq!(loaded.metadata.terminal_type, session.metadata.terminal_type);
    assert_eq!(loaded.metadata.window_size, session.metadata.window_size);
    assert_eq!(loaded.metadata.supports_color, session.metadata.supports_color);
    assert_eq!(loaded.metadata.supports_compression, session.metadata.supports_compression);
    assert_eq!(loaded.metadata.custom.get("theme"), Some(&"dark".to_string()));
    assert_eq!(loaded.metadata.custom.get("language"), Some(&"en".to_string()));
}

