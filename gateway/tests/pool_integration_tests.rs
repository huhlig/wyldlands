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

use wyldlands_gateway::pool::{ConnectionPool, PoolMessage};
use wyldlands_gateway::session::{ProtocolType, SessionState};
use wyldlands_gateway::session::manager::SessionManager;
use wyldlands_gateway::session::store::SessionStore;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Helper to create test database pool
async fn create_test_pool() -> sqlx::PgPool {
    dotenv::from_filename(".env.test").ok();
    
    let database_url = std::env::var("WYLDLANDS_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/wyldlands".to_string());
    
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Setup test database schema
async fn setup_test_db(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID PRIMARY KEY,
            entity_id UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            state VARCHAR(50) NOT NULL,
            protocol VARCHAR(20) NOT NULL,
            client_addr VARCHAR(100) NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create sessions table");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS session_command_queue (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            command TEXT NOT NULL,
            queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create command queue table");
}

/// Cleanup test database
async fn cleanup_test_db(pool: &sqlx::PgPool) {
    let _ = sqlx::query("DROP TABLE IF EXISTS session_command_queue CASCADE")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS sessions CASCADE")
        .execute(pool)
        .await;
}

/// Create test connection pool with session manager
async fn create_test_connection_pool() -> (Arc<ConnectionPool>, Arc<SessionManager>) {
    let db_pool = create_test_pool().await;
    setup_test_db(&db_pool).await;
    
    let store = SessionStore::new(db_pool);
    let manager = Arc::new(SessionManager::new(store, 300));
    let pool = Arc::new(ConnectionPool::new(Arc::clone(&manager)));
    
    (pool, manager)
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_lifecycle() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Create a session
    let session_id = manager
        .create_session(ProtocolType::WebSocket, "127.0.0.1:8080".to_string())
        .await
        .expect("Failed to create session");
    
    // Register connection
    let _sender = pool
        .register(session_id, ProtocolType::WebSocket)
        .await
        .expect("Failed to register connection");
    
    assert_eq!(pool.connection_count().await, 1);
    
    // Unregister connection
    pool.unregister(session_id)
        .await
        .expect("Failed to unregister connection");
    
    assert_eq!(pool.connection_count().await, 0);
    
    // Verify session state changed to Disconnected
    let session = manager.get_session(session_id).await.unwrap();
    assert_eq!(session.state, SessionState::Disconnected);
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_message_handling() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Spawn pool handler
    let pool_clone = Arc::clone(&pool);
    let handler = tokio::spawn(async move {
        pool_clone.run().await;
    });
    
    // Create and register session
    let session_id = manager
        .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
        .await
        .expect("Failed to create session");
    
    pool.register(session_id, ProtocolType::Telnet)
        .await
        .expect("Failed to register");
    
    // Send message via pool sender
    let sender = pool.sender();
    sender
        .send(PoolMessage::Send {
            session_id,
            data: b"Test message".to_vec(),
        })
        .expect("Failed to send message");
    
    // Give time for message processing
    sleep(Duration::from_millis(100)).await;
    
    // Get connection count via message
    let (tx, rx) = tokio::sync::oneshot::channel();
    sender
        .send(PoolMessage::GetCount { response: tx })
        .expect("Failed to send count request");
    
    let count = rx.await.expect("Failed to receive count");
    assert_eq!(count, 1);
    
    // Shutdown pool
    sender
        .send(PoolMessage::Shutdown)
        .expect("Failed to send shutdown");
    
    handler.await.expect("Handler task failed");
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_broadcast() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Create multiple sessions
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let session_id = manager
            .create_session(
                ProtocolType::WebSocket,
                format!("127.0.0.1:808{}", i),
            )
            .await
            .expect("Failed to create session");
        
        pool.register(session_id, ProtocolType::WebSocket)
            .await
            .expect("Failed to register");
        
        session_ids.push(session_id);
    }
    
    assert_eq!(pool.connection_count().await, 5);
    
    // Broadcast to all
    let data = b"Broadcast message".to_vec();
    let sent = pool.broadcast(data).await.expect("Failed to broadcast");
    assert_eq!(sent, 5);
    
    // Broadcast to specific sessions
    let target_sessions = &session_ids[0..3];
    let data = b"Targeted message".to_vec();
    let sent = pool
        .broadcast_to(target_sessions, data)
        .await
        .expect("Failed to broadcast to specific sessions");
    assert_eq!(sent, 3);
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_protocol_filtering() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Create WebSocket sessions
    for i in 0..3 {
        let session_id = manager
            .create_session(
                ProtocolType::WebSocket,
                format!("127.0.0.1:808{}", i),
            )
            .await
            .expect("Failed to create session");
        
        pool.register(session_id, ProtocolType::WebSocket)
            .await
            .expect("Failed to register");
    }
    
    // Create Telnet sessions
    for i in 0..2 {
        let session_id = manager
            .create_session(
                ProtocolType::Telnet,
                format!("127.0.0.1:2{}", i),
            )
            .await
            .expect("Failed to create session");
        
        pool.register(session_id, ProtocolType::Telnet)
            .await
            .expect("Failed to register");
    }
    
    let ws_connections = pool.connections_by_protocol(ProtocolType::WebSocket).await;
    let telnet_connections = pool.connections_by_protocol(ProtocolType::Telnet).await;
    
    assert_eq!(ws_connections.len(), 3);
    assert_eq!(telnet_connections.len(), 2);
    assert_eq!(pool.connection_count().await, 5);
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_cleanup_disconnected() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Create sessions and transition to Playing
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let session_id = manager
            .create_session(
                ProtocolType::WebSocket,
                format!("127.0.0.1:808{}", i),
            )
            .await
            .expect("Failed to create session");
        
        // Transition to Playing state
        manager
            .transition_session(session_id, SessionState::Authenticating)
            .await
            .expect("Failed to transition");
        manager
            .transition_session(session_id, SessionState::CharacterSelection)
            .await
            .expect("Failed to transition");
        manager
            .transition_session(session_id, SessionState::Playing)
            .await
            .expect("Failed to transition");
        
        session_ids.push(session_id);
    }
    
    // Register only 3 connections (2 are disconnected)
    for i in 0..3 {
        pool.register(session_ids[i], ProtocolType::WebSocket)
            .await
            .expect("Failed to register");
    }
    
    // Cleanup disconnected sessions
    let cleaned = pool
        .cleanup_disconnected()
        .await
        .expect("Failed to cleanup");
    
    assert_eq!(cleaned, 2); // 2 sessions should be marked as disconnected
    
    // Verify the disconnected sessions have correct state
    for i in 3..5 {
        let session = manager.get_session(session_ids[i]).await.unwrap();
        assert_eq!(session.state, SessionState::Disconnected);
    }
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_concurrent_operations() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Spawn multiple tasks that create and register sessions concurrently
    let mut handles = Vec::new();
    
    for i in 0..20 {
        let pool = Arc::clone(&pool);
        let manager = Arc::clone(&manager);
        
        let handle = tokio::spawn(async move {
            let session_id = manager
                .create_session(
                    ProtocolType::WebSocket,
                    format!("127.0.0.1:{}", 8000 + i),
                )
                .await
                .expect("Failed to create session");
            
            pool.register(session_id, ProtocolType::WebSocket)
                .await
                .expect("Failed to register");
            
            session_id
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let mut session_ids = Vec::new();
    for handle in handles {
        let session_id = handle.await.expect("Task failed");
        session_ids.push(session_id);
    }
    
    // Verify all sessions are registered
    assert_eq!(pool.connection_count().await, 20);
    
    // Concurrently send messages to all sessions
    let mut send_handles = Vec::new();
    for session_id in session_ids {
        let pool = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            pool.send(session_id, b"Test".to_vec())
                .await
                .expect("Failed to send");
        });
        send_handles.push(handle);
    }
    
    for handle in send_handles {
        handle.await.expect("Send task failed");
    }
}

#[tokio::test]
#[ignore] // Requires test database
async fn test_pool_stress_test() {
    let (pool, manager) = create_test_connection_pool().await;
    
    // Create 100 sessions rapidly
    let start = std::time::Instant::now();
    
    for i in 0..100 {
        let session_id = manager
            .create_session(
                ProtocolType::WebSocket,
                format!("127.0.0.1:{}", 8000 + i),
            )
            .await
            .expect("Failed to create session");
        
        pool.register(session_id, ProtocolType::WebSocket)
            .await
            .expect("Failed to register");
    }
    
    let duration = start.elapsed();
    println!("Created and registered 100 sessions in {:?}", duration);
    
    assert_eq!(pool.connection_count().await, 100);
    
    // Broadcast to all
    let broadcast_start = std::time::Instant::now();
    let sent = pool
        .broadcast(b"Stress test message".to_vec())
        .await
        .expect("Failed to broadcast");
    let broadcast_duration = broadcast_start.elapsed();
    
    println!("Broadcast to {} connections in {:?}", sent, broadcast_duration);
    assert_eq!(sent, 100);
}

